use crate::{GenericParamKind, MetadataArtifactError, MethodReceiver, PublishedMethod};

impl PublishedMethod {
    pub fn validate(&self) -> Result<(), MetadataArtifactError> {
        if matches!(self.receiver, MethodReceiver::Updating { slot } if slot != 0) {
            return Err(invalid(
                "updating receiver must be the first method parameter",
            ));
        }
        let root = self.root()?;
        let mut previous = None;
        let default_kinds = root
            .generic_params
            .iter()
            .chain(&self.owner_params)
            .map(|parameter| parameter.kind)
            .collect::<Vec<_>>();
        for default in &self.parameter_defaults {
            if default.slot as usize >= root.params.len()
                || previous.is_some_and(|old| old >= default.slot)
                || default.reference.owner
                    != etas_code::descriptor::ParameterDefaultOwner::Method(self.identity.clone())
            {
                return Err(invalid(
                    "method default has a different checked calling slot or owner",
                ));
            }
            previous = Some(default.slot);
            if let Some(graph) = &default.callable_value {
                graph.validate_references(Some((root.params.len(), &default_kinds)))?;
            }
        }
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
        if let MethodReceiver::Value { slot } | MethodReceiver::Updating { slot } = self.receiver
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
        self.callable
            .validate_references(Some((0, &kinds)))
            .map_err(|error| invalid(&format!("published method {:?}: {error}", self.identity)))
    }
}

fn invalid(reason: &str) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, reason)
}
