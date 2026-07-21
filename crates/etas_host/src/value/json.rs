use std::collections::BTreeSet;

use serde_json::{Number, Value};

use crate::{HostError, HostErrorCode, HostJsonValue, HostValue};

pub fn host_value_to_json(value: &HostValue) -> Result<Value, HostError> {
    match value {
        HostValue::Unit => Ok(Value::Null),
        HostValue::Bool(value) => Ok(Value::Bool(*value)),
        HostValue::Int(value) => {
            let Ok(value) = i64::try_from(*value) else {
                return Err(json_number_error(
                    "signed integer is outside JSON i64 range",
                ));
            };
            Ok(Value::Number(Number::from(value)))
        }
        HostValue::UInt(value) => {
            let Ok(value) = u64::try_from(*value) else {
                return Err(json_number_error(
                    "unsigned integer is outside JSON u64 range",
                ));
            };
            Ok(Value::Number(Number::from(value)))
        }
        HostValue::Float(value) => Number::from_f64(*value)
            .map(Value::Number)
            .ok_or_else(|| json_number_error("floating-point value is not finite")),
        HostValue::String(value) => Ok(Value::String(value.clone())),
        HostValue::Bytes(value) => Ok(Value::Array(
            value
                .iter()
                .map(|byte| Value::Number(Number::from(*byte)))
                .collect(),
        )),
        HostValue::List(values) => values
            .iter()
            .map(host_value_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        HostValue::Map(entries) => entries
            .iter()
            .map(|(key, value)| {
                Ok(Value::Array(vec![
                    host_value_to_json(key)?,
                    host_value_to_json(value)?,
                ]))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        HostValue::Record(fields) => fields_to_json_object(fields),
        HostValue::Variant { name, fields } => {
            let mut object = serde_json::Map::new();
            object.insert("name".to_owned(), Value::String(name.clone()));
            object.insert(
                "fields".to_owned(),
                Value::Array(
                    fields
                        .iter()
                        .map(host_value_to_json)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            );
            Ok(Value::Object(object))
        }
        HostValue::Json(value) => host_json_value_to_json(value),
    }
}

pub fn host_json_to_value(value: Value) -> Result<HostValue, HostError> {
    Ok(HostValue::Json(json_to_host_json_value(value)?))
}

fn host_json_value_to_json(value: &HostJsonValue) -> Result<Value, HostError> {
    match value {
        HostJsonValue::Null => Ok(Value::Null),
        HostJsonValue::Bool(value) => Ok(Value::Bool(*value)),
        HostJsonValue::Number(value) => Number::from_f64(*value)
            .map(Value::Number)
            .ok_or_else(|| json_number_error("host JSON number is not finite")),
        HostJsonValue::String(value) => Ok(Value::String(value.clone())),
        HostJsonValue::Array(values) => values
            .iter()
            .map(host_json_value_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        HostJsonValue::Object(entries) => host_json_fields_to_object(entries),
    }
}

fn json_to_host_json_value(value: Value) -> Result<HostJsonValue, HostError> {
    match value {
        Value::Null => Ok(HostJsonValue::Null),
        Value::Bool(value) => Ok(HostJsonValue::Bool(value)),
        Value::Number(value) => value
            .as_f64()
            .map(HostJsonValue::Number)
            .ok_or_else(|| json_number_error("JSON number cannot be represented as host f64")),
        Value::String(value) => Ok(HostJsonValue::String(value)),
        Value::Array(values) => values
            .into_iter()
            .map(json_to_host_json_value)
            .collect::<Result<Vec<_>, _>>()
            .map(HostJsonValue::Array),
        Value::Object(entries) => entries
            .into_iter()
            .map(|(name, value)| Ok((name, json_to_host_json_value(value)?)))
            .collect::<Result<Vec<_>, _>>()
            .map(HostJsonValue::Object),
    }
}

fn fields_to_json_object(fields: &[(String, HostValue)]) -> Result<Value, HostError> {
    let mut seen = BTreeSet::new();
    let mut object = serde_json::Map::new();
    for (name, value) in fields {
        if !seen.insert(name) {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "record contains duplicate JSON field name",
            )
            .with_detail("field", name));
        }
        object.insert(name.clone(), host_value_to_json(value)?);
    }
    Ok(Value::Object(object))
}

fn host_json_fields_to_object(entries: &[(String, HostJsonValue)]) -> Result<Value, HostError> {
    let mut seen = BTreeSet::new();
    let mut object = serde_json::Map::new();
    for (name, value) in entries {
        if !seen.insert(name) {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "host JSON object contains duplicate field name",
            )
            .with_detail("field", name));
        }
        object.insert(name.clone(), host_json_value_to_json(value)?);
    }
    Ok(Value::Object(object))
}

fn json_number_error(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::SchemaMismatch, message)
}
