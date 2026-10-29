use crate::{CodeFingerprint, DefinitionKey, RevisionInfo, descriptor::DeterminismInfo};

pub const CHANGE_REPORT_SCHEMA: &str = "etas.code.change-report.v2";

/// Canonical observations of one immutable definition. Missing current analysis
/// remains unavailable; it is never replaced with baseline or empty behavior.
pub struct DefinitionDiffSnapshot<'a> {
    pub identity: &'a DefinitionKey,
    pub revision: &'a RevisionInfo,
    pub signature: Option<CodeFingerprint>,
    pub body: Option<CodeFingerprint>,
    pub declared_bound: Option<CodeFingerprint>,
    pub escaping: Option<CodeFingerprint>,
    pub requested: Option<CodeFingerprint>,
    pub default_handled: Option<CodeFingerprint>,
    pub determinism: Option<DeterminismInfo>,
    pub trace: Option<CodeFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ChangeReport {
    pub identity: DefinitionKey,
    pub old_revision: RevisionInfo,
    pub new_revision: RevisionInfo,
    pub old_body_available: bool,
    pub new_body_available: bool,
    pub body_changed: Option<bool>,
    pub signature_changed: bool,
    pub dependencies_changed: bool,
    pub declared_bound_changed: bool,
    pub escaping_changed: Option<bool>,
    pub requested_changed: Option<bool>,
    pub default_handled_changed: Option<bool>,
    pub determinism_changed: Option<bool>,
    pub trace_changed: Option<bool>,
}

impl ChangeReport {
    pub fn compare(
        old: DefinitionDiffSnapshot<'_>,
        new: DefinitionDiffSnapshot<'_>,
    ) -> Result<Self, super::DefinitionOperationError> {
        if old.identity != new.identity {
            return Err(super::DefinitionOperationError::IncompatibleDefinitions);
        }
        let owns_entry = |snapshot: &DefinitionDiffSnapshot<'_>| match snapshot
            .revision
            .inputs()
            .definition
            .identity_ref()
        {
            crate::identity::CallableIdentityRef::Declaration(identity) => {
                identity == snapshot.identity
            }
            crate::identity::CallableIdentityRef::Method(method) => {
                snapshot.identity.kind() == crate::DefinitionKind::Agent
                    && method.implementation().subject() == snapshot.identity
            }
        };
        if !owns_entry(&old)
            || !owns_entry(&new)
            || old.revision.inputs().definition != new.revision.inputs().definition
        {
            return Err(super::DefinitionOperationError::IncompatibleDefinitions);
        }
        Ok(Self {
            identity: old.identity.clone(),
            old_revision: old.revision.clone(),
            new_revision: new.revision.clone(),
            old_body_available: old.body.is_some(),
            new_body_available: new.body.is_some(),
            body_changed: changed(old.body, new.body),
            signature_changed: old.signature != new.signature,
            dependencies_changed: old.revision.inputs().environment
                != new.revision.inputs().environment,
            declared_bound_changed: old.declared_bound != new.declared_bound,
            escaping_changed: changed(old.escaping, new.escaping),
            requested_changed: changed(old.requested, new.requested),
            default_handled_changed: changed(old.default_handled, new.default_handled),
            determinism_changed: changed(old.determinism, new.determinism),
            trace_changed: changed(old.trace, new.trace),
        })
    }
}

fn changed<T: PartialEq>(old: Option<T>, new: Option<T>) -> Option<bool> {
    old.zip(new).map(|(old, new)| old != new)
}

#[cfg(test)]
mod tests;
