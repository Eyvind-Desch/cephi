//! Validation and normalization of arguments produced by the model.

use std::collections::BTreeMap;

use serde_json::{Number, Value};

use super::manifest::{ParamType, Parameters};

pub type Args = BTreeMap<String, Value>;

/// Checks model-provided arguments against the manifest.
///
/// Small models are sloppy, so this is lenient where it is safe: arguments may arrive as a
/// JSON string, scalars are coerced (`"5"` → `5`), unknown keys are dropped and defaults
/// are applied. Errors are phrased for the model so it can retry.
pub fn normalize(params: &Parameters, raw: &Value) -> Result<Args, String> {
    let object = match raw {
        Value::Object(map) => map.clone(),
        Value::Null => Default::default(),
        Value::String(s) if s.trim().is_empty() => Default::default(),
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(Value::Object(map)) => map,
            _ => return Err("Argumente müssen ein JSON-Objekt sein.".into()),
        },
        _ => return Err("Argumente müssen ein JSON-Objekt sein.".into()),
    };

    let mut args = Args::new();
    for (name, spec) in &params.properties {
        let value = match object.get(name) {
            Some(Value::Null) | None => match &spec.default {
                Some(default) => default.clone(),
                None if params.required.contains(name) => {
                    return Err(format!("Pflichtparameter \"{name}\" fehlt."));
                }
                None => continue,
            },
            Some(value) => coerce(name, spec.kind, value)?,
        };

        if let Some(allowed) = &spec.allowed {
            if !allowed.contains(&value) {
                let options = allowed.iter().map(Value::to_string).collect::<Vec<_>>().join(", ");
                return Err(format!("\"{name}\" muss einer dieser Werte sein: {options}."));
            }
        }
        args.insert(name.clone(), value);
    }
    Ok(args)
}

fn coerce(name: &str, kind: ParamType, value: &Value) -> Result<Value, String> {
    let fail = || format!("\"{name}\" muss vom Typ {kind:?} sein, erhalten: {value}.").to_lowercase();
    match (kind, value) {
        (ParamType::String, Value::String(_)) => Ok(value.clone()),
        (ParamType::String, Value::Number(_) | Value::Bool(_)) => Ok(Value::String(value.to_string())),

        (ParamType::Boolean, Value::Bool(_)) => Ok(value.clone()),
        (ParamType::Boolean, Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(fail()),
        },

        (ParamType::Integer, Value::Number(n)) if n.is_i64() || n.is_u64() => Ok(value.clone()),
        (ParamType::Integer, Value::String(s)) => s.trim().parse::<i64>().map(Value::from).map_err(|_| fail()),

        (ParamType::Number, Value::Number(_)) => Ok(value.clone()),
        (ParamType::Number, Value::String(s)) => s
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(fail),

        _ => Err(fail()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params() -> Parameters {
        serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "count": { "type": "integer", "default": 3 },
                "mode":  { "type": "string", "enum": ["a", "b"] }
            },
            "required": ["query"]
        }))
        .unwrap()
    }

    #[test]
    fn applies_defaults_and_drops_unknown() {
        let args = normalize(&params(), &json!({ "query": "x", "junk": 1 })).unwrap();
        assert_eq!(args.get("count"), Some(&json!(3)));
        assert!(!args.contains_key("junk"));
        assert!(!args.contains_key("mode"));
    }

    #[test]
    fn accepts_stringified_json_and_coerces() {
        let args = normalize(&params(), &json!(r#"{"query": 42, "count": "7"}"#)).unwrap();
        assert_eq!(args.get("query"), Some(&json!("42")));
        assert_eq!(args.get("count"), Some(&json!(7)));
    }

    #[test]
    fn reports_missing_required_and_enum_violations() {
        assert!(normalize(&params(), &json!({})).unwrap_err().contains("query"));
        assert!(normalize(&params(), &json!({ "query": "x", "mode": "c" })).is_err());
    }
}
