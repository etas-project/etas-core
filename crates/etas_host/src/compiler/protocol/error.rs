use std::sync::Arc;

use etas_code::{diagnostic::DiagnosticRecord, report::CheckReport};

/// Authority, cancellation and budget exhaustion remain HostError, not a source
/// rejection or a compiler-service diagnostic.
#[derive(Clone, Debug)]
pub enum CompilerFailure {
    Rejected(Arc<CheckReport>),
    Service { diagnostics: Vec<DiagnosticRecord> },
}
