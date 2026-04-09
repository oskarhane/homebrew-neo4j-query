use clap::Parser;
use serde_json::{Map, Value};
use std::io::{self, IsTerminal, Read};

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
    message: String,
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let cypher = resolve_query(cli.query)?;
    let params = parse_params(&cli.p)?;

    let url = format!("{}/db/{}/query/v2", cli.uri.trim_end_matches('/'), cli.database);

    let mut body = Map::new();
    body.insert("statement".into(), Value::String(cypher));
    if !params.is_empty() {
        body.insert("parameters".into(), Value::Object(params));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .basic_auth(&cli.user, Some(&cli.password))
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {text}").into());
    }

    let parsed: QueryResponse = resp.json().await?;

    if let Some(errors) = parsed.errors {
        if !errors.is_empty() {
            let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
            return Err(msgs.join("; ").into());
        }
    }

    let data = parsed.data.ok_or("no data in response")?;
    let records = rows_to_records(&data.fields, &data.values)?;

    let toon = toon_format::encode_default(&records)?;
    print!("{toon}");

    Ok(())
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
