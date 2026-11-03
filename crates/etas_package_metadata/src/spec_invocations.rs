use crate::{CallableValueGraph, EffectRow, Type};

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum CallableRequestTarget {
    Parameter(String),
    SpecMethod(Box<SpecMethodInvocation>),
}

impl From<String> for CallableRequestTarget {
    fn from(parameter: String) -> Self {
        Self::Parameter(parameter)
    }
}
impl From<&str> for CallableRequestTarget {
    fn from(parameter: &str) -> Self {
        Self::Parameter(parameter.into())
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct SpecMethodInvocation {
    pub receiver: Type,
    pub spec: Vec<String>,
    pub spec_args: Vec<Type>,
    pub method: String,
    pub arity: u32,
    /// Declaration order, excluding the method-local implicit receiver binder.
    pub generic_arguments: Vec<SpecMethodGenericArgument>,
    pub callable_arguments: Vec<SpecMethodCallableArgument>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum SpecMethodGenericArgument {
    Type(Box<Type>),
    EffectRow(EffectRow),
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct SpecMethodCallableArgument {
    /// Index in the complete method input list, including its receiver.
    pub position: u32,
    /// Root parameters and generic binders belong to the enclosing callable.
    pub sources: CallableValueGraph,
}

impl CallableRequestTarget {
    pub fn validate_resources(&self) -> Result<(), crate::MetadataArtifactError> {
        crate::proto::validate_callable_request_target(self)
    }

    pub fn fingerprint(&self) -> Result<etas_code::CodeFingerprint, crate::MetadataArtifactError> {
        self.validate_resources()?;
        Ok(crate::proto::callable_request_target_fingerprint(self))
    }

    pub(crate) fn validate_shape(&self) -> Result<(), String> {
        match self {
            Self::Parameter(parameter) if parameter.is_empty() => {
                Err("callable request parameter is empty".into())
            }
            Self::Parameter(_) => Ok(()),
            Self::SpecMethod(invocation) => {
                if invocation.spec.is_empty()
                    || invocation.spec.iter().any(String::is_empty)
                    || invocation.method.is_empty()
                    || invocation.arity == 0
                {
                    return Err(
                        "spec invocation requires a spec identity, member and receiver input"
                            .into(),
                    );
                }
                let mut positions = std::collections::BTreeSet::new();
                for argument in &invocation.callable_arguments {
                    if argument.position >= invocation.arity || !positions.insert(argument.position)
                    {
                        return Err(
                            "spec invocation callable argument positions are invalid or duplicated"
                                .into(),
                        );
                    }
                }
                Ok(())
            }
        }
    }
}
