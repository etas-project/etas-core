pub mod anthropic;
pub mod client;
pub mod openai;
mod payload_error;
pub mod protocol;
pub(crate) mod tool_schema;
mod usage;

pub use anthropic::{
    AnthropicProtocolAdapter, AnthropicProviderRequest, AnthropicProviderResponse,
};
pub use client::ModelClient;
pub use openai::{
    OmlxRequestOptions, OpenAiProtocolAdapter, OpenAiProviderRequest, OpenAiProviderResponse,
    OpenAiTypedOutputEncoding,
};
pub use protocol::{
    ModelAction, ModelAttemptPolicy, ModelContent, ModelCostUsage, ModelMessage, ModelName,
    ModelOptions, ModelOutput, ModelProviderCapabilities, ModelProviderId, ModelRequest,
    ModelResponse, ModelRole, ModelTermination, ModelToolCall, ModelToolChoice, ModelUsage,
};
