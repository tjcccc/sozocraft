//! Local subset of the official request schemas, checked before paid submissions.
use serde_json::Value;

pub(super) fn validate(value: &Value, name: &str) -> Result<(), String> {
    let schemas: Value = serde_json::from_str(include_str!("request_schemas.json"))
        .expect("bundled PixelLab schema");
    check(value, &schemas["components"]["schemas"][name], &schemas)
        .map_err(|_| "Invalid PixelLab request: check fields, types, and ranges against the documented REST schema.".to_string())
}

// This deliberately implements only the keywords used by the bundled schema subset.
fn check(v: &Value, s: &Value, root: &Value) -> Result<(), ()> {
    if let Some(reference) = s["$ref"].as_str() {
        return check(
            v,
            root.pointer(reference.trim_start_matches('#')).ok_or(())?,
            root,
        );
    }
    if let Some(alternatives) = s["anyOf"].as_array() {
        return if alternatives.iter().any(|s| check(v, s, root).is_ok()) {
            Ok(())
        } else {
            Err(())
        };
    }
    if s.get("const").is_some_and(|c| c != v)
        || s["enum"].as_array().is_some_and(|a| !a.contains(v))
    {
        return Err(());
    }
    match s["type"].as_str().ok_or(())? {
        "null" if v.is_null() => {}
        "boolean" if v.is_boolean() => {}
        "string" if v.is_string() => {
            let text = v.as_str().ok_or(())?;
            let len = text.chars().count() as u64;
            if s["minLength"].as_u64().is_some_and(|n| len < n)
                || s["maxLength"].as_u64().is_some_and(|n| len > n)
                || (s["format"] == "uuid" && uuid::Uuid::parse_str(text).is_err())
            {
                return Err(());
            }
        }
        "integer" if v.is_i64() || v.is_u64() => bounds(v, s)?,
        "number" if v.is_number() => bounds(v, s)?,
        "array" if v.is_array() => {
            for item in v.as_array().ok_or(())? {
                check(item, &s["items"], root)?;
            }
        }
        "object" if v.is_object() => {
            let object = v.as_object().ok_or(())?;
            if let Some(required) = s["required"].as_array() {
                for field in required {
                    if !object.contains_key(field.as_str().ok_or(())?) {
                        return Err(());
                    }
                }
            }
            for (key, value) in object {
                let property = s["properties"].get(key).ok_or(())?;
                check(value, property, root)?;
            }
        }
        _ => return Err(()),
    }
    Ok(())
}

fn bounds(v: &Value, s: &Value) -> Result<(), ()> {
    let n = v.as_f64().ok_or(())?;
    if s["minimum"].as_f64().is_some_and(|min| n < min)
        || s["maximum"].as_f64().is_some_and(|max| n > max)
    {
        Err(())
    } else {
        Ok(())
    }
}
