use clap::{Parser, ValueEnum};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Toon,
    Json,
}

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 200;

#[derive(Parser)]
#[command(name = "neo4j-query", about = "Query Neo4j databases, output TOON")]
struct Cli {
    /// Cypher query to execute (or .schema for schema introspection)
    query: Option<String>,

    /// Neo4j HTTP URI
    #[arg(long, env = "NEO4J_URI", default_value = "http://localhost:7474")]
    uri: String,

    /// Neo4j username
    #[arg(short, long, env = "NEO4J_USERNAME", default_value = "neo4j")]
    username: String,

    /// Neo4j password
    #[arg(short, long, env = "NEO4J_PASSWORD")]
    password: String,

    /// Neo4j database name
    #[arg(long, env = "NEO4J_DATABASE", default_value = "neo4j")]
    database: String,

    /// Query parameters as key=value pairs
    #[arg(short = 'P', value_name = "KEY=VALUE")]
    p: Vec<String>,

    /// Path to .env file to load
    #[arg(long = "env", value_name = "FILE")]
    env_file: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value = "toon")]
    output: OutputFormat,
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
    Err(
        "no query provided. Usage: neo4j-query \"CYPHER QUERY\" or echo \"QUERY\" | neo4j-query"
            .into(),
    )
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
        .map(|f| {
            f.as_str()
                .ok_or_else(|| "field name is not a string".to_string())
        })
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
    errors.iter().any(|e| {
        e.code
            .as_deref()
            .unwrap_or("")
            .starts_with("Neo.TransientError.")
    })
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

async fn run_cypher(
    client: &reqwest::Client,
    url: &str,
    user: &str,
    password: &str,
    cypher: &str,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let body = json!({"statement": cypher});
    let parsed = execute_query(client, url, user, password, &body).await?;
    if let Some(errors) = &parsed.errors {
        if !errors.is_empty() {
            return Err(format_errors(errors).into());
        }
    }
    let data = parsed.data.ok_or("no data in response")?;
    rows_to_records(&data.fields, &data.values).map_err(|e| e.into())
}

async fn run_schema(
    client: &reqwest::Client,
    url: &str,
    user: &str,
    password: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    // 1. Get node labels and properties
    let node_rows = run_cypher(
        client,
        url,
        user,
        password,
        "CALL db.schema.nodeTypeProperties() \
         YIELD nodeType, nodeLabels, propertyName, propertyTypes, mandatory \
         RETURN nodeType, nodeLabels, propertyName, propertyTypes, mandatory",
    )
    .await?;

    // 2. Get relationship types and properties
    let rel_rows = run_cypher(
        client,
        url,
        user,
        password,
        "CALL db.schema.relTypeProperties() \
         YIELD relType, propertyName, propertyTypes, mandatory \
         RETURN relType, propertyName, propertyTypes, mandatory",
    )
    .await?;

    // Build node map: label -> { properties: [...] }
    let mut nodes: HashMap<String, Vec<Value>> = HashMap::new();
    for row in &node_rows {
        let labels = row.get("nodeLabels").and_then(|v| v.as_array());
        let prop_name = row.get("propertyName").and_then(|v| v.as_str());
        let prop_types = row.get("propertyTypes").and_then(|v| v.as_array());
        let mandatory = row
            .get("mandatory")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let (Some(labels), Some(prop_name), Some(prop_types)) = (labels, prop_name, prop_types) {
            // Use sorted joined labels as key
            let mut label_strs: Vec<&str> = labels.iter().filter_map(|l| l.as_str()).collect();
            label_strs.sort();
            let key = label_strs.join(":");

            if !prop_name.is_empty() {
                let type_strs: Vec<&str> = prop_types.iter().filter_map(|t| t.as_str()).collect();
                let prop = json!({
                    "name": prop_name,
                    "types": type_strs,
                    "mandatory": mandatory,
                });
                nodes.entry(key).or_default().push(prop);
            } else {
                nodes.entry(key).or_default();
            }
        }
    }

    // Build rel type -> properties map
    let mut rel_props: HashMap<String, Vec<Value>> = HashMap::new();
    let mut rel_types: Vec<String> = Vec::new();
    for row in &rel_rows {
        let rel_type = row.get("relType").and_then(|v| v.as_str()).unwrap_or("");
        let clean_type = rel_type
            .trim_start_matches(":`")
            .trim_end_matches('`')
            .to_string();
        let prop_name = row.get("propertyName").and_then(|v| v.as_str());
        let prop_types = row.get("propertyTypes").and_then(|v| v.as_array());
        let mandatory = row
            .get("mandatory")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !rel_types.contains(&clean_type) {
            rel_types.push(clean_type.clone());
        }

        if let (Some(prop_name), Some(prop_types)) = (prop_name, prop_types) {
            if !prop_name.is_empty() {
                let type_strs: Vec<&str> = prop_types.iter().filter_map(|t| t.as_str()).collect();
                let prop = json!({
                    "name": prop_name,
                    "types": type_strs,
                    "mandatory": mandatory,
                });
                rel_props.entry(clean_type).or_default().push(prop);
            } else {
                rel_props.entry(clean_type).or_default();
            }
        }
    }

    // 3. For each relationship type, find which node labels it connects
    let mut relationships: Vec<Value> = Vec::new();
    for rel_type in &rel_types {
        let cypher = format!(
            "MATCH (n)-[r:`{}`]->(m) \
             WITH DISTINCT labels(n) AS from, labels(m) AS to \
             RETURN from, to",
            rel_type.replace('`', "``")
        );
        let path_rows = run_cypher(client, url, user, password, &cypher).await?;

        let mut paths: Vec<Value> = Vec::new();
        for path_row in &path_rows {
            let from = path_row.get("from").and_then(|v| v.as_array());
            let to = path_row.get("to").and_then(|v| v.as_array());
            if let (Some(from), Some(to)) = (from, to) {
                let mut from_strs: Vec<&str> = from.iter().filter_map(|l| l.as_str()).collect();
                let mut to_strs: Vec<&str> = to.iter().filter_map(|l| l.as_str()).collect();
                from_strs.sort();
                to_strs.sort();
                paths.push(json!({
                    "from": from_strs,
                    "to": to_strs,
                }));
            }
        }

        let props = rel_props
            .get(rel_type.as_str())
            .cloned()
            .unwrap_or_default();
        relationships.push(json!({
            "type": rel_type,
            "properties": props,
            "paths": paths,
        }));
    }

    // Build final schema object
    let mut node_list: Vec<Value> = Vec::new();
    let mut sorted_keys: Vec<&String> = nodes.keys().collect();
    sorted_keys.sort();
    for key in sorted_keys {
        let props = nodes.get(key.as_str()).unwrap();
        let labels: Vec<&str> = key.split(':').collect();
        node_list.push(json!({
            "labels": labels,
            "properties": props,
        }));
    }

    Ok(json!({
        "nodes": node_list,
        "relationships": relationships,
    }))
}

fn load_env() -> Result<(), Box<dyn std::error::Error>> {
    // Pre-scan args for --env before clap parses, so the file is loaded
    // before clap resolves env-backed defaults.
    let args: Vec<String> = std::env::args().collect();
    let mut env_file: Option<PathBuf> = None;
    for i in 0..args.len() {
        if args[i] == "--env" {
            if let Some(path) = args.get(i + 1) {
                env_file = Some(PathBuf::from(path));
            }
        } else if let Some(path) = args[i].strip_prefix("--env=") {
            env_file = Some(PathBuf::from(path));
        }
    }

    if let Some(path) = env_file {
        dotenvy::from_path(&path)
            .map_err(|e| format!("failed to load env file '{}': {e}", path.display()))?;
    } else {
        dotenvy::dotenv().ok();
    }
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    load_env()?;
    let cli = Cli::parse();
    let input = resolve_query(cli.query)?;

    let url = format!(
        "{}/db/{}/query/v2",
        cli.uri.trim_end_matches('/'),
        cli.database
    );

    let client = reqwest::Client::new();

    // Handle built-in commands
    if input.trim() == ".schema" {
        let schema = run_schema(&client, &url, &cli.username, &cli.password).await?;
        let toon = toon_format::encode_default(&schema)?;
        println!("{toon}");
        return Ok(());
    }

    let params = parse_params(&cli.p)?;

    let mut body = Map::new();
    body.insert("statement".into(), Value::String(input));
    if !params.is_empty() {
        body.insert("parameters".into(), Value::Object(params));
    }
    let body = Value::Object(body);

    let mut last_err = None;
    for attempt in 0..=MAX_RETRIES {
        match execute_query(&client, &url, &cli.username, &cli.password, &body).await {
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
                let output = match cli.output {
                    OutputFormat::Json => serde_json::to_string(&records)?,
                    OutputFormat::Toon => toon_format::encode_default(&records)?,
                };
                println!("{output}");
                return Ok(());
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
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

    Err(last_err
        .unwrap_or_else(|| "max retries exceeded".into())
        .into())
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
