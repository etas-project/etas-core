use std::{future::Future, pin::Pin};

use serde_json::{Value, json};

use crate::{
    AuthConfig, HostError, HostErrorCode, HostValue, HttpTransport, ModelClient, ModelContent,
    ModelMessage, ModelProviderCapabilities, ModelRequest, ModelResponse, ModelRole, ModelToolCall,
    ModelToolChoice, ModelUsage, PrivateResolutionPolicy, RetryPolicy, TransportTimeoutPolicy,
    host_json_to_value, host_value_to_json,
};

use super::tool_schema::{host_schema_to_json_schema, openai_legacy_functions, openai_tools};

const OPENAI_OUTPUT_TOOL: &str = "etas_emit_output";

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiProviderRequest {
    pub base_url: String,
    pub request: ModelRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenAiProviderResponse {
    pub response: ModelResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiProtocolAdapter {
    pub base_url: String,
    pub transport: HttpTransport,
    pub dialect: OpenAiProviderDialect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAiProviderDialect {
    OpenAiTools,
    LegacyFunctions,
    OmlxCompatible,
}

impl OpenAiProtocolAdapter {
    pub const LOCAL_OMLX_BASE_URL: &'static str = "http://127.0.0.1:8848/v1";

    pub fn new(base_url: impl Into<String>) -> Result<Self, HostError> {
        Self::try_new_with_policy(base_url, PrivateResolutionPolicy::PublicOnly)
    }

    pub fn try_new_with_policy(
        base_url: impl Into<String>,
        policy: PrivateResolutionPolicy,
    ) -> Result<Self, HostError> {
        let base_url = base_url.into();
        Ok(Self {
            transport: HttpTransport::try_new(&base_url, policy)?,
            base_url,
            dialect: OpenAiProviderDialect::OpenAiTools,
        })
    }

    pub fn local_omlx() -> Result<Self, HostError> {
        Self::omlx_compatible(Self::LOCAL_OMLX_BASE_URL)
    }

    pub fn omlx_compatible(base_url: impl Into<String>) -> Result<Self, HostError> {
        Ok(
            Self::try_new_with_policy(base_url, PrivateResolutionPolicy::AllowPrivate)?
                .with_dialect(OpenAiProviderDialect::OmlxCompatible),
        )
    }

    pub fn legacy_functions(base_url: impl Into<String>) -> Result<Self, HostError> {
        Ok(Self::new(base_url)?.with_dialect(OpenAiProviderDialect::LegacyFunctions))
    }

    pub fn with_dialect(mut self, dialect: OpenAiProviderDialect) -> Self {
        self.dialect = dialect;
        self
    }

    pub fn capabilities() -> ModelProviderCapabilities {
        Self::capabilities_for_dialect(OpenAiProviderDialect::OpenAiTools)
    }

    pub fn omlx_capabilities() -> ModelProviderCapabilities {
        Self::capabilities_for_dialect(OpenAiProviderDialect::OmlxCompatible)
    }

    pub fn legacy_capabilities() -> ModelProviderCapabilities {
        Self::capabilities_for_dialect(OpenAiProviderDialect::LegacyFunctions)
    }

    pub fn capabilities_for_dialect(dialect: OpenAiProviderDialect) -> ModelProviderCapabilities {
        ModelProviderCapabilities {
            supports_forced_tool_output: true,
            supports_json_schema_response_format: !matches!(
                dialect,
                OpenAiProviderDialect::LegacyFunctions
            ),
            supports_plain_json_text_instruction: true,
            supports_tool_call_loop: true,
            supports_required_tool_choice: true,
        }
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.transport = self.transport.with_auth(auth);
        self
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.transport = self.transport.with_retry(retry);
        self
    }

    pub fn with_timeout(mut self, timeout: TransportTimeoutPolicy) -> Self {
        self.transport = self.transport.with_timeout(timeout);
        self
    }

    pub fn encode_request(&self, request: ModelRequest) -> OpenAiProviderRequest {
        OpenAiProviderRequest {
            base_url: self.base_url.clone(),
            request,
        }
    }

    pub fn decode_response(response: OpenAiProviderResponse) -> Result<ModelResponse, HostError> {
        Ok(response.response)
    }

    async fn complete_request(&self, request: ModelRequest) -> Result<ModelResponse, HostError> {
        let id = request.id;
        let budget_deadline = request.budget.deadline()?;
        let body = encode_openai_chat_request_with_dialect(&request, self.dialect)?;
        let response = self
            .transport
            .send_json_with_deadline("/chat/completions", body, budget_deadline)
            .await?;
        if !(200..300).contains(&response.status) {
            return Err(HostError::new(
                HostErrorCode::ProviderRejected,
                "OpenAI-compatible endpoint returned an error status",
            )
            .with_detail("status", response.status.to_string()));
        }
        decode_openai_chat_response(id, &response.body)
    }
}

impl ModelClient for OpenAiProtocolAdapter {
    type Error = HostError;
    type CompleteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ModelResponse, Self::Error>> + Send + 'a>>;

    fn complete(&self, request: ModelRequest) -> Self::CompleteFuture<'_> {
        Box::pin(async move { self.complete_request(request).await })
    }
}

fn encode_openai_chat_request_with_dialect(
    request: &ModelRequest,
    dialect: OpenAiProviderDialect,
) -> Result<String, HostError> {
    let require_model_tool =
        !request.tools.is_empty() && !matches!(request.tool_choice, ModelToolChoice::Auto);
    let mut messages = request
        .messages
        .iter()
        .map(encode_openai_message)
        .collect::<Result<Vec<_>, HostError>>()?;
    if let Some(schema) = &request.response_schema
        && !require_model_tool
    {
        messages.insert(0, structured_output_instruction(schema)?);
    }
    let mut body = serde_json::Map::new();
    body.insert("model".to_owned(), Value::String(request.model.0.clone()));
    body.insert("messages".to_owned(), Value::Array(messages));
    if let Some(temperature) = request.options.temperature {
        body.insert(
            "temperature".to_owned(),
            serde_json::Number::from_f64(f64::from(temperature))
                .map(Value::Number)
                .ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::InvalidRequest,
                        "model temperature is not finite",
                    )
                })?,
        );
    }
    if let Some(max_tokens) = request.options.max_output_tokens {
        body.insert(
            "max_tokens".to_owned(),
            Value::Number(serde_json::Number::from(max_tokens)),
        );
    }
    if !request.options.metadata.is_empty() {
        body.insert(
            "metadata".to_owned(),
            metadata_to_json_object(&request.options.metadata)?,
        );
    }
    if let Some(schema) = &request.response_schema
        && request.tools.is_empty()
        && !require_model_tool
    {
        insert_forced_output_tool(&mut body, schema, dialect)?;
    } else if !request.tools.is_empty() {
        insert_model_tools(&mut body, request, dialect)?;
    }
    if let Some(schema) = &request.response_schema
        && !require_model_tool
        && dialect_supports_response_format(dialect)
    {
        body.insert(
            "response_format".to_owned(),
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "etas_output",
                    "strict": true,
                    "schema": host_schema_to_json_schema(schema)?,
                },
            }),
        );
    }
    Ok(Value::Object(body).to_string())
}

fn insert_forced_output_tool(
    body: &mut serde_json::Map<String, Value>,
    schema: &crate::HostSchema,
    dialect: OpenAiProviderDialect,
) -> Result<(), HostError> {
    match dialect {
        OpenAiProviderDialect::OpenAiTools | OpenAiProviderDialect::OmlxCompatible => {
            body.insert(
                "tools".to_owned(),
                Value::Array(vec![json!({
                    "type": "function",
                    "function": {
                        "name": OPENAI_OUTPUT_TOOL,
                        "description": "Emit the final Etas agent output.",
                        "parameters": host_schema_to_json_schema(schema)?,
                    },
                })]),
            );
            body.insert(
                "tool_choice".to_owned(),
                json!({
                    "type": "function",
                    "function": { "name": OPENAI_OUTPUT_TOOL },
                }),
            );
        }
        OpenAiProviderDialect::LegacyFunctions => {
            body.insert(
                "functions".to_owned(),
                Value::Array(vec![json!({
                    "name": OPENAI_OUTPUT_TOOL,
                    "description": "Emit the final Etas agent output.",
                    "parameters": host_schema_to_json_schema(schema)?,
                })]),
            );
            body.insert(
                "function_call".to_owned(),
                json!({ "name": OPENAI_OUTPUT_TOOL }),
            );
        }
    }
    Ok(())
}

fn insert_model_tools(
    body: &mut serde_json::Map<String, Value>,
    request: &ModelRequest,
    dialect: OpenAiProviderDialect,
) -> Result<(), HostError> {
    match dialect {
        OpenAiProviderDialect::OpenAiTools | OpenAiProviderDialect::OmlxCompatible => {
            body.insert("tools".to_owned(), openai_tools(&request.tools)?);
            if let Some(tool_choice) = openai_tool_choice(&request.tool_choice)? {
                body.insert("tool_choice".to_owned(), tool_choice);
            }
        }
        OpenAiProviderDialect::LegacyFunctions => {
            body.insert(
                "functions".to_owned(),
                openai_legacy_functions(&request.tools)?,
            );
            if let Some(function_call) = openai_legacy_function_call(&request.tool_choice)? {
                body.insert("function_call".to_owned(), function_call);
            }
        }
    }
    Ok(())
}

fn dialect_supports_response_format(dialect: OpenAiProviderDialect) -> bool {
    !matches!(dialect, OpenAiProviderDialect::LegacyFunctions)
}

fn openai_tool_choice(choice: &ModelToolChoice) -> Result<Option<Value>, HostError> {
    match choice {
        ModelToolChoice::Auto => Ok(None),
        ModelToolChoice::RequiredAny => Ok(Some(Value::String("required".to_owned()))),
        ModelToolChoice::RequiredTool(name) => {
            if name.is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "required model tool choice cannot be empty",
                ));
            }
            Ok(Some(Value::String("required".to_owned())))
        }
    }
}

fn openai_legacy_function_call(choice: &ModelToolChoice) -> Result<Option<Value>, HostError> {
    match choice {
        ModelToolChoice::Auto | ModelToolChoice::RequiredAny => Ok(None),
        ModelToolChoice::RequiredTool(name) => {
            if name.is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "required model tool choice cannot be empty",
                ));
            }
            Ok(Some(json!({ "name": name })))
        }
    }
}

fn structured_output_instruction(schema: &crate::HostSchema) -> Result<Value, HostError> {
    Ok(json!({
        "role": "system",
        "content": format!(
            "Return only a JSON value matching this JSON Schema. Do not include markdown, prose, or extra keys. Use plain UTF-8 string text; do not emit Unicode escape sequences.\n{}",
            host_schema_to_json_schema(schema)?
        ),
    }))
}

fn encode_openai_message(message: &ModelMessage) -> Result<Value, HostError> {
    match message.role {
        ModelRole::Tool => {
            let Some(tool_call_id) = &message.tool_call_id else {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "OpenAI-compatible model adapter requires a tool_call_id for tool messages",
                ));
            };
            if !message.tool_calls.is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "OpenAI-compatible tool messages cannot carry tool_calls",
                ));
            }
            Ok(json!({
                "role": "tool",
                "tool_call_id": tool_call_id,
                "content": message_text(message)?,
            }))
        }
        role => {
            if message.tool_call_id.is_some() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "only tool messages may carry a tool_call_id",
                ));
            }
            let mut object = serde_json::Map::new();
            object.insert(
                "role".to_owned(),
                Value::String(role_text(role)?.to_owned()),
            );
            if message.tool_calls.is_empty() {
                object.insert("content".to_owned(), Value::String(message_text(message)?));
            } else {
                if !matches!(role, ModelRole::Assistant) {
                    return Err(HostError::new(
                        HostErrorCode::InvalidRequest,
                        "only assistant messages may carry tool_calls",
                    ));
                }
                let content = if message.content.is_empty() {
                    Value::Null
                } else {
                    Value::String(message_text(message)?)
                };
                object.insert("content".to_owned(), content);
                object.insert(
                    "tool_calls".to_owned(),
                    Value::Array(
                        message
                            .tool_calls
                            .iter()
                            .map(encode_openai_tool_call)
                            .collect::<Result<Vec<_>, HostError>>()?,
                    ),
                );
            }
            Ok(Value::Object(object))
        }
    }
}

fn encode_openai_tool_call(call: &ModelToolCall) -> Result<Value, HostError> {
    Ok(json!({
        "id": call.id,
        "type": "function",
        "function": {
            "name": call.tool,
            "arguments": host_value_to_json(&call.args)?.to_string(),
        },
    }))
}

pub(crate) fn message_text(message: &ModelMessage) -> Result<String, HostError> {
    message
        .content
        .iter()
        .map(|content| match content {
            ModelContent::Text(text) => Ok(text.clone()),
            ModelContent::Value(value) => Ok(host_value_to_json(value)?.to_string()),
        })
        .collect::<Result<Vec<String>, HostError>>()
        .map(|parts| parts.join("\n"))
}

pub(crate) fn role_text(role: ModelRole) -> Result<&'static str, HostError> {
    match role {
        ModelRole::System => Ok("system"),
        ModelRole::User => Ok("user"),
        ModelRole::Assistant => Ok("assistant"),
        ModelRole::Tool => Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "tool role cannot be encoded without protocol-specific tool call metadata",
        )),
    }
}

fn metadata_to_json_object(entries: &[(String, HostValue)]) -> Result<Value, HostError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut object = serde_json::Map::new();
    for (name, value) in entries {
        if !seen.insert(name) {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "model metadata contains duplicate key",
            )
            .with_detail("key", name));
        }
        object.insert(name.clone(), host_value_to_json(value)?);
    }
    Ok(Value::Object(object))
}

fn decode_openai_chat_response(
    id: crate::HostRequestId,
    body: &str,
) -> Result<ModelResponse, HostError> {
    let value = serde_json::from_str::<Value>(body).map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "OpenAI-compatible response is not valid JSON",
        )
        .with_detail("error", error.to_string())
    })?;
    let message = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "OpenAI-compatible response is missing choices[0].message",
            )
        })?;
    let mut content = match message.get("content") {
        Some(Value::String(content)) if !content.is_empty() => {
            vec![ModelContent::Text(content.clone())]
        }
        Some(Value::String(_)) | Some(Value::Null) | None => Vec::new(),
        Some(_) => {
            return Err(HostError::new(
                HostErrorCode::InvalidResponse,
                "OpenAI-compatible response message content is not a string",
            ));
        }
    };
    let (structured_content, tool_calls) = decode_openai_tool_calls(message)?;
    content.extend(structured_content);
    if content.is_empty() && tool_calls.is_empty() {
        return Err(HostError::new(
            HostErrorCode::InvalidResponse,
            "OpenAI-compatible response contained no assistant content or tool calls",
        ));
    }
    Ok(ModelResponse {
        id,
        message: ModelMessage {
            role: ModelRole::Assistant,
            content,
            tool_call_id: None,
            tool_calls: tool_calls.clone(),
        },
        tool_calls,
        usage: decode_openai_usage(&value)?,
    })
}

fn decode_openai_tool_calls(
    message: &Value,
) -> Result<(Vec<ModelContent>, Vec<ModelToolCall>), HostError> {
    let Some(tool_calls) = message.get("tool_calls") else {
        if let Some(function_call) = message.get("function_call") {
            return decode_openai_legacy_function_call(function_call);
        }
        return Ok((Vec::new(), Vec::new()));
    };
    let tool_calls = tool_calls.as_array().ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "OpenAI-compatible response tool_calls field is not an array",
        )
    })?;
    let mut structured = Vec::new();
    let mut decoded = Vec::new();
    for call in tool_calls {
        let id = required_string(call, "id")?;
        let function = call.get("function").ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "OpenAI-compatible tool call is missing function object",
            )
        })?;
        let tool = required_string(function, "name")?;
        let arguments = required_string(function, "arguments")?;
        let args_json = serde_json::from_str::<Value>(&arguments).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "OpenAI-compatible tool call arguments are not valid JSON",
            )
            .with_detail("error", error.to_string())
        })?;
        if tool == OPENAI_OUTPUT_TOOL {
            structured.push(ModelContent::Value(host_json_to_value(args_json)?));
        } else {
            decoded.push(ModelToolCall {
                id,
                tool,
                args: host_json_to_value(args_json)?,
            });
        }
    }
    Ok((structured, decoded))
}

fn decode_openai_legacy_function_call(
    function_call: &Value,
) -> Result<(Vec<ModelContent>, Vec<ModelToolCall>), HostError> {
    let tool = required_string(function_call, "name")?;
    let arguments = required_string(function_call, "arguments")?;
    let args_json = serde_json::from_str::<Value>(&arguments).map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "OpenAI-compatible function_call arguments are not valid JSON",
        )
        .with_detail("error", error.to_string())
    })?;
    if tool == OPENAI_OUTPUT_TOOL {
        Ok((
            vec![ModelContent::Value(host_json_to_value(args_json)?)],
            Vec::new(),
        ))
    } else {
        Ok((
            Vec::new(),
            vec![ModelToolCall {
                id: "function-call-1".to_owned(),
                tool,
                args: host_json_to_value(args_json)?,
            }],
        ))
    }
}

fn decode_openai_usage(value: &Value) -> Result<Option<ModelUsage>, HostError> {
    let Some(usage) = value.get("usage") else {
        return Ok(None);
    };
    let input_tokens = required_u64(usage, "prompt_tokens")?;
    let output_tokens = required_u64(usage, "completion_tokens")?;
    Ok(Some(ModelUsage {
        input_tokens,
        output_tokens,
        cost: None,
    }))
}

fn required_string(object: &Value, field: &'static str) -> Result<String, HostError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "provider response field is missing or not a string",
            )
            .with_detail("field", field)
        })
}

fn required_u64(object: &Value, field: &'static str) -> Result<u64, HostError> {
    object.get(field).and_then(Value::as_u64).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "provider usage field is missing or not an unsigned integer",
        )
        .with_detail("field", field)
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        AuthorityContext, ExecutionBudget, HostFieldSchema, HostRequestId, HostSchema, HostValue,
        ModelName, ModelOptions, SandboxPolicy, TraceContext, TraceId,
    };

    use super::*;

    #[test]
    fn openai_request_for_typed_output_uses_forced_output_tool() {
        let request = ModelRequest {
            id: HostRequestId(1),
            provider: None,
            model: ModelName("test-model".to_owned()),
            messages: vec![ModelMessage {
                role: ModelRole::User,
                content: vec![ModelContent::Text("emit output".to_owned())],
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            tools: Vec::new(),
            tool_choice: Default::default(),
            response_schema: Some(HostSchema::Record(vec![HostFieldSchema {
                name: "summary".to_owned(),
                schema: HostSchema::String,
                optional: false,
            }])),
            policy_ref: None,
            options: ModelOptions::default(),
            authority: AuthorityContext {
                grants: Vec::new(),
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        };

        let body =
            encode_openai_chat_request_with_dialect(&request, OpenAiProviderDialect::OpenAiTools)
                .expect("request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(
            body["messages"][0]["content"]
                .as_str()
                .is_some_and(|content| content.contains("JSON Schema"))
        );
        assert_eq!(body["tools"][0]["function"]["name"], OPENAI_OUTPUT_TOOL);
        assert_eq!(body["tool_choice"]["function"]["name"], OPENAI_OUTPUT_TOOL);
        assert_eq!(
            body["tools"][0]["function"]["parameters"]["properties"]["summary"]["type"],
            "string"
        );
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(
            body["response_format"]["json_schema"]["schema"]["properties"]["summary"]["type"],
            "string"
        );
    }

    #[test]
    fn openai_request_for_required_tool_choice_names_tool() {
        let request = ModelRequest {
            id: HostRequestId(1),
            provider: None,
            model: ModelName("test-model".to_owned()),
            messages: vec![ModelMessage {
                role: ModelRole::User,
                content: vec![ModelContent::Text("use the tool".to_owned())],
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            tools: vec![crate::ToolSchema {
                tool: crate::ToolRef::anonymous_test("host.echo"),
                input: HostSchema::Record(vec![HostFieldSchema {
                    name: "message".to_owned(),
                    schema: HostSchema::String,
                    optional: false,
                }]),
                output: Some(HostSchema::String),
            }],
            tool_choice: ModelToolChoice::RequiredTool("host.echo".to_owned()),
            response_schema: Some(HostSchema::Record(vec![HostFieldSchema {
                name: "summary".to_owned(),
                schema: HostSchema::String,
                optional: false,
            }])),
            policy_ref: None,
            options: ModelOptions::default(),
            authority: AuthorityContext {
                grants: Vec::new(),
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        };

        let body =
            encode_openai_chat_request_with_dialect(&request, OpenAiProviderDialect::OpenAiTools)
                .expect("request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["tools"][0]["function"]["name"], "host.echo");
        assert!(body.get("functions").is_none());
        assert_eq!(body["tool_choice"], "required");
        assert!(body.get("function_call").is_none());
        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn legacy_function_dialect_does_not_emit_openai_tools() {
        let request = ModelRequest {
            id: HostRequestId(1),
            provider: None,
            model: ModelName("test-model".to_owned()),
            messages: vec![ModelMessage {
                role: ModelRole::User,
                content: vec![ModelContent::Text("use the tool".to_owned())],
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            tools: vec![crate::ToolSchema {
                tool: crate::ToolRef::anonymous_test("host.echo"),
                input: HostSchema::Record(vec![HostFieldSchema {
                    name: "message".to_owned(),
                    schema: HostSchema::String,
                    optional: false,
                }]),
                output: Some(HostSchema::String),
            }],
            tool_choice: ModelToolChoice::RequiredTool("host.echo".to_owned()),
            response_schema: None,
            policy_ref: None,
            options: ModelOptions::default(),
            authority: AuthorityContext {
                grants: Vec::new(),
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        };

        let body = encode_openai_chat_request_with_dialect(
            &request,
            OpenAiProviderDialect::LegacyFunctions,
        )
        .expect("legacy request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
        assert_eq!(body["functions"][0]["name"], "host.echo");
        assert_eq!(body["function_call"]["name"], "host.echo");
    }

    #[test]
    fn openai_output_tool_call_decodes_to_structured_model_content() {
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "out-1",
                        "type": "function",
                        "function": {
                            "name": OPENAI_OUTPUT_TOOL,
                            "arguments": r#"{"summary":"done"}"#,
                        },
                    }],
                },
            }],
        })
        .to_string();

        let response =
            decode_openai_chat_response(HostRequestId(7), &body).expect("response decodes");
        assert!(response.tool_calls.is_empty());
        assert_eq!(
            response.message.content,
            vec![ModelContent::Value(HostValue::Json(
                crate::HostJsonValue::Object(vec![(
                    "summary".to_owned(),
                    crate::HostJsonValue::String("done".to_owned())
                )])
            ))]
        );
    }

    #[test]
    fn openai_legacy_function_call_decodes_to_tool_call() {
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "function_call": {
                        "name": "host.echo",
                        "arguments": r#"{"message":"hello"}"#,
                    },
                },
            }],
        })
        .to_string();

        let response =
            decode_openai_chat_response(HostRequestId(7), &body).expect("response decodes");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].id, "function-call-1");
        assert_eq!(response.tool_calls[0].tool, "host.echo");
        assert_eq!(response.message.tool_calls, response.tool_calls);
    }
}
