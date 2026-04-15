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
