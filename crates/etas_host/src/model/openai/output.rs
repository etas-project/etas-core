use serde_json::{Map, Value, json};

use super::{OPENAI_OUTPUT_TOOL, OpenAiProviderDialect};
use crate::model::tool_schema::host_schema_to_json_schema;
use crate::{HostError, HostErrorCode, HostSchema, ModelProviderCapabilities};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAiTypedOutputEncoding {
    JsonSchema,
    OutputTool,
}

impl OpenAiTypedOutputEncoding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JsonSchema => "json-schema",
            Self::OutputTool => "output-tool",
        }
    }
}

impl std::str::FromStr for OpenAiTypedOutputEncoding {
    type Err = HostError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "json-schema" => Ok(Self::JsonSchema),
            "output-tool" => Ok(Self::OutputTool),
            _ => Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "typed output encoding must be `json-schema` or `output-tool`",
            )),
        }
    }
}

pub(super) fn default_encoding(dialect: OpenAiProviderDialect) -> OpenAiTypedOutputEncoding {
    match dialect {
        OpenAiProviderDialect::LegacyFunctions => OpenAiTypedOutputEncoding::OutputTool,
        _ => OpenAiTypedOutputEncoding::JsonSchema,
    }
}

pub(super) fn resolve(
    dialect: OpenAiProviderDialect,
    configured: Option<OpenAiTypedOutputEncoding>,
) -> Result<OpenAiTypedOutputEncoding, HostError> {
    let encoding = configured.unwrap_or_else(|| default_encoding(dialect));
    if encoding == OpenAiTypedOutputEncoding::JsonSchema
        && dialect == OpenAiProviderDialect::LegacyFunctions
    {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "legacy functions dialect does not support json-schema output encoding",
        ));
    }
    Ok(encoding)
}

pub(super) fn capabilities(encoding: OpenAiTypedOutputEncoding) -> ModelProviderCapabilities {
    ModelProviderCapabilities {
        supports_forced_tool_output: encoding == OpenAiTypedOutputEncoding::OutputTool,
        supports_json_schema_response_format: encoding == OpenAiTypedOutputEncoding::JsonSchema,
        supports_plain_json_text_instruction: true,
        supports_tool_call_loop: true,
        supports_required_tool_choice: true,
    }
}

pub(super) fn insert_format(
    body: &mut Map<String, Value>,
    schema: &HostSchema,
) -> Result<(), HostError> {
    body.insert("response_format".into(), json!({
        "type": "json_schema",
        "json_schema": { "name": "etas_output", "strict": true, "schema": host_schema_to_json_schema(schema)? }
    }));
    Ok(())
}

pub(super) fn insert_tool(
    body: &mut Map<String, Value>,
    schema: &HostSchema,
    dialect: OpenAiProviderDialect,
    force_output: bool,
) -> Result<(), HostError> {
    if !force_output && dialect == OpenAiProviderDialect::LegacyFunctions {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "legacy functions cannot require a typed output or application tool choice",
        ));
    }
    let function = json!({
        "name": OPENAI_OUTPUT_TOOL,
        "description": "Emit the final Etas agent output.",
        "parameters": host_schema_to_json_schema(schema)?,
    });
    let (key, tool, choice_key, choice) = match dialect {
        OpenAiProviderDialect::LegacyFunctions => (
            "functions",
            function,
            "function_call",
            json!({"name":OPENAI_OUTPUT_TOOL}),
        ),
        _ => (
            "tools",
            json!({"type":"function", "function":function}),
            "tool_choice",
            if force_output {
                json!({"type":"function", "function":{"name":OPENAI_OUTPUT_TOOL}})
            } else {
                json!("required")
            },
        ),
    };
    match body.get_mut(key) {
        Some(Value::Array(tools)) => tools.push(tool),
        None => {
            body.insert(key.into(), Value::Array(vec![tool]));
        }
        Some(_) => {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "encoded model tools must be an array",
            ));
        }
    }
    body.insert(choice_key.into(), choice);
    Ok(())
}
