use std::{future::Future, pin::Pin};

use crate::{
    HostError, HttpToolProtocolAdapter, PrivateResolutionPolicy, ToolClient, ToolRequest,
    ToolResponse,
};

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolRequestEnvelope {
    pub request: ToolRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolResponseEnvelope {
    pub response: ToolResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpToolProtocolAdapter {
    pub http: HttpToolProtocolAdapter,
}

impl McpToolProtocolAdapter {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, HostError> {
        Self::try_new_with_policy(base_url, PrivateResolutionPolicy::PublicOnly)
    }

    pub fn try_new_with_policy(
        base_url: impl AsRef<str>,
        policy: PrivateResolutionPolicy,
    ) -> Result<Self, HostError> {
        Ok(Self {
            http: HttpToolProtocolAdapter::try_new_with_policy(base_url, "/mcp/call", policy)?,
        })
    }

    pub fn encode_request(&self, request: ToolRequest) -> McpToolRequestEnvelope {
        McpToolRequestEnvelope { request }
    }

    pub fn decode_response(response: McpToolResponseEnvelope) -> Result<ToolResponse, HostError> {
        Ok(response.response)
    }
}

impl ToolClient for McpToolProtocolAdapter {
    type Error = HostError;
    type InvokeFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ToolResponse, Self::Error>> + Send + 'a>>;

    fn invoke(&self, request: ToolRequest) -> Self::InvokeFuture<'_> {
        Box::pin(async move { self.http.invoke_request(request).await })
    }
}
