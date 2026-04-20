use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ParamSpec {
    Literal(Value),
    Embed(String),
}

pub fn parse_param(raw: &str) -> Result<(String, ParamSpec), String> {
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

pub fn parse_param_value(v: &str) -> Value {
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

/// Parse a list of `-P key=value` pairs into `(name, ParamSpec)` tuples.
///
/// Preserves order so error messages point at the first offender and callers
/// that care about insertion order (e.g. lazy provider init that only fires
/// on the first `:embed`) behave deterministically.
pub fn parse_param_specs(pairs: &[String]) -> Result<Vec<(String, ParamSpec)>, String> {
    pairs.iter().map(|p| parse_param(p)).collect()
}
