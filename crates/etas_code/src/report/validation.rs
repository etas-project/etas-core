use crate::{
    descriptor::{
        DescriptorError, DescriptorGraph, DescriptorLimits, EffectRowInfoRef, EffectSummaryInfo,
        TraceInfoRef,
    },
    diagnostic::{DescriptorDetail, DiagnosticLocation, DiagnosticRecord, DiagnosticSeverity},
};

use super::{CheckReport, CheckStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportValidationError {
    Descriptor(DescriptorError),
    MissingEffectRow(EffectRowInfoRef),
    MissingTrace(TraceInfoRef),
    MissingType(crate::descriptor::TypeInfoRef),
    InvalidLocation,
    InvalidStatus,
    IncompleteAcceptedAnalysis,
    ForeignEnvironment,
    ForeignDefinition,
    InvalidSpec,
}

impl std::fmt::Display for ReportValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid compiler report: {self:?}")
    }
}
impl std::error::Error for ReportValidationError {}
impl From<DescriptorError> for ReportValidationError {
    fn from(error: DescriptorError) -> Self {
        Self::Descriptor(error)
    }
}

impl CheckReport {
    /// This validates descriptive data only. It never establishes contract
    /// compatibility or turns a user-supplied report into execution evidence.
    pub fn validate(&self, limits: DescriptorLimits) -> Result<(), ReportValidationError> {
        self.descriptors.validate(limits)?;
        let inputs = self.revision.inputs();
        if self.dependencies.environment != inputs.environment {
            return Err(ReportValidationError::ForeignEnvironment);
        }
        let owner = inputs.definition.as_declaration().or_else(|| {
            inputs
                .definition
                .as_method()
                .map(|method| method.implementation().subject())
        });
        if owner != Some(&self.dependencies.lexical_owner) {
            return Err(ReportValidationError::ForeignDefinition);
        }
        validate_effects(&self.descriptors, &self.effects)?;
        validate_trace(&self.descriptors, self.trace)?;
        if self.status != CheckStatus::Rejected {
            if self.effects.escaping.is_none()
                || self.effects.requested.is_none()
                || self.effects.default_handled.is_none()
                || self.determinism.is_none()
                || self.trace.is_none()
            {
                return Err(ReportValidationError::IncompleteAcceptedAnalysis);
            }
            if self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
                || (self.status == CheckStatus::Accepted && !self.runtime_checks.is_empty())
                || (self.status == CheckStatus::Guarded && self.runtime_checks.is_empty())
            {
                return Err(ReportValidationError::InvalidStatus);
            }
        }
        for diagnostic in &self.diagnostics {
            validate_diagnostic(diagnostic, limits)?;
        }
        for check in &self.runtime_checks {
            validate_location(check.location.as_ref())?;
        }
        Ok(())
    }
}

impl DiagnosticRecord {
    pub fn validate(&self, limits: DescriptorLimits) -> Result<(), ReportValidationError> {
        validate_diagnostic(self, limits)
    }
}

fn validate_diagnostic(
    value: &DiagnosticRecord,
    limits: DescriptorLimits,
) -> Result<(), ReportValidationError> {
    validate_location(value.location.as_ref())?;
    if let Some(details) = &value.details {
        for detail in details.expected.iter().chain(&details.actual) {
            match detail {
                DescriptorDetail::Type { graph, ty } => {
                    graph.validate(limits)?;
                    if graph.type_info(*ty).is_none() {
                        return Err(ReportValidationError::MissingType(*ty));
                    }
                }
                DescriptorDetail::EffectRow { graph, row } => {
                    graph.validate(limits)?;
                    validate_row(graph, *row)?;
                }
                DescriptorDetail::Spec { graph, spec } => {
                    graph.validate(limits)?;
                    if spec.definition.kind() != crate::DefinitionKind::Spec {
                        return Err(ReportValidationError::InvalidSpec);
                    }
                    for ty in &spec.arguments {
                        if graph.type_info(*ty).is_none() {
                            return Err(ReportValidationError::MissingType(*ty));
                        }
                    }
                }
                DescriptorDetail::Trace { graph, trace } => {
                    graph.validate(limits)?;
                    validate_trace(graph, Some(*trace))?;
                }
            }
        }
        for related in &details.related {
            validate_location(Some(&related.location))?;
        }
        for step in &details.analysis_path {
            validate_location(step.location.as_ref())?;
        }
    }
    Ok(())
}

fn validate_effects(
    graph: &DescriptorGraph,
    effects: &EffectSummaryInfo,
) -> Result<(), ReportValidationError> {
    for row in [
        effects.declared_bound,
        effects.escaping,
        effects.requested,
        effects.default_handled,
    ]
    .into_iter()
    .flatten()
    {
        validate_row(graph, row)?;
    }
    Ok(())
}
fn validate_row(
    graph: &DescriptorGraph,
    row: EffectRowInfoRef,
) -> Result<(), ReportValidationError> {
    if graph.effect_row(row).is_none() {
        return Err(ReportValidationError::MissingEffectRow(row));
    }
    Ok(())
}
fn validate_trace(
    graph: &DescriptorGraph,
    trace: Option<TraceInfoRef>,
) -> Result<(), ReportValidationError> {
    if let Some(trace) = trace
        && graph.trace(trace).is_none()
    {
        return Err(ReportValidationError::MissingTrace(trace));
    }
    Ok(())
}
fn validate_location(location: Option<&DiagnosticLocation>) -> Result<(), ReportValidationError> {
    if location.is_some_and(|location| location.byte_start > location.byte_end) {
        return Err(ReportValidationError::InvalidLocation);
    }
    Ok(())
}
