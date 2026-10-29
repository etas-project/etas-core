use crate::{MetadataArtifactError, RequestInstantiationTemplate};
use std::collections::BTreeSet;

impl RequestInstantiationTemplate {
    pub(crate) fn validate_shape(&self) -> Result<(), MetadataArtifactError> {
        let invalid = |reason| MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, reason);
        for action in &self.actions {
            if action.action.path.is_empty() || action.action.path.iter().any(String::is_empty) {
                return Err(invalid("request instance action path is required"));
            }
            if let Some(bindings) = &action.type_bindings {
                let mut names = BTreeSet::new();
                for binding in bindings {
                    if binding.parameter.is_empty() || !names.insert(&binding.parameter) {
                        return Err(invalid(
                            "request instance has invalid or duplicate generic parameters",
                        ));
                    }
                }
            }
        }
        for parameter in &self.parameters {
            parameter.parameter.validate_shape().map_err(|reason| {
                MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, reason)
            })?;
            for effect in &parameter.excluded {
                if effect.path.is_empty() || effect.path.iter().any(String::is_empty) {
                    return Err(invalid("request template handler mask path is required"));
                }
            }
        }
        Ok(())
    }
}
