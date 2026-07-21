use std::{future::Future, pin::Pin, process::Stdio};

use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
    HostError, HostErrorCode, SandboxBroker, ToolClient, ToolRequest, ToolResponse,
    host_json_to_value, host_value_to_json,
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

    async fn invoke_process(&self, request: ToolRequest) -> Result<ToolResponse, HostError> {
        SandboxBroker::new(request.authority.sandbox.clone()).check_command(&self.program)?;
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| {
                HostError::new(
                    HostErrorCode::ToolUnavailable,
                    "failed to spawn process tool",
                )
                .with_detail("error", error.to_string())
            })?;
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            HostError::new(
                HostErrorCode::ToolUnavailable,
                "process tool stdin pipe was not available",
            )
        })?;
        let body = host_value_to_json(&request.args)?.to_string();
        stdin.write_all(body.as_bytes()).await.map_err(|error| {
            HostError::new(
                HostErrorCode::ToolRejected,
                "failed to write process tool input",
            )
            .with_detail("error", error.to_string())
        })?;
        let output = child.wait_with_output().await.map_err(|error| {
            HostError::new(
                HostErrorCode::ToolRejected,
                "failed to wait for process tool output",
            )
            .with_detail("error", error.to_string())
        })?;
        if !output.status.success() {
            return Err(HostError::new(
                HostErrorCode::ToolRejected,
                "process tool exited with failure",
            )
            .with_detail("status", output.status.to_string()));
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
}

impl ToolClient for ProcessToolProtocolAdapter {
    type Error = HostError;
    type InvokeFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ToolResponse, Self::Error>> + Send + 'a>>;

    fn invoke(&self, request: ToolRequest) -> Self::InvokeFuture<'_> {
        Box::pin(async move { self.invoke_process(request).await })
    }
}
