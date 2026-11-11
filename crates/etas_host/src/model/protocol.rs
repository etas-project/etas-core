use crate::{
    AuthorityContext, ExecutionBudget, HostRequestId, HostSchema, HostValue, ToolSchema,
    TraceContext,
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
    pub action: ModelAction,
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
    pub budget: ExecutionBudget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelAction {
    Infer,
    Step,
}

impl ModelAction {
    pub fn qualified_name(self) -> &'static str {
        match self {
            Self::Infer => "Agentic.infer",
            Self::Step => "Agentic.step",
        }
    }
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
    pub provider_data: Vec<(String, HostValue)>,
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
    /// Usage is retained even when generation did not produce a complete output.
    pub usage: Option<ModelUsage>,
    pub result: Result<ModelOutput, crate::HostError>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelOutput {
    pub message: ModelMessage,
    pub termination: ModelTermination,
    pub provider_request_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelTermination {
    Complete,
    Truncated,
    Refused(String),
    InvalidToolCall(String),
    InvalidOutput(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelToolCall {
    pub id: String,
    pub tool: String,
    pub args: HostValue,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelOptions {
    pub attempt_policy: ModelAttemptPolicy,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u64>,
    pub metadata: Vec<(String, HostValue)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelAttemptPolicy {
    #[default]
    Configured,
    Single,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost: Option<ModelCostUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCostUsage {
    pub micros: u128,
    pub currency: String,
}
