use serde_json::{Map, Value};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ParamSpec {
    Literal(Value),
    Embed(String),
}

#[allow(dead_code)]
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

pub fn parse_params(pairs: &[String]) -> Result<Map<String, Value>, String> {
    let mut map = Map::new();
    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| format!("invalid param format '{pair}', expected key=value"))?;
        map.insert(k.to_string(), parse_param_value(v));
    }
    Ok(map)
}
