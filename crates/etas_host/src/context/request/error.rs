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

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)?;
        for detail in &self.details {
            write!(formatter, " ({}={})", detail.key, detail.value)?;
        }
        Ok(())
    }
}

impl std::error::Error for HostError {}

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
    TimedOut,
    Cancelled,
    Closed,
    Interrupted,
}

impl HostErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProviderRejected => "ProviderRejected",
            Self::ProviderUnavailable => "ProviderUnavailable",
            Self::ToolRejected => "ToolRejected",
            Self::ToolUnavailable => "ToolUnavailable",
            Self::InvalidRequest => "InvalidRequest",
            Self::InvalidResponse => "InvalidResponse",
            Self::SchemaMismatch => "SchemaMismatch",
            Self::BudgetExceeded => "BudgetExceeded",
            Self::AuthorityDenied => "AuthorityDenied",
            Self::TimedOut => "TimedOut",
            Self::Cancelled => "Cancelled",
            Self::Closed => "Closed",
            Self::Interrupted => "Interrupted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostErrorDetail {
    pub key: String,
    pub value: String,
}
use std::fmt;
