use crate::{
    AuthorityContext, Budget, HostRequestId, HostSchema, HostValue, ToolSchema, TraceContext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelProviderId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelName(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelProviderCapabilities {
    pub supports_forced_tool_output: bool,
    pub supports_json_schema_response_format: bool,
    pub supports_plain_json_text_instruction: bool,
    pub supports_tool_call_loop: bool,
    pub supports_required_tool_choice: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelRequest {
    pub id: HostRequestId,
    pub provider: Option<ModelProviderId>,
    pub model: ModelName,
    pub messages: Vec<ModelMessage>,
    pub tools: Vec<ToolSchema>,
    pub tool_choice: ModelToolChoice,
    pub response_schema: Option<HostSchema>,
    pub policy_ref: Option<HostValue>,
    pub options: ModelOptions,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ModelToolChoice {
    #[default]
    Auto,
    RequiredAny,
    RequiredTool(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelMessage {
    pub role: ModelRole,
    pub content: Vec<ModelContent>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Vec<ModelToolCall>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelContent {
    Text(String),
    Value(HostValue),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelResponse {
    pub id: HostRequestId,
    pub message: ModelMessage,
    pub tool_calls: Vec<ModelToolCall>,
    pub usage: Option<ModelUsage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelToolCall {
    pub id: String,
    pub tool: String,
    pub args: HostValue,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelOptions {
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u64>,
    pub metadata: Vec<(String, HostValue)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}
