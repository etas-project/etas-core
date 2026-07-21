use std::{future::Future, pin::Pin};

use crate::{
    HostError, HostErrorCode, HttpTransport, PrivateResolutionPolicy, ToolClient, ToolRequest,
    ToolResponse, host_json_to_value, host_value_to_json,
};

#[derive(Clone, Debug, PartialEq)]
pub struct HttpToolRequestEnvelope {
    pub request: ToolRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HttpToolResponseEnvelope {
    pub response: ToolResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpToolProtocolAdapter {
    pub transport: HttpTransport,
    pub path: String,
}

impl HttpToolProtocolAdapter {
    pub fn new(base_url: impl AsRef<str>, path: impl Into<String>) -> Result<Self, HostError> {
        Self::try_new_with_policy(base_url, path, PrivateResolutionPolicy::PublicOnly)
    }

    pub fn try_new_with_policy(
        base_url: impl AsRef<str>,
        path: impl Into<String>,
        policy: PrivateResolutionPolicy,
    ) -> Result<Self, HostError> {
        Ok(Self {
            transport: HttpTransport::try_new(base_url, policy)?,
            path: path.into(),
        })
    }

    pub fn encode_request(&self, request: ToolRequest) -> HttpToolRequestEnvelope {
        HttpToolRequestEnvelope { request }
    }

    pub fn decode_response(response: HttpToolResponseEnvelope) -> Result<ToolResponse, HostError> {
        Ok(response.response)
    }

    pub(crate) async fn invoke_request(
        &self,
        request: ToolRequest,
    ) -> Result<ToolResponse, HostError> {
        let id = request.id;
        let body = host_value_to_json(&request.args)?.to_string();
        let response = self.transport.send_json(&self.path, body).await?;
        if !(200..300).contains(&response.status) {
            return Err(HostError::new(
                HostErrorCode::ToolRejected,
                "HTTP tool endpoint returned an error status",
            )
            .with_detail("status", response.status.to_string()));
        }
        let result_json = serde_json::from_str(&response.body).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "HTTP tool response is not valid JSON",
            )
            .with_detail("error", error.to_string())
        })?;
        Ok(ToolResponse {
            id,
            result: Ok(host_json_to_value(result_json)?),
        })
    }
}

impl ToolClient for HttpToolProtocolAdapter {
    type Error = HostError;
    type InvokeFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ToolResponse, Self::Error>> + Send + 'a>>;

    fn invoke(&self, request: ToolRequest) -> Self::InvokeFuture<'_> {
        Box::pin(async move { self.invoke_request(request).await })
    }
}
