use crate::{
    CodeFingerprint, DefinitionKey, PackageKey, RevisionInfo,
    descriptor::{DescriptorGraph, DeterminismInfo, EffectSummaryInfo, TraceInfoRef},
    diagnostic::{DiagnosticLocation, DiagnosticRecord},
};

mod validation;
pub use validation::ReportValidationError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum CheckStatus {
    Rejected,
    Accepted,
    Guarded,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::Accepted => "accepted",
            Self::Guarded => "guarded",
        }
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct ModuleOriginInfo {
    pub module: Vec<String>,
    pub package: PackageKey,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionBindingInfo {
    pub path: Vec<String>,
    pub definition: DefinitionKey,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DependencyArtifactInfo {
    pub import_root: String,
    pub fingerprint: String,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DependencyInfo {
    pub environment: CodeFingerprint,
    pub lexical_owner: DefinitionKey,
    pub standard_package: PackageKey,
    pub modules: Vec<ModuleOriginInfo>,
    pub bindings: Vec<DefinitionBindingInfo>,
    pub artifacts: Vec<DependencyArtifactInfo>,
}

/// Descriptive obligations only. Copying a report cannot install a guard or
/// authorize execution; publication must consume native checked evidence.
#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct RuntimeCheckInfo {
    pub kind: String,
    pub location: Option<DiagnosticLocation>,
    pub spec: Option<DefinitionKey>,
}

/// An immutable result for one exact input snapshot, never a compile permit.
#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct CheckReport {
    pub status: CheckStatus,
    pub diagnostics: Vec<DiagnosticRecord>,
    pub revision: RevisionInfo,
    pub dependencies: DependencyInfo,
    pub snapshot_fingerprint: CodeFingerprint,
    pub descriptors: DescriptorGraph,
    pub effects: EffectSummaryInfo,
    pub determinism: Option<DeterminismInfo>,
    pub trace: Option<TraceInfoRef>,
    pub runtime_checks: Vec<RuntimeCheckInfo>,
}
