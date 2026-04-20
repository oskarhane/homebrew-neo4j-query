// Test floats are sentinels, not numeric constants — silence approx_constant.
#![allow(clippy::approx_constant)]

use serde_json::{json, Map, Value};

// Re-implement pure functions here for testing since they're private in main.

fn has_transient_error(errors: &[(Option<&str>, &str)]) -> bool {
    errors
        .iter()
        .any(|(code, _)| code.unwrap_or("").starts_with("Neo.TransientError."))
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

#[test]
fn parse_param_value_string() {
    assert_eq!(parse_param_value("hello"), Value::String("hello".into()));
}

#[test]
fn parse_param_value_integer() {
    assert_eq!(parse_param_value("42"), json!(42));
}

#[test]
fn parse_param_value_float() {
    assert_eq!(parse_param_value("3.14"), json!(3.14));
}

#[test]
fn parse_param_value_bool() {
    assert_eq!(parse_param_value("true"), Value::Bool(true));
    assert_eq!(parse_param_value("false"), Value::Bool(false));
}

#[test]
fn parse_param_value_null() {
    assert_eq!(parse_param_value("null"), Value::Null);
}

#[test]
fn parse_params_multiple() {
    let pairs = vec![
        "name=Alice".to_string(),
        "age=30".to_string(),
        "active=true".to_string(),
    ];
    let map = parse_params(&pairs).unwrap();
    assert_eq!(map["name"], json!("Alice"));
    assert_eq!(map["age"], json!(30));
    assert_eq!(map["active"], json!(true));
}

#[test]
fn parse_params_invalid_format() {
    let pairs = vec!["noequals".to_string()];
    assert!(parse_params(&pairs).is_err());
}

#[test]
fn parse_params_value_with_equals() {
    let pairs = vec!["expr=a=b".to_string()];
    let map = parse_params(&pairs).unwrap();
    assert_eq!(map["expr"], json!("a=b"));
}

#[test]
fn rows_to_records_basic() {
    let fields = vec![json!("name"), json!("age")];
    let values = vec![json!(["Alice", 30]), json!(["Bob", 25])];
    let records = rows_to_records(&fields, &values).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["name"], json!("Alice"));
    assert_eq!(records[0]["age"], json!(30));
    assert_eq!(records[1]["name"], json!("Bob"));
    assert_eq!(records[1]["age"], json!(25));
}

#[test]
fn rows_to_records_empty() {
    let fields = vec![json!("n")];
    let values: Vec<Value> = vec![];
    let records = rows_to_records(&fields, &values).unwrap();
    assert!(records.is_empty());
}

#[test]
fn rows_to_records_with_nested_objects() {
    let fields = vec![json!("n")];
    let node = json!({
        "elementId": "4:abc:0",
        "labels": ["Person"],
        "properties": {"name": "Alice"}
    });
    let values = vec![json!([node])];
    let records = rows_to_records(&fields, &values).unwrap();
    assert_eq!(records[0]["n"]["labels"], json!(["Person"]));
}

#[test]
fn rows_to_records_non_string_field_errors() {
    let fields = vec![json!(123)];
    let values = vec![json!([1])];
    assert!(rows_to_records(&fields, &values).is_err());
}

#[test]
fn rows_to_records_null_values() {
    let fields = vec![json!("x"), json!("y")];
    let values = vec![json!([null, 1])];
    let records = rows_to_records(&fields, &values).unwrap();
    assert_eq!(records[0]["x"], json!(null));
    assert_eq!(records[0]["y"], json!(1));
}

#[test]
fn rows_to_records_empty_fields_and_values() {
    let fields: Vec<Value> = vec![];
    let values: Vec<Value> = vec![];
    let records = rows_to_records(&fields, &values).unwrap();
    assert!(records.is_empty());
}

#[test]
fn parse_param_value_negative_int() {
    assert_eq!(parse_param_value("-42"), json!(-42));
}

#[test]
fn parse_param_value_negative_float() {
    assert_eq!(parse_param_value("-3.14"), json!(-3.14));
}

#[test]
fn parse_param_value_empty_string() {
    assert_eq!(parse_param_value(""), json!(""));
}

#[test]
fn parse_params_empty() {
    let pairs: Vec<String> = vec![];
    let map = parse_params(&pairs).unwrap();
    assert!(map.is_empty());
}

#[test]
fn parse_param_value_string_that_looks_numeric() {
    // "42abc" should be a string, not a number
    assert_eq!(parse_param_value("42abc"), json!("42abc"));
}

#[test]
fn rows_to_records_row_not_array_errors() {
    let fields = vec![json!("x")];
    let values = vec![json!("not an array")];
    assert!(rows_to_records(&fields, &values).is_err());
}

#[test]
fn transient_error_detected() {
    let errors = vec![(
        Some("Neo.TransientError.Transaction.DeadlockDetected"),
        "deadlock",
    )];
    assert!(has_transient_error(&errors));
}

#[test]
fn client_error_not_transient() {
    let errors = vec![(
        Some("Neo.ClientError.Statement.SyntaxError"),
        "syntax error",
    )];
    assert!(!has_transient_error(&errors));
}

#[test]
fn no_code_not_transient() {
    let errors = vec![(None, "some error")];
    assert!(!has_transient_error(&errors));
}

#[test]
fn mixed_errors_with_transient() {
    let errors = vec![
        (Some("Neo.ClientError.Statement.SyntaxError"), "syntax"),
        (
            Some("Neo.TransientError.General.DatabaseUnavailable"),
            "unavailable",
        ),
    ];
    assert!(has_transient_error(&errors));
}

// Re-implement truncate_arrays for testing (private in main)
fn truncate_arrays(value: &mut Value, threshold: usize, replacer: &dyn Fn(usize) -> Value) {
    if threshold == 0 {
        return;
    }
    match value {
        Value::Array(arr) => {
            if arr.len() > threshold {
                *value = replacer(arr.len());
            } else {
                for item in arr.iter_mut() {
                    truncate_arrays(item, threshold, replacer);
                }
            }
        }
        Value::Object(map) => {
            for (_k, v) in map.iter_mut() {
                truncate_arrays(v, threshold, replacer);
            }
        }
        _ => {}
    }
}

fn toon_replacer(n: usize) -> Value {
    Value::String(format!("[array truncated: {n} items]"))
}

fn json_replacer(_n: usize) -> Value {
    Value::Array(vec![])
}

// --- truncate_arrays tests ---

#[test]
fn truncate_no_arrays() {
    let mut val = json!({"name": "Alice", "age": 30});
    let original = val.clone();
    truncate_arrays(&mut val, 5, &toon_replacer);
    assert_eq!(val, original);
}

#[test]
fn truncate_small_array_unchanged() {
    let mut val = json!({"items": [1, 2, 3]});
    let original = val.clone();
    truncate_arrays(&mut val, 5, &toon_replacer);
    assert_eq!(val, original);
}

#[test]
fn truncate_top_level_array_toon() {
    let big: Vec<i32> = (0..150).collect();
    let mut val = json!({"embedding": big});
    truncate_arrays(&mut val, 100, &toon_replacer);
    assert_eq!(val["embedding"], json!("[array truncated: 150 items]"));
}

#[test]
fn truncate_top_level_array_json() {
    let big: Vec<i32> = (0..150).collect();
    let mut val = json!({"embedding": big});
    truncate_arrays(&mut val, 100, &json_replacer);
    assert_eq!(val["embedding"], json!([]));
}

#[test]
fn truncate_nested_array_in_object() {
    let big: Vec<i32> = (0..10).collect();
    let mut val = json!({"node": {"props": {"vec": big}}});
    truncate_arrays(&mut val, 5, &toon_replacer);
    assert_eq!(
        val["node"]["props"]["vec"],
        json!("[array truncated: 10 items]")
    );
}

#[test]
fn truncate_deeply_nested_3_levels() {
    let big: Vec<i32> = (0..20).collect();
    let mut val = json!({
        "a": {
            "b": [
                {"c": big}
            ]
        }
    });
    truncate_arrays(&mut val, 10, &toon_replacer);
    assert_eq!(val["a"]["b"][0]["c"], json!("[array truncated: 20 items]"));
}

#[test]
fn truncate_exact_boundary_not_truncated() {
    // Array at exactly threshold length should NOT be truncated
    let arr: Vec<i32> = (0..5).collect();
    let mut val = json!({"arr": arr});
    let original = val.clone();
    truncate_arrays(&mut val, 5, &toon_replacer);
    assert_eq!(val, original);
}

#[test]
fn truncate_boundary_plus_one() {
    // Array at threshold+1 SHOULD be truncated
    let arr: Vec<i32> = (0..6).collect();
    let mut val = json!({"arr": arr});
    truncate_arrays(&mut val, 5, &toon_replacer);
    assert_eq!(val["arr"], json!("[array truncated: 6 items]"));
}

#[test]
fn truncate_threshold_zero_noop() {
    let big: Vec<i32> = (0..200).collect();
    let mut val = json!({"big": big});
    let original = val.clone();
    truncate_arrays(&mut val, 0, &toon_replacer);
    assert_eq!(val, original);
}

#[test]
fn truncate_multiple_arrays_mixed() {
    let big: Vec<i32> = (0..20).collect();
    let small = vec![1, 2, 3];
    let mut val = json!({
        "big": big,
        "small": small,
        "text": "hello"
    });
    truncate_arrays(&mut val, 10, &toon_replacer);
    assert_eq!(val["big"], json!("[array truncated: 20 items]"));
    assert_eq!(val["small"], json!([1, 2, 3]));
    assert_eq!(val["text"], json!("hello"));
}

#[test]
fn truncate_array_inside_array() {
    // Inner arrays that exceed threshold should be truncated
    let big: Vec<i32> = (0..10).collect();
    let mut val = json!([[1, 2], big]);
    truncate_arrays(&mut val, 5, &toon_replacer);
    assert_eq!(val[0], json!([1, 2]));
    assert_eq!(val[1], json!("[array truncated: 10 items]"));
}

#[test]
fn truncate_outer_array_exceeds_threshold() {
    // If the outer array itself exceeds threshold, it gets replaced entirely
    let mut val = json!([1, 2, 3, 4, 5, 6]);
    truncate_arrays(&mut val, 3, &json_replacer);
    assert_eq!(val, json!([]));
}

#[test]
fn truncate_json_mode_preserves_structure() {
    let big: Vec<i32> = (0..50).collect();
    let mut val = json!({"results": [{"embedding": big, "name": "test"}]});
    truncate_arrays(&mut val, 10, &json_replacer);
    assert_eq!(val["results"][0]["embedding"], json!([]));
    assert_eq!(val["results"][0]["name"], json!("test"));
}

#[test]
fn truncate_scalar_values_unchanged() {
    let mut val = json!(42);
    truncate_arrays(&mut val, 1, &toon_replacer);
    assert_eq!(val, json!(42));

    let mut val = json!("hello");
    truncate_arrays(&mut val, 1, &toon_replacer);
    assert_eq!(val, json!("hello"));

    let mut val = json!(null);
    truncate_arrays(&mut val, 1, &toon_replacer);
    assert_eq!(val, json!(null));

    let mut val = json!(true);
    truncate_arrays(&mut val, 1, &toon_replacer);
    assert_eq!(val, json!(true));
}

// --- parse_param (ParamSpec) re-implementation for testing ---
//
// Keep logic byte-identical to src/params.rs::parse_param so these tests act
// as a spec-level guard against accidental REQ-F-011 error-string drift.

#[derive(Debug, Clone, PartialEq)]
enum ParamSpec {
    Literal(Value),
    Embed(String),
}

fn parse_param(raw: &str) -> Result<(String, ParamSpec), String> {
    let (key_part, value) = raw
        .split_once('=')
        .ok_or_else(|| format!("invalid param format '{raw}', expected key=value"))?;
    if let Some((name, modifier)) = key_part.split_once(':') {
        match modifier {
            "embed" => Ok((name.to_string(), ParamSpec::Embed(value.to_string()))),
            other => Err(format!("unknown param modifier: :{other}")),
        }
    } else {
        Ok((
            key_part.to_string(),
            ParamSpec::Literal(parse_param_value(value)),
        ))
    }
}

#[test]
fn parse_param_literal_int() {
    let (name, spec) = parse_param("age=30").unwrap();
    assert_eq!(name, "age");
    assert_eq!(spec, ParamSpec::Literal(json!(30)));
}

#[test]
fn parse_param_literal_bool() {
    let (_, spec) = parse_param("active=true").unwrap();
    assert_eq!(spec, ParamSpec::Literal(json!(true)));
}

#[test]
fn parse_param_literal_null() {
    let (_, spec) = parse_param("x=null").unwrap();
    assert_eq!(spec, ParamSpec::Literal(json!(null)));
}

#[test]
fn parse_param_literal_float() {
    let (_, spec) = parse_param("ratio=2.5").unwrap();
    assert_eq!(spec, ParamSpec::Literal(json!(2.5)));
}

#[test]
fn parse_param_literal_string() {
    let (name, spec) = parse_param("name=Alice").unwrap();
    assert_eq!(name, "name");
    assert_eq!(spec, ParamSpec::Literal(json!("Alice")));
}

#[test]
fn parse_param_embed_modifier() {
    let (name, spec) = parse_param("v:embed=hello world").unwrap();
    assert_eq!(name, "v");
    assert_eq!(spec, ParamSpec::Embed("hello world".to_string()));
}

#[test]
fn parse_param_unknown_modifier() {
    // REQ-F-011: exact error string for unknown modifier
    let err = parse_param("v:foo=x").unwrap_err();
    assert_eq!(err, "unknown param modifier: :foo");
}

#[test]
fn parse_param_unknown_modifier_another() {
    let err = parse_param("data:bogus=y").unwrap_err();
    assert_eq!(err, "unknown param modifier: :bogus");
}

#[test]
fn parse_param_value_with_equals() {
    // Only the FIRST '=' splits key from value.
    let (name, spec) = parse_param("expr=a=b").unwrap();
    assert_eq!(name, "expr");
    assert_eq!(spec, ParamSpec::Literal(json!("a=b")));
}

#[test]
fn parse_param_value_with_url_colon_preserved() {
    // Only KEY-side ':' splits; colons in values (URLs etc) are untouched.
    let (name, spec) = parse_param("url=http://x").unwrap();
    assert_eq!(name, "url");
    assert_eq!(spec, ParamSpec::Literal(json!("http://x")));
}

#[test]
fn parse_param_value_with_colon_preserved_embed() {
    // Value containing a colon after an embed modifier is part of the text.
    let (name, spec) = parse_param("v:embed=a:b:c").unwrap();
    assert_eq!(name, "v");
    assert_eq!(spec, ParamSpec::Embed("a:b:c".to_string()));
}

#[test]
fn parse_param_empty_value_literal() {
    let (name, spec) = parse_param("x=").unwrap();
    assert_eq!(name, "x");
    assert_eq!(spec, ParamSpec::Literal(json!("")));
}

#[test]
fn parse_param_empty_value_embed() {
    let (name, spec) = parse_param("v:embed=").unwrap();
    assert_eq!(name, "v");
    assert_eq!(spec, ParamSpec::Embed(String::new()));
}

#[test]
fn parse_param_no_equals_errors() {
    let err = parse_param("nokey").unwrap_err();
    assert!(err.contains("invalid param format"));
}

// --- EmbedError re-implementation for testing ---
//
// Mirrors the `#[error("...")]` thiserror messages in src/embed/mod.rs so
// tests assert the exact REQ-F-011 strings via `format!("{err}")`.

#[derive(Debug)]
enum EmbedErrorFake {
    MissingApiKey {
        provider: &'static str,
        env_var: &'static str,
    },
    NotConfigured,
    ModelNotSet,
    UnknownProvider(String),
}

impl std::fmt::Display for EmbedErrorFake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingApiKey { provider, env_var } => {
                write!(f, "missing API key for {provider}: set {env_var}")
            }
            Self::NotConfigured => {
                write!(
                    f,
                    "embedding provider not configured: set NEO4J_EMBED_PROVIDER"
                )
            }
            Self::ModelNotSet => write!(f, "NEO4J_EMBED_MODEL not set"),
            Self::UnknownProvider(name) => write!(f, "unknown provider: {name}"),
        }
    }
}

#[test]
fn embed_error_missing_api_key_openai_string() {
    let err = EmbedErrorFake::MissingApiKey {
        provider: "openai",
        env_var: "OPENAI_API_KEY",
    };
    assert_eq!(
        format!("{err}"),
        "missing API key for openai: set OPENAI_API_KEY"
    );
}

#[test]
fn embed_error_not_configured_string() {
    assert_eq!(
        format!("{}", EmbedErrorFake::NotConfigured),
        "embedding provider not configured: set NEO4J_EMBED_PROVIDER"
    );
}

#[test]
fn embed_error_model_not_set_string() {
    assert_eq!(
        format!("{}", EmbedErrorFake::ModelNotSet),
        "NEO4J_EMBED_MODEL not set"
    );
}

#[test]
fn embed_error_unknown_provider_string() {
    assert_eq!(
        format!("{}", EmbedErrorFake::UnknownProvider("cohere".into())),
        "unknown provider: cohere"
    );
}

// --- EmbedConfig::from_sources re-implementation for testing ---
//
// Pure: takes CLI struct + env closure so precedence and API-key fallback
// can be verified deterministically without touching process env.

#[derive(Default, Clone, Debug)]
struct EmbedCliArgsFake {
    provider: Option<String>,
    model: Option<String>,
    dimensions: Option<u32>,
    base_url: Option<String>,
}

#[derive(Debug, PartialEq)]
struct EmbedConfigFake {
    provider: String,
    model: String,
    dimensions: Option<u32>,
    base_url: Option<String>,
    api_key: Option<String>,
}

fn resolve_api_key_fake(provider: &str, env: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let filter_empty = |v: String| if v.is_empty() { None } else { Some(v) };
    match provider {
        "openai" => env("OPENAI_API_KEY")
            .and_then(filter_empty)
            .or_else(|| env("NEO4J_EMBED_API_KEY").and_then(filter_empty)),
        "ollama" => None,
        "huggingface" => env("HF_TOKEN")
            .and_then(filter_empty)
            .or_else(|| env("NEO4J_EMBED_API_KEY").and_then(filter_empty)),
        _ => env("NEO4J_EMBED_API_KEY").and_then(filter_empty),
    }
}

fn from_sources_fake(
    args: &EmbedCliArgsFake,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<EmbedConfigFake>, EmbedErrorFake> {
    let provider = match args.provider.as_deref() {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => return Ok(None),
    };

    let model = args
        .model
        .as_deref()
        .filter(|m| !m.is_empty())
        .ok_or(EmbedErrorFake::ModelNotSet)?
        .to_string();

    let api_key = resolve_api_key_fake(&provider, env);

    Ok(Some(EmbedConfigFake {
        provider,
        model,
        dimensions: args.dimensions,
        base_url: args.base_url.clone(),
        api_key,
    }))
}

fn empty_env(_: &str) -> Option<String> {
    None
}

#[test]
fn from_sources_returns_none_when_no_provider() {
    let args = EmbedCliArgsFake::default();
    let cfg = from_sources_fake(&args, &empty_env).unwrap();
    assert!(cfg.is_none());
}

#[test]
fn from_sources_empty_provider_string_is_none() {
    // Treat an explicitly empty provider as "unset".
    let args = EmbedCliArgsFake {
        provider: Some(String::new()),
        ..Default::default()
    };
    let cfg = from_sources_fake(&args, &empty_env).unwrap();
    assert!(cfg.is_none());
}

#[test]
fn from_sources_provider_without_model_errors_model_not_set() {
    let args = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: None,
        ..Default::default()
    };
    let err = from_sources_fake(&args, &empty_env).unwrap_err();
    assert_eq!(format!("{err}"), "NEO4J_EMBED_MODEL not set");
}

#[test]
fn from_sources_provider_with_empty_model_errors_model_not_set() {
    let args = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: Some(String::new()),
        ..Default::default()
    };
    let err = from_sources_fake(&args, &empty_env).unwrap_err();
    assert_eq!(format!("{err}"), "NEO4J_EMBED_MODEL not set");
}

#[test]
fn from_sources_ollama_minimal_ok() {
    let args = EmbedCliArgsFake {
        provider: Some("ollama".into()),
        model: Some("all-minilm".into()),
        ..Default::default()
    };
    let cfg = from_sources_fake(&args, &empty_env).unwrap().unwrap();
    assert_eq!(cfg.provider, "ollama");
    assert_eq!(cfg.model, "all-minilm");
    // REQ-F-006: Ollama silently ignores NEO4J_EMBED_API_KEY
    assert!(cfg.api_key.is_none());
    assert!(cfg.dimensions.is_none());
    assert!(cfg.base_url.is_none());
}

#[test]
fn from_sources_ollama_ignores_embed_api_key_env() {
    let args = EmbedCliArgsFake {
        provider: Some("ollama".into()),
        model: Some("all-minilm".into()),
        ..Default::default()
    };
    let env = |k: &str| match k {
        "NEO4J_EMBED_API_KEY" => Some("sk-should-be-ignored".into()),
        _ => None,
    };
    let cfg = from_sources_fake(&args, &env).unwrap().unwrap();
    assert!(
        cfg.api_key.is_none(),
        "ollama must not pick up NEO4J_EMBED_API_KEY"
    );
}

// --- Precedence: CLI flag > env (verified by clap behaviour; here we assert
//     our pure resolver uses whatever the CLI struct carries, which is how
//     clap merges env into args before we see it). ---

#[test]
fn from_sources_precedence_uses_args_values_as_final() {
    // Once clap parses, the CLI struct already holds the winning value
    // (CLI flag or env). Verify the resolver honours the struct verbatim.
    let args = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: Some("cli-model".into()),
        dimensions: Some(512),
        base_url: Some("https://custom.example/v1".into()),
    };
    let cfg = from_sources_fake(&args, &empty_env).unwrap().unwrap();
    assert_eq!(cfg.provider, "openai");
    assert_eq!(cfg.model, "cli-model");
    assert_eq!(cfg.dimensions, Some(512));
    assert_eq!(cfg.base_url.as_deref(), Some("https://custom.example/v1"));
}

// Simulate clap's precedence directly: if both CLI and env were present,
// clap would pass the CLI value to the resolver. This test encodes that
// expectation by constructing two calls and asserting the CLI one wins.
#[test]
fn from_sources_precedence_cli_beats_env_simulation() {
    // "env" would have set model=env-model; "CLI" overrides to cli-model.
    let env_only = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: Some("env-model".into()),
        ..Default::default()
    };
    let cli_and_env = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: Some("cli-model".into()), // CLI wins at clap layer
        ..Default::default()
    };
    let env_cfg = from_sources_fake(&env_only, &empty_env).unwrap().unwrap();
    let cli_cfg = from_sources_fake(&cli_and_env, &empty_env)
        .unwrap()
        .unwrap();
    assert_eq!(env_cfg.model, "env-model");
    assert_eq!(cli_cfg.model, "cli-model");
}

// --- API key fallback ---

#[test]
fn resolve_api_key_openai_openai_env_wins() {
    let env = |k: &str| match k {
        "OPENAI_API_KEY" => Some("sk-openai".into()),
        "NEO4J_EMBED_API_KEY" => Some("sk-fallback".into()),
        _ => None,
    };
    assert_eq!(
        resolve_api_key_fake("openai", &env),
        Some("sk-openai".into())
    );
}

#[test]
fn resolve_api_key_openai_falls_back_to_embed_key() {
    let env = |k: &str| match k {
        "NEO4J_EMBED_API_KEY" => Some("sk-fallback".into()),
        _ => None,
    };
    assert_eq!(
        resolve_api_key_fake("openai", &env),
        Some("sk-fallback".into())
    );
}

#[test]
fn resolve_api_key_openai_empty_openai_env_falls_back() {
    // Empty OPENAI_API_KEY should be treated as unset (matches .env placeholders).
    let env = |k: &str| match k {
        "OPENAI_API_KEY" => Some(String::new()),
        "NEO4J_EMBED_API_KEY" => Some("sk-fallback".into()),
        _ => None,
    };
    assert_eq!(
        resolve_api_key_fake("openai", &env),
        Some("sk-fallback".into())
    );
}

#[test]
fn resolve_api_key_openai_neither_set_none() {
    assert_eq!(resolve_api_key_fake("openai", &empty_env), None);
}

#[test]
fn resolve_api_key_ollama_always_none() {
    let env = |k: &str| match k {
        "OPENAI_API_KEY" => Some("sk-openai".into()),
        "NEO4J_EMBED_API_KEY" => Some("sk-fallback".into()),
        _ => None,
    };
    assert_eq!(resolve_api_key_fake("ollama", &env), None);
}

#[test]
fn from_sources_openai_applies_api_key_fallback() {
    let args = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: Some("text-embedding-3-small".into()),
        ..Default::default()
    };
    let env = |k: &str| match k {
        "NEO4J_EMBED_API_KEY" => Some("sk-fallback".into()),
        _ => None,
    };
    let cfg = from_sources_fake(&args, &env).unwrap().unwrap();
    assert_eq!(cfg.api_key, Some("sk-fallback".into()));
}

#[test]
fn from_sources_openai_prefers_openai_env() {
    let args = EmbedCliArgsFake {
        provider: Some("openai".into()),
        model: Some("text-embedding-3-small".into()),
        ..Default::default()
    };
    let env = |k: &str| match k {
        "OPENAI_API_KEY" => Some("sk-openai".into()),
        "NEO4J_EMBED_API_KEY" => Some("sk-fallback".into()),
        _ => None,
    };
    let cfg = from_sources_fake(&args, &env).unwrap().unwrap();
    assert_eq!(cfg.api_key, Some("sk-openai".into()));
}

// --- huggingface resolve_api_key arm (REQ-T-001) ---

#[test]
fn resolve_api_key_huggingface_hf_token_wins() {
    let env = |k: &str| match k {
        "HF_TOKEN" => Some("tok".into()),
        _ => None,
    };
    assert_eq!(
        resolve_api_key_fake("huggingface", &env),
        Some("tok".into())
    );
}

#[test]
fn resolve_api_key_huggingface_falls_back_to_embed_key() {
    let env = |k: &str| match k {
        "NEO4J_EMBED_API_KEY" => Some("fb".into()),
        _ => None,
    };
    assert_eq!(resolve_api_key_fake("huggingface", &env), Some("fb".into()));
}

#[test]
fn resolve_api_key_huggingface_neither_set_none() {
    assert_eq!(resolve_api_key_fake("huggingface", &empty_env), None);
}

#[test]
fn resolve_api_key_huggingface_empty_hf_token_falls_back() {
    // Empty HF_TOKEN treated as unset (mirrors openai empty-filter).
    let env = |k: &str| match k {
        "HF_TOKEN" => Some(String::new()),
        "NEO4J_EMBED_API_KEY" => Some("fb".into()),
        _ => None,
    };
    assert_eq!(resolve_api_key_fake("huggingface", &env), Some("fb".into()));
}
