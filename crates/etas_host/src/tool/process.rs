use std::{future::Future, pin::Pin};

use tokio::process::Command;

use crate::{
    HostError, HostErrorCode, SandboxBroker, ToolClient, ToolRequest, ToolResponse,
    host_json_to_value, host_value_to_json_string,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessToolRequestEnvelope {
    pub request: ToolRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessToolResponseEnvelope {
    pub response: ToolResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessToolProtocolAdapter {
    pub program: String,
    pub args: Vec<String>,
}

impl ProcessToolProtocolAdapter {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }

    pub fn encode_request(&self, request: ToolRequest) -> ProcessToolRequestEnvelope {
        ProcessToolRequestEnvelope { request }
    }

    pub fn decode_response(
        response: ProcessToolResponseEnvelope,
    ) -> Result<ToolResponse, HostError> {
        Ok(response.response)
    }

    async fn invoke_process(
        &self,
        request: ToolRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<ToolResponse, HostError> {
        SandboxBroker::new(request.authority.sandbox.clone()).check_command(&self.program)?;
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        let body = host_value_to_json_string(&request.args)?;
        let output = crate::command::execute_tool_process(
            command,
            body.into_bytes(),
            &request.budget,
            operation,
            self.program.clone(),
        )
        .await?;
        if output.exit_code != 0 {
            return Err(HostError::new(
                HostErrorCode::ToolRejected,
                "process tool exited with failure",
            )
            .with_detail("status", output.exit_code.to_string()));
        }
        let body = String::from_utf8(output.stdout).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "process tool output is not valid UTF-8",
            )
            .with_detail("error", error.to_string())
        })?;
        let result_json = serde_json::from_str(&body).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "process tool output is not valid JSON",
            )
            .with_detail("error", error.to_string())
        })?;
        Ok(ToolResponse {
            id: request.id,
            result: Ok(host_json_to_value(result_json)?),
        })
    }

    pub async fn invoke_scoped(
        &self,
        request: ToolRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<ToolResponse, HostError> {
        let client = self.clone();
        operation
            .supervise(move |context| async move {
                client.invoke_process(request, Some(&context)).await
            })
            .await
    }
}

impl ToolClient for ProcessToolProtocolAdapter {
    type Error = HostError;
    type InvokeFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ToolResponse, Self::Error>> + Send + 'a>>;

    fn invoke(&self, request: ToolRequest) -> Self::InvokeFuture<'_> {
        Box::pin(async move { self.invoke_process(request, None).await })
    }
}
