use std::collections::BTreeMap;

use crate::{
    CodeFingerprint,
    descriptor::{
        DescriptorError, DescriptorFingerprintBudget, DescriptorGraph, DescriptorLimits,
        TypeInfoRef,
    },
};

use super::identity::{GenericParameterId, MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS};

/// Canonical closed type arguments supplied by a checked application. All roots
/// share one graph, retaining nominal declarations, binders and recursive edges.
#[derive(Clone, Debug, Default)]
pub struct TypeArgumentDescriptors {
    graph: DescriptorGraph,
    arguments: BTreeMap<GenericParameterId, (TypeInfoRef, CodeFingerprint)>,
}

impl TypeArgumentDescriptors {
    pub fn new(
        graph: DescriptorGraph,
        arguments: BTreeMap<GenericParameterId, TypeInfoRef>,
        limits: DescriptorLimits,
    ) -> Result<Self, DescriptorError> {
        graph.validate(limits)?;
        if arguments.len() > MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS {
            return Err(DescriptorError::Limit {
                resource: "type argument bindings",
                limit: MAX_RESIDUAL_REFERENCE_SUBSTITUTIONS,
            });
        }
        let mut budget = DescriptorFingerprintBudget::new(limits.max_edges);
        let arguments = arguments
            .into_iter()
            .map(|(parameter, root)| {
                graph
                    .type_fingerprint_with_budget(root, &mut budget)
                    .map(|fingerprint| (parameter, (root, fingerprint)))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { graph, arguments })
    }

    pub fn graph(&self) -> &DescriptorGraph {
        &self.graph
    }

    pub fn argument(
        &self,
        parameter: GenericParameterId,
    ) -> Option<(TypeInfoRef, CodeFingerprint)> {
        self.arguments.get(&parameter).copied()
    }
}
