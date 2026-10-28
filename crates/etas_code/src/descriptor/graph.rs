use std::sync::Arc;

use super::{
    DescriptorError, EffectRowInfo, EffectRowInfoRef, GenericParameterInfo, GenericParameterRef,
    TraceInfo, TraceInfoRef, TypeInfo, TypeInfoRef,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct DescriptorLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_expansion_depth: usize,
    pub max_text_bytes: usize,
}

impl Default for DescriptorLimits {
    fn default() -> Self {
        Self {
            max_nodes: 65_536,
            max_edges: 262_144,
            max_expansion_depth: 256,
            max_text_bytes: 8 * 1024 * 1024,
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct DescriptorGraphInput {
    pub types: Vec<TypeInfo>,
    pub rows: Vec<EffectRowInfo>,
    pub binders: Vec<GenericParameterInfo>,
    pub traces: Vec<TraceInfo>,
}

impl DescriptorGraphInput {
    pub fn validate(&self, limits: DescriptorLimits) -> Result<(), DescriptorError> {
        super::validation::validate(self, limits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct DescriptorGraph(Arc<DescriptorGraphInput>);

impl DescriptorGraph {
    pub(super) fn from_validated_projection(input: DescriptorGraphInput) -> Self {
        Self(Arc::new(input))
    }

    pub fn validate(&self, limits: DescriptorLimits) -> Result<(), DescriptorError> {
        super::validation::validate(&self.0, limits)
    }
    pub fn new(
        input: DescriptorGraphInput,
        limits: DescriptorLimits,
    ) -> Result<Self, DescriptorError> {
        super::validation::validate(&input, limits)?;
        Ok(Self(Arc::new(input)))
    }

    pub fn type_info(&self, reference: TypeInfoRef) -> Option<&TypeInfo> {
        self.0.types.get(reference.0 as usize)
    }
    pub fn effect_row(&self, reference: EffectRowInfoRef) -> Option<&EffectRowInfo> {
        self.0.rows.get(reference.0 as usize)
    }
    pub fn binder(&self, reference: GenericParameterRef) -> Option<&GenericParameterInfo> {
        self.0.binders.get(reference.0 as usize)
    }
    pub fn trace(&self, reference: TraceInfoRef) -> Option<&TraceInfo> {
        self.0.traces.get(reference.0 as usize)
    }
    pub fn data(&self) -> &DescriptorGraphInput {
        &self.0
    }
}
