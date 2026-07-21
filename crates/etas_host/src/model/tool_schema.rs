use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::{HostError, HostErrorCode, HostFieldSchema, HostSchema, HostVariantSchema, ToolSchema};

pub(crate) fn openai_tools(schemas: &[ToolSchema]) -> Result<Value, HostError> {
    let tools = schemas
        .iter()
        .map(|schema| {
            Ok(json!({
                "type": "function",
                "function": {
                    "name": schema.tool.name,
                    "parameters": host_schema_to_json_schema(&schema.input)?,
                },
            }))
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    Ok(Value::Array(tools))
}

pub(crate) fn openai_legacy_functions(schemas: &[ToolSchema]) -> Result<Value, HostError> {
    let functions = schemas
        .iter()
        .map(|schema| {
            Ok(json!({
                "name": schema.tool.name,
                "parameters": host_schema_to_json_schema(&schema.input)?,
            }))
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    Ok(Value::Array(functions))
}

pub(crate) fn anthropic_tools(schemas: &[ToolSchema]) -> Result<Value, HostError> {
    let tools = schemas
        .iter()
        .map(|schema| {
            Ok(json!({
                "name": schema.tool.name,
                "input_schema": host_schema_to_json_schema(&schema.input)?,
            }))
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    Ok(Value::Array(tools))
}

pub(crate) fn host_schema_to_json_schema(schema: &HostSchema) -> Result<Value, HostError> {
    match schema {
        HostSchema::Unit => Ok(json!({ "type": "null" })),
        HostSchema::Bool => Ok(json!({ "type": "boolean" })),
        HostSchema::Int | HostSchema::UInt => Ok(json!({ "type": "integer" })),
        HostSchema::Float => Ok(json!({ "type": "number" })),
        HostSchema::String => Ok(json!({ "type": "string" })),
        HostSchema::Bytes => Ok(json!({
            "type": "array",
            "items": { "type": "integer", "minimum": 0, "maximum": 255 },
        })),
        HostSchema::List(item) => Ok(json!({
            "type": "array",
            "items": host_schema_to_json_schema(item)?,
        })),
        HostSchema::Map { key, value } => map_schema_to_json_schema(key, value),
        HostSchema::Record(fields) => record_schema_to_json_schema(fields),
        HostSchema::Variant(variants) => variant_schema_to_json_schema(variants),
        HostSchema::Json => Ok(json!({})),
    }
}

fn map_schema_to_json_schema(key: &HostSchema, value: &HostSchema) -> Result<Value, HostError> {
    if matches!(key, HostSchema::String) {
        Ok(json!({
            "type": "object",
            "additionalProperties": host_schema_to_json_schema(value)?,
        }))
    } else {
        Ok(json!({
            "type": "array",
            "items": {
                "type": "array",
                "prefixItems": [
                    host_schema_to_json_schema(key)?,
                    host_schema_to_json_schema(value)?,
                ],
                "minItems": 2,
                "maxItems": 2,
            },
        }))
    }
}

fn record_schema_to_json_schema(fields: &[HostFieldSchema]) -> Result<Value, HostError> {
    let mut seen = BTreeSet::new();
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for field in fields {
        if !seen.insert(field.name.as_str()) {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "tool schema contains duplicate record field",
            )
            .with_detail("field", &field.name));
        }
        properties.insert(
            field.name.clone(),
            host_schema_to_json_schema(&field.schema)?,
        );
        if !field.optional {
            required.push(Value::String(field.name.clone()));
        }
    }
    Ok(json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": required,
        "additionalProperties": false,
    }))
}

fn variant_schema_to_json_schema(variants: &[HostVariantSchema]) -> Result<Value, HostError> {
    let mut seen = BTreeSet::new();
    let mut one_of = Vec::new();
    for variant in variants {
        if !seen.insert(variant.name.as_str()) {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "tool schema contains duplicate variant name",
            )
            .with_detail("variant", &variant.name));
        }
        one_of.push(json!({
            "type": "object",
            "properties": {
                "name": { "const": variant.name.clone() },
                "fields": {
                    "type": "array",
                    "prefixItems": variant
                        .fields
                        .iter()
                        .map(host_schema_to_json_schema)
                        .collect::<Result<Vec<_>, _>>()?,
                    "minItems": variant.fields.len(),
                    "maxItems": variant.fields.len(),
                },
            },
            "required": ["name", "fields"],
            "additionalProperties": false,
        }));
    }
    Ok(json!({ "oneOf": one_of }))
}

#[cfg(test)]
mod tests {
    use crate::{HostFieldSchema, ToolRef};

    use super::*;

    #[test]
    fn openai_tool_schema_encodes_record_input() {
        let tools = openai_tools(&[echo_tool_schema()]).expect("tool schema should encode");
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "host.echo");
        assert_eq!(
            tools[0]["function"]["parameters"]["properties"]["message"]["type"],
            "string"
        );
        assert_eq!(
            tools[0]["function"]["parameters"]["required"],
            json!(["message"])
        );
    }

    #[test]
    fn anthropic_tool_schema_encodes_record_input() {
        let tools = anthropic_tools(&[echo_tool_schema()]).expect("tool schema should encode");
        assert_eq!(tools[0]["name"], "host.echo");
        assert_eq!(
            tools[0]["input_schema"]["properties"]["message"]["type"],
            "string"
        );
    }

    #[test]
    fn duplicate_tool_schema_fields_are_rejected() {
        let schema = ToolSchema {
            tool: ToolRef::anonymous_test("host.bad"),
            input: HostSchema::Record(vec![
                HostFieldSchema {
                    name: "x".to_owned(),
                    schema: HostSchema::String,
                    optional: false,
                },
                HostFieldSchema {
                    name: "x".to_owned(),
                    schema: HostSchema::Bool,
                    optional: false,
                },
            ]),
            output: None,
        };
        assert_eq!(
            openai_tools(&[schema])
                .expect_err("duplicate schema field should fail")
                .code,
            HostErrorCode::InvalidRequest
        );
    }

    fn echo_tool_schema() -> ToolSchema {
        ToolSchema {
            tool: ToolRef::anonymous_test("host.echo"),
            input: HostSchema::Record(vec![HostFieldSchema {
                name: "message".to_owned(),
                schema: HostSchema::String,
                optional: false,
            }]),
            output: None,
        }
    }
}
