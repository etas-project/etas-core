use std::{future::Future, pin::Pin};

use serde_json::{Value, json};

use crate::{
    AuthConfig, HostError, HostErrorCode, HttpTransport, ModelClient, ModelContent, ModelMessage,
    ModelProviderCapabilities, ModelRequest, ModelResponse, ModelRole, ModelToolCall,
    ModelToolChoice, ModelUsage, PrivateResolutionPolicy, RetryPolicy, host_json_to_value,
    host_value_to_json,
};

use super::{
    openai::message_text,
    tool_schema::{anthropic_tools, host_schema_to_json_schema},
};

const ANTHROPIC_OUTPUT_TOOL: &str = "etas_emit_output";

#[derive(Clone, Debug, PartialEq)]
pub struct AnthropicProviderRequest {
    pub base_url: String,
    pub request: ModelRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnthropicProviderResponse {
    pub response: ModelResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnthropicProtocolAdapter {
    pub base_url: String,
    pub transport: HttpTransport,
}

impl AnthropicProtocolAdapter {
    pub const LOCAL_OMLX_BASE_URL: &'static str = "http://127.0.0.1:8848";

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
        })
    }

    pub fn local_omlx() -> Result<Self, HostError> {
        Self::omlx_compatible(Self::LOCAL_OMLX_BASE_URL)
    }

    pub fn omlx_compatible(base_url: impl Into<String>) -> Result<Self, HostError> {
        Self::try_new_with_policy(base_url, PrivateResolutionPolicy::AllowPrivate)
    }

    pub fn capabilities() -> ModelProviderCapabilities {
        ModelProviderCapabilities {
            supports_forced_tool_output: true,
            supports_json_schema_response_format: false,
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

    pub fn encode_request(&self, request: ModelRequest) -> AnthropicProviderRequest {
        AnthropicProviderRequest {
            base_url: self.base_url.clone(),
            request,
        }
    }

    pub fn decode_response(
        response: AnthropicProviderResponse,
    ) -> Result<ModelResponse, HostError> {
        Ok(response.response)
    }

    async fn complete_request(&self, request: ModelRequest) -> Result<ModelResponse, HostError> {
        let id = request.id;
        let body = encode_anthropic_messages_request(&request)?;
        let response = self.transport.send_json("/v1/messages", body).await?;
        if !(200..300).contains(&response.status) {
            return Err(HostError::new(
                HostErrorCode::ProviderRejected,
                "Anthropic-compatible endpoint returned an error status",
            )
            .with_detail("status", response.status.to_string()));
        }
        decode_anthropic_messages_response(id, &response.body)
    }
}

impl ModelClient for AnthropicProtocolAdapter {
    type Error = HostError;
    type CompleteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ModelResponse, Self::Error>> + Send + 'a>>;

    fn complete(&self, request: ModelRequest) -> Self::CompleteFuture<'_> {
        Box::pin(async move { self.complete_request(request).await })
    }
}

fn encode_anthropic_messages_request(request: &ModelRequest) -> Result<String, HostError> {
    let require_model_tool =
        !request.tools.is_empty() && !matches!(request.tool_choice, ModelToolChoice::Auto);
    if !request.options.metadata.is_empty() {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "Anthropic-compatible model adapter cannot encode arbitrary metadata",
        ));
    }
    let max_tokens = request.options.max_output_tokens.ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "Anthropic-compatible model adapter requires max_output_tokens",
        )
    })?;

    let mut system_parts = Vec::new();
    let mut messages = Vec::new();
    for message in &request.messages {
        match message.role {
            ModelRole::System => system_parts.push(message_text(message)?),
            ModelRole::User | ModelRole::Assistant => {
                messages.push(json!({
                    "role": anthropic_role_text(message.role)?,
                    "content": message_text(message)?,
                }));
            }
            ModelRole::Tool => {
                messages.push(encode_anthropic_tool_result_message(message)?);
            }
        }
    }
    if let Some(schema) = &request.response_schema
        && !require_model_tool
    {
        system_parts.push(format!(
            "Return only a JSON value matching this JSON Schema. Do not include markdown, prose, or extra keys. Use plain UTF-8 string text; do not emit Unicode escape sequences.\n{}",
            host_schema_to_json_schema(schema)?
        ));
    }

    let mut body = serde_json::Map::new();
    body.insert("model".to_owned(), Value::String(request.model.0.clone()));
    body.insert(
        "max_tokens".to_owned(),
        Value::Number(serde_json::Number::from(max_tokens)),
    );
    body.insert("messages".to_owned(), Value::Array(messages));
    if !system_parts.is_empty() {
        body.insert("system".to_owned(), Value::String(system_parts.join("\n")));
    }
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
    if let Some(schema) = &request.response_schema
        && !require_model_tool
    {
        let mut tools = match anthropic_tools(&request.tools)? {
            Value::Array(tools) => tools,
            _ => unreachable!("anthropic_tools always returns an array"),
        };
        tools.push(output_tool(schema)?);
        body.insert("tools".to_owned(), Value::Array(tools));
        if request.tools.is_empty() {
            body.insert(
                "tool_choice".to_owned(),
                json!({
                    "type": "tool",
                    "name": ANTHROPIC_OUTPUT_TOOL,
                }),
            );
        } else if let Some(tool_choice) = anthropic_tool_choice(&request.tool_choice)? {
            body.insert("tool_choice".to_owned(), tool_choice);
        } else {
            body.insert("tool_choice".to_owned(), json!({ "type": "auto" }));
        }
    } else if !request.tools.is_empty() {
        body.insert("tools".to_owned(), anthropic_tools(&request.tools)?);
        if let Some(tool_choice) = anthropic_tool_choice(&request.tool_choice)? {
            body.insert("tool_choice".to_owned(), tool_choice);
        }
    }
    Ok(Value::Object(body).to_string())
}

fn anthropic_tool_choice(choice: &ModelToolChoice) -> Result<Option<Value>, HostError> {
    match choice {
        ModelToolChoice::Auto => Ok(None),
        ModelToolChoice::RequiredAny => Ok(Some(json!({ "type": "any" }))),
        ModelToolChoice::RequiredTool(name) => {
            if name.is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "required model tool choice cannot be empty",
                ));
            }
            Ok(Some(json!({
                "type": "tool",
                "name": name,
            })))
        }
    }
}

fn encode_anthropic_tool_result_message(message: &ModelMessage) -> Result<Value, HostError> {
    let Some(tool_use_id) = &message.tool_call_id else {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "Anthropic-compatible tool result messages require a tool_call_id",
        ));
    };
    if !message.tool_calls.is_empty() {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "Anthropic-compatible tool result messages cannot carry tool_calls",
        ));
    }
    Ok(json!({
        "role": "user",
        "content": [{
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": message
                .content
                .iter()
                .map(|content| match content {
                    ModelContent::Text(text) => Ok(text.clone()),
                    ModelContent::Value(value) => Ok(host_value_to_json(value)?.to_string()),
                })
                .collect::<Result<Vec<_>, HostError>>()?
                .join("\n"),
        }],
    }))
}

fn output_tool(schema: &crate::HostSchema) -> Result<Value, HostError> {
    Ok(json!({
        "name": ANTHROPIC_OUTPUT_TOOL,
        "description": "Emit the final Etas agent output.",
        "input_schema": host_schema_to_json_schema(schema)?,
    }))
}

fn anthropic_role_text(role: ModelRole) -> Result<&'static str, HostError> {
    match role {
        ModelRole::User => Ok("user"),
        ModelRole::Assistant => Ok("assistant"),
        ModelRole::System | ModelRole::Tool => Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "model role is not valid inside Anthropic messages",
        )),
    }
}

fn decode_anthropic_messages_response(
    id: crate::HostRequestId,
    body: &str,
) -> Result<ModelResponse, HostError> {
    let value = serde_json::from_str::<Value>(body).map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidResponse,
            "Anthropic-compatible response is not valid JSON",
        )
        .with_detail("error", error.to_string())
    })?;
    let content_blocks = value
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "Anthropic-compatible response is missing content array",
            )
        })?;
    let mut content = Vec::new();
    let mut tool_calls = Vec::new();
    for block in content_blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                let text = block.get("text").and_then(Value::as_str).ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::InvalidResponse,
                        "Anthropic-compatible text block is missing text",
                    )
                })?;
                if !text.is_empty() {
                    content.push(ModelContent::Text(text.to_owned()));
                }
            }
            Some("tool_use") => {
                let id = required_string(block, "id")?;
                let tool = required_string(block, "name")?;
                let input = block.get("input").cloned().ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::InvalidResponse,
                        "Anthropic-compatible tool_use block is missing input",
                    )
                })?;
                if tool == ANTHROPIC_OUTPUT_TOOL {
                    content.push(ModelContent::Value(host_json_to_value(input)?));
                    continue;
                }
                tool_calls.push(ModelToolCall {
                    id,
                    tool,
                    args: host_json_to_value(input)?,
                });
            }
            Some(kind) => {
                return Err(HostError::new(
                    HostErrorCode::InvalidResponse,
                    "Anthropic-compatible response contains an unsupported content block",
                )
                .with_detail("type", kind));
            }
            None => {
                return Err(HostError::new(
                    HostErrorCode::InvalidResponse,
                    "Anthropic-compatible content block is missing type",
                ));
            }
        }
    }
    if content.is_empty() && tool_calls.is_empty() {
        return Err(HostError::new(
            HostErrorCode::InvalidResponse,
            "Anthropic-compatible response contained no assistant content or tool calls",
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
        usage: decode_anthropic_usage(&value)?,
    })
}

fn decode_anthropic_usage(value: &Value) -> Result<Option<ModelUsage>, HostError> {
    let Some(usage) = value.get("usage") else {
        return Ok(None);
    };
    Ok(Some(ModelUsage {
        input_tokens: required_u64(usage, "input_tokens")?,
        output_tokens: required_u64(usage, "output_tokens")?,
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
        AuthorityContext, Budget, HostFieldSchema, HostRequestId, HostSchema, HostValue, ModelName,
        ModelOptions, SandboxPolicy, ToolSchema, TraceContext, TraceId,
    };

    use super::*;

    #[test]
    fn anthropic_request_for_typed_output_uses_forced_output_tool() {
        let request = typed_request();
        let body = encode_anthropic_messages_request(&request).expect("request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");

        assert_eq!(body["tools"][0]["name"], ANTHROPIC_OUTPUT_TOOL);
        assert_eq!(body["tool_choice"]["name"], ANTHROPIC_OUTPUT_TOOL);
        assert_eq!(
            body["tools"][0]["input_schema"]["properties"]["summary"]["type"],
            "string"
        );
    }

    #[test]
    fn anthropic_output_tool_response_decodes_to_structured_model_content() {
        let body = json!({
            "content": [{
                "type": "tool_use",
                "id": "out-1",
                "name": ANTHROPIC_OUTPUT_TOOL,
                "input": { "summary": "done" },
            }],
        })
        .to_string();

        let response =
            decode_anthropic_messages_response(HostRequestId(7), &body).expect("response decodes");
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
    fn anthropic_adapter_encodes_tool_result_messages() {
        let mut request = typed_request();
        request.response_schema = None;
        request.messages = vec![ModelMessage {
            role: ModelRole::Tool,
            content: vec![ModelContent::Value(HostValue::String("ok".to_owned()))],
            tool_call_id: Some("toolu_1".to_owned()),
            tool_calls: Vec::new(),
        }];

        let body = encode_anthropic_messages_request(&request).expect("request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"][0]["type"], "tool_result");
        assert_eq!(body["messages"][0]["content"][0]["tool_use_id"], "toolu_1");
    }

    #[test]
    fn anthropic_adapter_combines_typed_output_and_tools() {
        let mut request = typed_request();
        request.tools = vec![ToolSchema {
            tool: crate::ToolRef::anonymous_test("host.echo"),
            input: HostSchema::Record(vec![HostFieldSchema {
                name: "message".to_owned(),
                schema: HostSchema::String,
                optional: false,
            }]),
            output: Some(HostSchema::String),
        }];

        let body = encode_anthropic_messages_request(&request).expect("request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");
        let tool_names = body["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        assert!(tool_names.contains(&"host.echo"));
        assert!(tool_names.contains(&ANTHROPIC_OUTPUT_TOOL));
        assert_eq!(body["tool_choice"]["type"], "auto");
    }

    #[test]
    fn anthropic_request_for_required_tool_choice_names_tool() {
        let mut request = typed_request();
        request.tools = vec![ToolSchema {
            tool: crate::ToolRef::anonymous_test("host.echo"),
            input: HostSchema::Record(vec![HostFieldSchema {
                name: "message".to_owned(),
                schema: HostSchema::String,
                optional: false,
            }]),
            output: Some(HostSchema::String),
        }];
        request.tool_choice = ModelToolChoice::RequiredTool("host.echo".to_owned());

        let body = encode_anthropic_messages_request(&request).expect("request should encode");
        let body: Value = serde_json::from_str(&body).expect("encoded request should be JSON");
        assert_eq!(body["tools"][0]["name"], "host.echo");
        assert_eq!(body["tools"].as_array().expect("tools array").len(), 1);
        assert!(body.get("system").is_none());
        assert_eq!(body["tool_choice"]["type"], "tool");
        assert_eq!(body["tool_choice"]["name"], "host.echo");
    }

    fn typed_request() -> ModelRequest {
        ModelRequest {
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
            options: ModelOptions {
                temperature: None,
                max_output_tokens: Some(64),
                metadata: Vec::new(),
            },
            authority: AuthorityContext {
                grants: Vec::new(),
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(1)),
            budget: Budget::default(),
        }
    }
}
