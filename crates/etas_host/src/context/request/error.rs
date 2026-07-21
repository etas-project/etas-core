#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    pub code: HostErrorCode,
    pub message: String,
    pub details: Vec<HostErrorDetail>,
}

impl HostError {
    pub fn new(code: HostErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.push(HostErrorDetail {
            key: key.into(),
            value: value.into(),
        });
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostErrorCode {
    ProviderRejected,
    ProviderUnavailable,
    ToolRejected,
    ToolUnavailable,
    InvalidRequest,
    InvalidResponse,
    SchemaMismatch,
    BudgetExceeded,
    AuthorityDenied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostErrorDetail {
    pub key: String,
    pub value: String,
}
