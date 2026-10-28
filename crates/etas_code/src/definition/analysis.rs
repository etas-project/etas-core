use std::{fmt, sync::Arc};

use crate::{CodeFingerprint, RevisionKey, identity::canonical::CanonicalEncoder};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, etas_core::RetainedSize)]
pub struct AnalysisFingerprint {
    revision: RevisionKey,
    analysis_configuration: CodeFingerprint,
}

impl AnalysisFingerprint {
    pub fn new(revision: RevisionKey, analysis_configuration: CodeFingerprint) -> Self {
        Self {
            revision,
            analysis_configuration,
        }
    }

    pub fn revision(self) -> RevisionKey {
        self.revision
    }

    pub fn fingerprint(self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.analysis.v1");
        encoder.fingerprint(self.revision.fingerprint());
        encoder.fingerprint(self.analysis_configuration);
        encoder.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum AnalysisUnavailableReason {
    NotAnalyzed,
    Rejected,
    MissingDefaultEntry,
    MissingDependency,
    EditedBody,
    InputMismatch,
}

/// Internal state; source-facing fields project only matching Checked analysis.
/// S is an immutable backend summary bundle, never an executable or permit.
#[derive(Clone, Debug, etas_core::RetainedSize)]
pub enum DefinitionAnalysisState<S> {
    Checked {
        input: AnalysisFingerprint,
        summaries: Arc<S>,
    },
    BaselineOnly {
        base: AnalysisFingerprint,
        summaries: Arc<S>,
    },
    Unavailable {
        reason: AnalysisUnavailableReason,
    },
}

impl<S> DefinitionAnalysisState<S> {
    pub fn current(&self, expected: AnalysisFingerprint) -> Result<&S, AnalysisUnavailableReason> {
        match self {
            Self::Checked { input, summaries } if *input == expected => Ok(summaries),
            Self::Checked { .. } => Err(AnalysisUnavailableReason::InputMismatch),
            Self::BaselineOnly { .. } => Err(AnalysisUnavailableReason::EditedBody),
            Self::Unavailable { reason } => Err(*reason),
        }
    }

    /// Does not mutate the original and never labels baseline summaries as current.
    pub fn after_body_edit(&self) -> Self {
        match self {
            Self::Checked { input, summaries } => Self::BaselineOnly {
                base: *input,
                summaries: Arc::clone(summaries),
            },
            Self::BaselineOnly { base, summaries } => Self::BaselineOnly {
                base: *base,
                summaries: Arc::clone(summaries),
            },
            Self::Unavailable { reason } => Self::Unavailable { reason: *reason },
        }
    }
}

impl fmt::Display for AnalysisUnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAnalyzed => "definition has not been analyzed",
            Self::Rejected => "definition analysis was rejected",
            Self::MissingDefaultEntry => "agent has no default-entry contract",
            Self::MissingDependency => "definition dependency is unavailable",
            Self::EditedBody => "baseline analysis does not describe the edited body",
            Self::InputMismatch => "analysis inputs do not match the requested snapshot",
        })
    }
}

impl std::error::Error for AnalysisUnavailableReason {}
