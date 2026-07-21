use serde_json::{Value, json};

use crate::{HostError, HostErrorCode, HostJsonValue, HostValue};

pub(crate) fn host_value_to_tagged_json_string(value: &HostValue) -> Result<String, HostError> {
    tagged_host_value_json(value)
        .and_then(|value| serde_json::to_string(&value).map_err(json_error))
}

pub(crate) fn host_value_from_tagged_json_str(value: &str) -> Result<HostValue, HostError> {
    let value = serde_json::from_str::<Value>(value).map_err(json_error)?;
    tagged_host_value_from_json(&value)
}

fn tagged_host_value_json(value: &HostValue) -> Result<Value, HostError> {
    Ok(match value {
        HostValue::Unit => json!({ "kind": "unit" }),
        HostValue::Bool(value) => json!({ "kind": "bool", "value": value }),
        HostValue::Int(value) => json!({ "kind": "int", "value": value.to_string() }),
        HostValue::UInt(value) => json!({ "kind": "uint", "value": value.to_string() }),
        HostValue::Float(value) if value.is_finite() => {
            json!({ "kind": "float", "value": value })
        }
        HostValue::Float(_) => {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "non-finite floating-point host value cannot be stored",
            ));
        }
        HostValue::String(value) => json!({ "kind": "string", "value": value }),
        HostValue::Bytes(value) => json!({ "kind": "bytes", "value": value }),
        HostValue::List(values) => json!({
            "kind": "list",
            "items": values.iter().map(tagged_host_value_json).collect::<Result<Vec<_>, _>>()?,
        }),
        HostValue::Map(entries) => json!({
            "kind": "map",
            "entries": entries.iter().map(|(key, value)| {
                Ok(json!({
                    "key": tagged_host_value_json(key)?,
                    "value": tagged_host_value_json(value)?,
                }))
            }).collect::<Result<Vec<_>, HostError>>()?,
        }),
        HostValue::Record(fields) => json!({
            "kind": "record",
            "fields": fields.iter().map(|(name, value)| {
                Ok(json!({
                    "name": name,
                    "value": tagged_host_value_json(value)?,
                }))
            }).collect::<Result<Vec<_>, HostError>>()?,
        }),
        HostValue::Variant { name, fields } => json!({
            "kind": "variant",
            "name": name,
            "fields": fields.iter().map(tagged_host_value_json).collect::<Result<Vec<_>, _>>()?,
        }),
        HostValue::Json(value) => json!({
            "kind": "json",
            "value": host_json_value_json(value),
        }),
    })
}

fn tagged_host_value_from_json(value: &Value) -> Result<HostValue, HostError> {
    let kind = string_field(value, "kind")?;
    match kind {
        "unit" => Ok(HostValue::Unit),
        "bool" => bool_field(value, "value").map(HostValue::Bool),
        "int" => string_field(value, "value")?
            .parse::<i128>()
            .map(HostValue::Int)
            .map_err(json_error),
        "uint" => string_field(value, "value")?
            .parse::<u128>()
            .map(HostValue::UInt)
            .map_err(json_error),
        "float" => number_field(value, "value").and_then(|number| {
            if number.is_finite() {
                Ok(HostValue::Float(number))
            } else {
                Err(HostError::new(
                    HostErrorCode::SchemaMismatch,
                    "stored host value float is not finite",
                ))
            }
        }),
        "string" => string_field(value, "value").map(|value| HostValue::String(value.to_owned())),
        "bytes" => value
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| schema_error("stored host value bytes field is missing value array"))?
            .iter()
            .map(|byte| {
                byte.as_u64()
                    .and_then(|byte| u8::try_from(byte).ok())
                    .ok_or_else(|| schema_error("stored host value byte is outside u8 range"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(HostValue::Bytes),
        "list" => array_field(value, "items")?
            .iter()
            .map(tagged_host_value_from_json)
            .collect::<Result<Vec<_>, _>>()
            .map(HostValue::List),
        "map" => array_field(value, "entries")?
            .iter()
            .map(|entry| {
                Ok((
                    tagged_host_value_from_json(required_field(entry, "key")?)?,
                    tagged_host_value_from_json(required_field(entry, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>, HostError>>()
            .map(HostValue::Map),
        "record" => array_field(value, "fields")?
            .iter()
            .map(|field| {
                Ok((
                    string_field(field, "name")?.to_owned(),
                    tagged_host_value_from_json(required_field(field, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>, HostError>>()
            .map(HostValue::Record),
        "variant" => Ok(HostValue::Variant {
            name: string_field(value, "name")?.to_owned(),
            fields: array_field(value, "fields")?
                .iter()
                .map(tagged_host_value_from_json)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        "json" => host_json_value_from_json(required_field(value, "value")?).map(HostValue::Json),
        other => Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "stored host value has unknown kind",
        )
        .with_detail("kind", other)),
    }
}

fn host_json_value_json(value: &HostJsonValue) -> Value {
    match value {
        HostJsonValue::Null => Value::Null,
        HostJsonValue::Bool(value) => Value::Bool(*value),
        HostJsonValue::Number(value) => json!(value),
        HostJsonValue::String(value) => Value::String(value.clone()),
        HostJsonValue::Array(values) => {
            Value::Array(values.iter().map(host_json_value_json).collect())
        }
        HostJsonValue::Object(entries) => Value::Object(
            entries
                .iter()
                .map(|(name, value)| (name.clone(), host_json_value_json(value)))
                .collect(),
        ),
    }
}

fn host_json_value_from_json(value: &Value) -> Result<HostJsonValue, HostError> {
    Ok(match value {
        Value::Null => HostJsonValue::Null,
        Value::Bool(value) => HostJsonValue::Bool(*value),
        Value::Number(value) => HostJsonValue::Number(
            value
                .as_f64()
                .ok_or_else(|| schema_error("stored JSON number cannot be represented as f64"))?,
        ),
        Value::String(value) => HostJsonValue::String(value.clone()),
        Value::Array(values) => HostJsonValue::Array(
            values
                .iter()
                .map(host_json_value_from_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(entries) => HostJsonValue::Object(
            entries
                .iter()
                .map(|(name, value)| Ok((name.clone(), host_json_value_from_json(value)?)))
                .collect::<Result<Vec<_>, HostError>>()?,
        ),
    })
}

fn required_field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, HostError> {
    value
        .get(name)
        .ok_or_else(|| schema_error(format!("stored host value is missing `{name}` field")))
}

fn string_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, HostError> {
    required_field(value, name)?
        .as_str()
        .ok_or_else(|| schema_error(format!("stored host value `{name}` field must be a string")))
}

fn bool_field(value: &Value, name: &str) -> Result<bool, HostError> {
    required_field(value, name)?
        .as_bool()
        .ok_or_else(|| schema_error(format!("stored host value `{name}` field must be a bool")))
}

fn number_field(value: &Value, name: &str) -> Result<f64, HostError> {
    required_field(value, name)?
        .as_f64()
        .ok_or_else(|| schema_error(format!("stored host value `{name}` field must be a number")))
}

fn array_field<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], HostError> {
    required_field(value, name)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| schema_error(format!("stored host value `{name}` field must be an array")))
}

fn json_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(HostErrorCode::SchemaMismatch, "host value JSON codec error")
        .with_detail("error", error.to_string())
}

fn schema_error(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::SchemaMismatch, message)
}
