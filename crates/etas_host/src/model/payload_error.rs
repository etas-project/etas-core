use crate::{HostError, ModelTermination};

#[derive(Debug)]
pub(super) enum ModelPayloadError {
    ToolCall(String),
    Output(String),
}

impl ModelPayloadError {
    pub(super) fn for_tool(tool: &str, output_tool: &str, error: HostError) -> Self {
        if tool == output_tool {
            Self::Output(error.message)
        } else {
            error.into()
        }
    }
}

impl From<HostError> for ModelPayloadError {
    fn from(error: HostError) -> Self {
        Self::ToolCall(error.message)
    }
}

impl From<ModelPayloadError> for ModelTermination {
    fn from(error: ModelPayloadError) -> Self {
        match error {
            ModelPayloadError::ToolCall(message) => Self::InvalidToolCall(message),
            ModelPayloadError::Output(message) => Self::InvalidOutput(message),
        }
    }
}
