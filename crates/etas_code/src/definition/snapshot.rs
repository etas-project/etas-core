use std::{fmt, sync::Arc};

use crate::{
    CallableId, CodeFingerprint, DefinitionKey, DefinitionKind, RevisionInfo, RevisionInputs,
    descriptor::{DescriptorGraph, SignatureInfo, TypeInfo, TypeInfoRef},
    syntax::SyntaxBlockRef,
};

use super::{AnalysisFingerprint, AnalysisUnavailableReason, DefinitionAnalysisState};

/// Supplied by the checked catalog, not reconstructed from editable descriptors.
pub struct FrozenDefinitionInput<C> {
    pub identity: DefinitionKey,
    pub descriptors: DescriptorGraph,
    pub default_entry: Option<TypeInfoRef>,
    pub default_callable: Option<CallableId>,
    pub environment: CodeFingerprint,
    pub contract_fingerprint: CodeFingerprint,
    pub contract: Arc<C>,
    pub compilation: CodeFingerprint,
}

#[derive(Debug, etas_core::RetainedSize)]
pub struct FrozenDefinition<C> {
    identity: DefinitionKey,
    descriptors: DescriptorGraph,
    default_entry: Option<TypeInfoRef>,
    default_callable: Option<CallableId>,
    environment: CodeFingerprint,
    contract_fingerprint: CodeFingerprint,
    contract: Arc<C>,
    compilation: CodeFingerprint,
    callable: CallableId,
}

impl<C> FrozenDefinition<C> {
    pub fn new(input: FrozenDefinitionInput<C>) -> Result<Arc<Self>, DefinitionError> {
        if !matches!(
            input.identity.kind(),
            DefinitionKind::Agent | DefinitionKind::Flow
        ) {
            return Err(DefinitionError::UnsupportedDefinitionKind);
        }
        let callable = match (&input.default_entry, &input.default_callable) {
            (Some(entry), Some(callable)) => {
                let valid_identity = match input.identity.kind() {
                    DefinitionKind::Agent => callable.as_method().is_some_and(|method| {
                        method.implementation().subject() == &input.identity
                            && method.implementation().selected_spec().is_none()
                            && method.name() == "run"
                    }),
                    DefinitionKind::Flow => callable.as_declaration() == Some(&input.identity),
                    _ => false,
                };
                if !valid_identity
                    || !matches!(
                        input.descriptors.type_info(*entry),
                        Some(TypeInfo::Function(_))
                    )
                {
                    return Err(DefinitionError::InvalidDefaultEntry);
                }
                callable.clone()
            }
            (None, None) if input.identity.kind() == DefinitionKind::Agent => {
                CallableId::declaration(input.identity.clone())
                    .map_err(|_| DefinitionError::InvalidDefaultEntry)?
            }
            _ => return Err(DefinitionError::InvalidDefaultEntry),
        };
        Ok(Arc::new(Self {
            identity: input.identity,
            descriptors: input.descriptors,
            default_entry: input.default_entry,
            default_callable: input.default_callable,
            environment: input.environment,
            contract_fingerprint: input.contract_fingerprint,
            contract: input.contract,
            compilation: input.compilation,
            callable,
        }))
    }

    pub fn identity(&self) -> &DefinitionKey {
        &self.identity
    }

    pub fn descriptors(&self) -> &DescriptorGraph {
        &self.descriptors
    }

    pub fn default_callable(&self) -> Option<&CallableId> {
        self.default_callable.as_ref()
    }

    pub fn signature(&self) -> Option<&SignatureInfo> {
        match self.descriptors.type_info(self.default_entry?)? {
            TypeInfo::Function(signature) => Some(signature),
            _ => None,
        }
    }

    pub fn contract(&self) -> &C {
        &self.contract
    }

    pub fn environment(&self) -> CodeFingerprint {
        self.environment
    }

    pub fn contract_fingerprint(&self) -> CodeFingerprint {
        self.contract_fingerprint
    }
}

/// Code data only. No constructor here can publish an executable or a proof.
#[derive(Debug, etas_core::RetainedSize)]
pub struct DefinitionSnapshot<C, S> {
    base: Arc<FrozenDefinition<C>>,
    body: Option<SyntaxBlockRef>,
    revision: RevisionInfo,
    analysis_configuration: CodeFingerprint,
    analysis: DefinitionAnalysisState<S>,
}

pub struct BaseRevisionInput<S> {
    pub body: Option<SyntaxBlockRef>,
    /// Mandatory even when retention/visibility hides the body.
    pub content: CodeFingerprint,
    pub parent: Option<crate::RevisionKey>,
    pub analysis_configuration: CodeFingerprint,
    pub summaries: Option<Arc<S>>,
}

impl<C, S> DefinitionSnapshot<C, S> {
    pub fn from_base(
        base: Arc<FrozenDefinition<C>>,
        input: BaseRevisionInput<S>,
    ) -> Result<Self, DefinitionError> {
        if base.default_entry.is_none() && (input.body.is_some() || input.summaries.is_some()) {
            return Err(DefinitionError::MissingDefaultEntry);
        }
        if input
            .body
            .as_ref()
            .is_some_and(|body| body.identity().content() != input.content)
        {
            return Err(DefinitionError::BodyContentMismatch);
        }
        let revision = revision(&base, input.content, input.parent);
        let analysis = match input.summaries {
            Some(summaries) => DefinitionAnalysisState::Checked {
                input: AnalysisFingerprint::new(revision.key(), input.analysis_configuration),
                summaries,
            },
            None => DefinitionAnalysisState::Unavailable {
                reason: if base.default_entry.is_some() {
                    AnalysisUnavailableReason::NotAnalyzed
                } else {
                    AnalysisUnavailableReason::MissingDefaultEntry
                },
            },
        };
        Ok(Self {
            base,
            body: input.body,
            revision,
            analysis_configuration: input.analysis_configuration,
            analysis,
        })
    }

    pub fn with_body(&self, body: SyntaxBlockRef) -> Self {
        let revision = revision(
            &self.base,
            body.identity().content(),
            Some(self.revision.key()),
        );
        Self {
            base: Arc::clone(&self.base),
            body: Some(body),
            revision,
            analysis_configuration: self.analysis_configuration,
            analysis: self.analysis.after_body_edit(),
        }
    }

    pub fn base(&self) -> &FrozenDefinition<C> {
        &self.base
    }

    pub fn shared_base(&self) -> Arc<FrozenDefinition<C>> {
        self.base.clone()
    }

    pub fn body(&self) -> Option<&SyntaxBlockRef> {
        self.body.as_ref()
    }

    pub fn revision(&self) -> &RevisionInfo {
        &self.revision
    }

    pub fn analysis(&self) -> &DefinitionAnalysisState<S> {
        &self.analysis
    }

    pub fn current_analysis(&self) -> Result<&S, AnalysisUnavailableReason> {
        self.analysis.current(AnalysisFingerprint::new(
            self.revision.key(),
            self.analysis_configuration,
        ))
    }
}

fn revision<C>(
    base: &FrozenDefinition<C>,
    content: CodeFingerprint,
    parent: Option<crate::RevisionKey>,
) -> RevisionInfo {
    RevisionInfo::new(RevisionInputs {
        definition: base.callable.clone(),
        content,
        environment: base.environment,
        contract: base.contract_fingerprint,
        compilation: base.compilation,
        parent,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionError {
    UnsupportedDefinitionKind,
    InvalidDefaultEntry,
    MissingDefaultEntry,
    BodyContentMismatch,
}

impl fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedDefinitionKind => {
                "editable definition requires an agent or ordinary-flow identity"
            }
            Self::InvalidDefaultEntry => {
                "default entry must reference a checked callable descriptor"
            }
            Self::MissingDefaultEntry => "base agent has no default-entry contract",
            Self::BodyContentMismatch => {
                "retained body does not match the revision content fingerprint"
            }
        })
    }
}

impl std::error::Error for DefinitionError {}
