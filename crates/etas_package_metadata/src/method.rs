use crate::{GenericParamKind, MetadataArtifactError, MethodReceiver, PublishedMethod};

impl PublishedMethod {
    pub fn validate(&self) -> Result<(), MetadataArtifactError> {
        let root = self.root()?;
        for limit in &self.requirements.limits {
            use crate::{MethodLimitKind as K, MethodLimitValue as V};
            if !matches!(
                (&limit.kind, &limit.value),
                (_, None)
                    | (K::Cost, Some(V::MoneyMicros { .. }))
                    | (K::WallTime, Some(V::DurationMillis(_)))
                    | (
                        K::Iterations | K::Tokens | K::ContextTokens | K::Attempts | K::Concurrency,
                        Some(V::Count(_))
                    )
            ) {
                return Err(invalid("method limit value has the wrong budget kind"));
            }
        }
        if root.param_names.len() != root.params.len() {
            return Err(invalid(
                "published method has an incomplete named parameter ABI",
            ));
        }
        if let MethodReceiver::Value { slot } = self.receiver
            && root.params.get(slot as usize) != Some(&self.self_type)
        {
            return Err(invalid(
                "published method receiver does not match its owner type",
            ));
        }
        let mut names = std::collections::BTreeSet::new();
        for param in &self.owner_params {
            if param.name.is_empty()
                || !names.insert(&param.name)
                || param.kind != GenericParamKind::Type
            {
                return Err(invalid(
                    "published method has invalid owner type parameters",
                ));
            }
        }
        let kinds = self
            .owner_params
            .iter()
            .map(|param| param.kind)
            .collect::<Vec<_>>();
        self.callable.validate_references(Some((0, &kinds)))
    }
}

fn invalid(reason: &str) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, reason)
}
