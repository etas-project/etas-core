use std::sync::Arc;

use crate::{
    CallableId, CodeFingerprint, DefinitionKey,
    descriptor::{DescriptorGraph, EffectRowInfoRef, SpecInfo, TraceInfoRef, TypeInfoRef},
};

mod serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct DiagnosticLocation {
    pub source: CodeFingerprint,
    /// Source revision, including unbound parsed syntax not yet tied to an agent.
    pub revision: CodeFingerprint,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum DescriptorDetail {
    Type {
        graph: DescriptorGraph,
        ty: TypeInfoRef,
    },
    EffectRow {
        graph: DescriptorGraph,
        row: EffectRowInfoRef,
    },
    Spec {
        graph: DescriptorGraph,
        spec: SpecInfo,
    },
    Trace {
        graph: DescriptorGraph,
        trace: TraceInfoRef,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct ViolatedConstraint {
    pub kind: String,
    pub owner: Option<DefinitionKey>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct RelatedLocation {
    pub location: DiagnosticLocation,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct AnalysisStep {
    pub callable: CallableId,
    pub location: Option<DiagnosticLocation>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct DiagnosticDetails {
    pub expected: Option<DescriptorDetail>,
    pub actual: Option<DescriptorDetail>,
    pub constraint: Option<ViolatedConstraint>,
    pub related: Vec<RelatedLocation>,
    pub analysis_path: Vec<AnalysisStep>,
    pub trace_witness: Option<crate::contract::TraceInclusionWitness>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct DiagnosticRecord {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub location: Option<DiagnosticLocation>,
    pub details: Option<Arc<DiagnosticDetails>>,
}
