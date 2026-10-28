use std::fmt;

use super::SyntaxBlockRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum SyntaxResource {
    SourceBytes,
    Tokens,
    AstNodes,
    Nesting,
    RetainedObjects,
    RetainedBytes,
    FormattedBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum SyntaxOperationError {
    Interrupted(etas_utils::PipelineInterruption),
    ResourceLimit {
        resource: SyntaxResource,
        limit: u64,
    },
    ForeignSyntaxObject,
    BackendUnavailable {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ParseError {
    pub diagnostics: Vec<crate::diagnostic::DiagnosticRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum ParseBodyFailure {
    InvalidSyntax(ParseError),
    Operation(SyntaxOperationError),
}

/// In-memory syntax operations, separate from effectful checking/compilation.
pub trait CodeSyntaxOperations: Send + Sync {
    fn parse_body(&self, source: &str) -> Result<SyntaxBlockRef, ParseBodyFailure>;
    fn parse_body_controlled(
        &self,
        source: &str,
        control: &dyn etas_utils::PipelineExecutionControl,
    ) -> Result<SyntaxBlockRef, ParseBodyFailure>;
    fn format(&self, syntax: &SyntaxBlockRef) -> Result<String, SyntaxOperationError>;
    fn format_bounded(
        &self,
        syntax: &SyntaxBlockRef,
        max_output_bytes: usize,
    ) -> Result<String, SyntaxOperationError>;
    fn format_bounded_controlled(
        &self,
        syntax: &SyntaxBlockRef,
        max_output_bytes: usize,
        control: &dyn etas_utils::PipelineExecutionControl,
    ) -> Result<String, SyntaxOperationError>;
}

impl fmt::Display for SyntaxOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interrupted(interruption) => {
                write!(f, "syntax operation interrupted: {}", interruption.reason)
            }
            Self::ResourceLimit { resource, limit } => {
                write!(f, "syntax operation exceeded {resource:?} limit {limit}")
            }
            Self::ForeignSyntaxObject => f.write_str("syntax object belongs to another backend"),
            Self::BackendUnavailable { reason } => {
                write!(f, "syntax backend unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for SyntaxOperationError {}

impl fmt::Display for ParseBodyFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntax(_) => f.write_str("body source has syntax errors"),
            Self::Operation(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ParseBodyFailure {}
