use super::*;

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct ExistentialValueNode {
    pub parent_scope: CallableValueScope,
    pub generic_bindings: Vec<CallableValueGenericBinding>,
    pub object_type: Type,
    pub source_type: Type,
    pub payload_sources: Vec<CallableValueSource>,
    pub witnesses: Vec<ExistentialMemberWitness>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct ExistentialMemberWitness {
    pub bound: crate::SpecBound,
    pub member: String,
    pub target: ExistentialWitnessTarget,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum ExistentialWitnessTarget {
    /// Analysis of the checked implementation, not an executable permission.
    Implementation {
        method: etas_code::MethodId,
        value: u32,
    },
    /// The producer's checked generic/packed evidence supplies this member.
    Bound,
}

impl CallableValueGraph {
    pub(super) fn validate_objects(
        &self,
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        for object in &self.objects {
            if object.object_type.kind != crate::TypeKind::Existential {
                return Err(invalid("object evidence has no existential interface"));
            }
            let scopes = self.lexical_scopes(object.parent_scope)?;
            let mut labels = std::collections::BTreeSet::new();
            for binding in &object.generic_bindings {
                if !scopes.contains(&binding.scope) || !labels.insert(&binding.label) {
                    return Err(invalid(
                        "object generic binding escapes or duplicates its lexical scope",
                    ));
                }
                self.generic_parameter(binding.scope, binding.position, root)?;
            }
            self.validate_object_descriptors(object, root)?;
            if !object.payload_sources.is_empty() {
                self.validate_sources(&object.payload_sources, root)?;
                for source in &object.payload_sources {
                    self.validate_source_scope(*source, &scopes)?;
                }
            }
            let mut members = std::collections::BTreeSet::new();
            for witness in &object.witnesses {
                if witness.member.is_empty()
                    || !members.insert((
                        super::fingerprint::bound_input_fingerprint(&witness.bound),
                        witness.member.as_str(),
                    ))
                {
                    return Err(invalid("object member evidence is empty or duplicated"));
                }
                if let ExistentialWitnessTarget::Implementation { method, value } = &witness.target
                {
                    if method.name() != witness.member
                        || method.implementation().selected_spec().is_none()
                    {
                        return Err(invalid(
                            "object member target has a different checked method identity",
                        ));
                    }
                    self.value(*value)?;
                    self.validate_source_scope(CallableValueSource::Value(*value), &scopes)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn lexical_scopes(
        &self,
        mut scope: CallableValueScope,
    ) -> Result<std::collections::BTreeSet<CallableValueScope>, crate::MetadataArtifactError> {
        let mut scopes = std::collections::BTreeSet::from([CallableValueScope::Root]);
        while let CallableValueScope::Value(index) = scope {
            if !scopes.insert(scope) {
                return Err(invalid("value evidence lexical scopes contain a cycle"));
            }
            if scopes.len() > crate::MAX_METADATA_GRAPH_DEPTH + 1 {
                return Err(invalid(
                    "value evidence lexical scopes exceed the maximum depth",
                ));
            }
            scope = self.value(index)?.parent_scope;
        }
        Ok(scopes)
    }

    pub(super) fn object(
        &self,
        index: u32,
    ) -> Result<&ExistentialValueNode, crate::MetadataArtifactError> {
        self.objects
            .get(index as usize)
            .ok_or_else(|| invalid("object value reference is out of range"))
    }
}
