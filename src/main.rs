use clap::Parser;
use serde_json::{Map, Value};
use std::io::{self, IsTerminal, Read};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 200;

#[derive(Parser)]
#[command(name = "neo4j-query", about = "Query Neo4j databases, output TOON")]
struct Cli {
    /// Cypher query to execute
    query: Option<String>,

    /// Neo4j HTTP URI
    #[arg(long, env = "NEO4J_URI", default_value = "http://localhost:7474")]
    uri: String,

    /// Neo4j username
    #[arg(long, env = "NEO4J_USER", default_value = "neo4j")]
    user: String,

    /// Neo4j password
    #[arg(long, env = "NEO4J_PASSWORD")]
    password: String,

    /// Neo4j database name
    #[arg(long, env = "NEO4J_DATABASE", default_value = "neo4j")]
    database: String,

    /// Query parameters as key=value pairs
    #[arg(short, value_name = "KEY=VALUE")]
    p: Vec<String>,
}

fn resolve_query(arg: Option<String>) -> Result<String, String> {
    if let Some(q) = arg {
        return Ok(q);
    }
    if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            return Err("empty query from stdin".into());
        }
        return Ok(trimmed);
    }
    Err("no query provided. Usage: neo4j-query \"CYPHER QUERY\" or echo \"QUERY\" | neo4j-query".into())
}

fn parse_param_value(v: &str) -> Value {
    if v == "true" {
        return Value::Bool(true);
    }
    if v == "false" {
        return Value::Bool(false);
    }
    if v == "null" {
        return Value::Null;
    }
    if let Ok(n) = v.parse::<i64>() {
        return Value::Number(n.into());
    }
    if let Ok(n) = v.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return Value::Number(num);
        }
    }
    Value::String(v.to_string())
}

fn parse_params(pairs: &[String]) -> Result<Map<String, Value>, String> {
    let mut map = Map::new();
    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| format!("invalid param format '{pair}', expected key=value"))?;
        map.insert(k.to_string(), parse_param_value(v));
    }
    Ok(map)
}

fn rows_to_records(fields: &[Value], values: &[Value]) -> Result<Vec<Value>, String> {
    let field_names: Vec<&str> = fields
        .iter()
        .map(|f| f.as_str().ok_or_else(|| "field name is not a string".to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    values
        .iter()
        .map(|row| {
            let cells = row
                .as_array()
                .ok_or_else(|| "row is not an array".to_string())?;
            let mut record = Map::new();
            for (name, val) in field_names.iter().zip(cells.iter()) {
                record.insert(name.to_string(), val.clone());
            }
            Ok(Value::Object(record))
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct QueryResponse {
    data: Option<ResponseData>,
    errors: Option<Vec<ResponseError>>,
}

#[derive(serde::Deserialize)]
struct ResponseData {
    fields: Vec<Value>,
    values: Vec<Value>,
}

#[derive(serde::Deserialize)]
struct ResponseError {
    code: Option<String>,
    message: String,
}

fn has_transient_error(errors: &[ResponseError]) -> bool {
    errors
        .iter()
        .any(|e| e.code.as_deref().unwrap_or("").starts_with("Neo.TransientError."))
}

fn format_errors(errors: &[ResponseError]) -> String {
    errors
        .iter()
        .map(|e| match &e.code {
            Some(code) => format!("[{code}] {}", e.message),
            None => e.message.clone(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

async fn execute_query(
    client: &reqwest::Client,
    url: &str,
    user: &str,
    password: &str,
    body: &Value,
) -> Result<QueryResponse, Box<dyn std::error::Error>> {
    let resp = client
        .post(url)
        .basic_auth(user, Some(password))
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<QueryResponse>(&text) {
            if let Some(errors) = &parsed.errors {
                if !errors.is_empty() {
                    return Err(format!("HTTP {status}: {}", format_errors(errors)).into());
                }
            }
        }
        return Err(format!("HTTP {status}: {text}").into());
    }

    Ok(resp.json().await?)
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let cypher = resolve_query(cli.query)?;
    let params = parse_params(&cli.p)?;

    let url = format!(
        "{}/db/{}/query/v2",
        cli.uri.trim_end_matches('/'),
        cli.database
    );

    let mut body = Map::new();
    body.insert("statement".into(), Value::String(cypher));
    if !params.is_empty() {
        body.insert("parameters".into(), Value::Object(params));
    }
    let body = Value::Object(body);

    let client = reqwest::Client::new();

    let mut last_err = None;
    for attempt in 0..=MAX_RETRIES {
        match execute_query(&client, &url, &cli.user, &cli.password, &body).await {
            Ok(parsed) => {
                if let Some(errors) = &parsed.errors {
                    if !errors.is_empty() {
                        if has_transient_error(errors) && attempt < MAX_RETRIES {
                            let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
                            eprintln!(
                                "transient error, retrying in {backoff}ms (attempt {}/{})",
                                attempt + 1,
                                MAX_RETRIES
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                            last_err = Some(format_errors(errors));
                            continue;
                        }
                        return Err(format_errors(errors).into());
                    }
                }

                let data = parsed.data.ok_or("no data in response")?;
                let records = rows_to_records(&data.fields, &data.values)?;
                let toon = toon_format::encode_default(&records)?;
                print!("{toon}");
                return Ok(());
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    // Retry on network errors too (connection reset, timeout)
                    let is_network_err = e.to_string().contains("connection")
                        || e.to_string().contains("timeout")
                        || e.to_string().contains("reset");
                    if is_network_err {
                        let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
                        eprintln!(
                            "network error, retrying in {backoff}ms (attempt {}/{})",
                            attempt + 1,
                            MAX_RETRIES
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                        last_err = Some(e.to_string());
                        continue;
                    }
                }
                return Err(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| "max retries exceeded".into()).into())
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    if let Err(e) = rt.block_on(run()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
