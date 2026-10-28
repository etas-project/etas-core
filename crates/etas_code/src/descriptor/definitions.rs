use crate::{CallableId, DefinitionKey};

use super::{AnnotationInfo, AttachedSpecInfo, CallableAnalysisInfo, TypeInfoRef};

mod bodies;
mod headers;
pub use bodies::{BodyAccess, CallableBodyInfo, DefinitionBodies};
mod snapshots;
pub use snapshots::{AgentSnapshotInfo, DefinitionSnapshots};

/// Portable descriptions. These records do not retain executable code or proofs.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionDescriptorSet {
    pub graph: super::DescriptorGraphInput,
    pub declarations: Vec<DefinitionTypeBinding>,
    #[serde(default)]
    pub headers: Option<DefinitionHeaders>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionHeaders {
    pub callables: Vec<CallableHeaderInfo>,
    pub agents: Vec<AgentHeaderInfo>,
    /// None identifies a producer that did not publish revision coordinates.
    #[serde(default)]
    pub snapshots: Option<DefinitionSnapshots>,
    #[serde(default)]
    pub bodies: Option<DefinitionBodies>,
}

impl DefinitionHeaders {
    pub fn resource_units(
        &self,
        limits: super::DescriptorLimits,
    ) -> Result<usize, super::DescriptorError> {
        headers::measure(self, limits).map(|resources| resources.nodes)
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionTypeBinding {
    pub definition: DefinitionKey,
    pub ty: TypeInfoRef,
}

impl DefinitionDescriptorSet {
    pub fn validate(&self, limits: super::DescriptorLimits) -> Result<(), super::DescriptorError> {
        use super::{DescriptorError, DescriptorNode, TypeInfo};
        if self.declarations.len() > limits.max_nodes {
            return Err(DescriptorError::Limit {
                resource: "declarations",
                limit: limits.max_nodes,
            });
        }
        let resources = self
            .headers
            .as_ref()
            .map(|headers| headers::measure(headers, limits))
            .transpose()?
            .unwrap_or_default();
        let reserved = resources.nodes.saturating_add(self.declarations.len());
        if reserved > limits.max_nodes {
            return Err(DescriptorError::Limit {
                resource: "nodes",
                limit: limits.max_nodes,
            });
        }
        let mut text_bytes = resources.text;
        for binding in &self.declarations {
            for segment in binding
                .definition
                .module()
                .iter()
                .chain(binding.definition.declaration())
            {
                text_bytes = text_bytes.saturating_add(segment.len());
                if text_bytes > limits.max_text_bytes {
                    return Err(DescriptorError::Limit {
                        resource: "text bytes",
                        limit: limits.max_text_bytes,
                    });
                }
            }
        }
        self.graph.validate(super::DescriptorLimits {
            max_text_bytes: limits.max_text_bytes - text_bytes,
            max_nodes: limits.max_nodes - reserved,
            ..limits
        })?;
        let mut seen = std::collections::BTreeSet::new();
        for binding in &self.declarations {
            if !seen.insert(&binding.definition) {
                return Err(DescriptorError::InvalidDescription {
                    reason: "duplicate descriptor declaration identity",
                });
            }
            let target = self.graph.types.get(binding.ty.0 as usize).ok_or(
                DescriptorError::MissingReference {
                    from: DescriptorNode::Type(binding.ty),
                    target: DescriptorNode::Type(binding.ty),
                },
            )?;
            let valid = match target {
                TypeInfo::Nominal { definition, .. }
                | TypeInfo::Alias { definition, .. }
                | TypeInfo::Enum { definition, .. } => definition == &binding.definition,
                TypeInfo::Function(signature) => {
                    matches!(
                        binding.definition.kind(),
                        crate::DefinitionKind::Flow | crate::DefinitionKind::Tool
                    ) && signature.generics.iter().all(|parameter| {
                        self.graph
                            .binders
                            .get(parameter.0 as usize)
                            .is_some_and(|parameter| {
                                parameter.owner
                                    == super::GenericOwner::Declaration(binding.definition.clone())
                            })
                    })
                }
                _ => false,
            };
            if !valid {
                return Err(DescriptorError::InvalidDescription {
                    reason: "descriptor declaration identity disagrees with its graph root",
                });
            }
        }
        if let Some(headers) = &self.headers {
            headers::validate(headers, &self.graph, &self.declarations)?;
            if let Some(snapshots) = &headers.snapshots {
                snapshots::validate(snapshots, headers)?;
            }
            if let Some(bodies) = &headers.bodies {
                if headers.snapshots.is_none() {
                    return Err(DescriptorError::InvalidDescription {
                        reason: "retained bodies require snapshot metadata",
                    });
                }
                bodies::validate(bodies, headers)?;
            }
        }
        Ok(())
    }
}

/// Inert catalog headers. They describe code but do not retain executable proof.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct CallableHeaderInfo {
    pub id: CallableId,
    pub signature: TypeInfoRef,
    pub analysis: CallableAnalysisInfo,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct AgentHeaderInfo {
    pub identity: DefinitionKey,
    pub component: TypeInfoRef,
    pub default_entry: Option<CallableId>,
    pub methods: Vec<CallableId>,
    /// None denotes a legacy description without method selection facts.
    #[serde(default)]
    pub method_info: Option<Vec<AgentMethodInfo>>,
    pub tools: Vec<DefinitionKey>,
    pub annotations: Vec<AnnotationInfo>,
    pub specs: Vec<AttachedSpecInfo>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct AgentMethodInfo {
    pub callable: CallableId,
    pub inherent: bool,
    pub is_public: bool,
    pub specs: Vec<super::SpecInfo>,
}

impl AgentMethodInfo {
    pub fn fingerprint(
        &self,
        graph: &super::DescriptorGraph,
    ) -> Result<crate::CodeFingerprint, super::DescriptorError> {
        use crate::CodeFingerprint as F;
        let mut specs = Vec::new();
        for spec in &self.specs {
            let mut parts = vec![spec.definition.fingerprint()];
            for arg in &spec.arguments {
                parts.push(graph.type_fingerprint(*arg)?);
            }
            specs.push(F::of_parts(
                "etas.code.method-spec-binding.v1",
                parts.iter().map(|part| part.as_bytes().as_slice()),
            ));
        }
        specs.sort_unstable();
        let mut parts = vec![
            self.callable.fingerprint(),
            F::of_bytes(
                "etas.code.method-selection-kind.v1",
                &[u8::from(self.inherent), u8::from(self.is_public)],
            ),
        ];
        parts.extend(specs);
        Ok(F::of_parts(
            "etas.code.method-selection.v1",
            parts.iter().map(|part| part.as_bytes().as_slice()),
        ))
    }
}

impl AgentHeaderInfo {
    pub fn retain_additional_methods(&mut self, mut keep: impl FnMut(&CallableId) -> bool) {
        self.methods.retain(|id| keep(id));
        let visible = self
            .default_entry
            .iter()
            .chain(&self.methods)
            .collect::<std::collections::BTreeSet<_>>();
        if let Some(info) = &mut self.method_info {
            info.retain(|method| visible.contains(&method.callable));
        }
    }
}
