use std::{collections::BTreeMap, sync::Arc};

use etas_code::descriptor::{DefinitionDescriptorSet, GenericOwner, GenericParameterKind};

use crate::{GenericTypeParameter, MetadataArtifactError, PublishedMethod};

type OwnerBindings = BTreeMap<GenericOwner, BTreeMap<u32, GenericParameterKind>>;

/// Import-local index of the canonical binders declared by checked descriptors.
#[derive(Clone, Default)]
pub struct MetadataTypeParameters(Arc<OwnerBindings>);

impl MetadataTypeParameters {
    pub fn from_descriptors(set: &DefinitionDescriptorSet) -> Result<Self, MetadataArtifactError> {
        crate::validate_definition_descriptors(set)?;
        Ok(Self::from_checked_descriptors(Some(set)))
    }

    pub(crate) fn from_checked_descriptors(set: Option<&DefinitionDescriptorSet>) -> Self {
        let mut owners = OwnerBindings::new();
        for binder in set.into_iter().flat_map(|set| &set.graph.binders) {
            owners
                .entry(binder.owner.clone())
                .or_default()
                .insert(binder.position, binder.kind);
        }
        Self(Arc::new(owners))
    }

    pub fn validate_reference(
        &self,
        parameter: &GenericTypeParameter,
    ) -> Result<(), MetadataArtifactError> {
        if parameter.owner.is_portable_parameter_owner()
            && self
                .0
                .get(&parameter.owner)
                .and_then(|positions| positions.get(&parameter.position))
                == Some(&GenericParameterKind::Type)
        {
            Ok(())
        } else {
            Err(MetadataArtifactError::invalid(
                crate::PACKAGE_METADATA_FILE,
                "type parameter reference has no declared type binder at its canonical owner and position",
            ))
        }
    }

    pub fn validate_method(&self, method: &PublishedMethod) -> Result<(), MetadataArtifactError> {
        crate::proto::validate_method_type_parameters(method, self.clone())
    }
}
