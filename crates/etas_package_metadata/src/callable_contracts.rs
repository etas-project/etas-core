use etas_code::MethodId;

/// Checked promise that these callback parameters are used only inside fresh,
/// drained task scopes and are never returned or retained by the callable.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct ScopedCallbackContract {
    pub target: CallableContractTarget,
    pub parameters: Vec<u32>,
    /// Callback slots whose returned values cross a fresh resource owner.
    /// Their results and nonlocal writes must not retain newly owned handles.
    #[serde(default)]
    pub resource_boundaries: Vec<u32>,
}

impl ScopedCallbackContract {
    pub fn validate(
        &self,
        metadata: &crate::PublicMetadata,
    ) -> Result<(), crate::MetadataArtifactError> {
        let invalid = || {
            crate::MetadataArtifactError::invalid(
                crate::PACKAGE_METADATA_FILE,
                "scoped callback contract requires distinct callable parameter slots on its published target",
            )
        };
        let parameters = match &self.target {
            CallableContractTarget::Item(path) => {
                &metadata
                    .flows
                    .iter()
                    .find(|flow| &flow.path == path)
                    .ok_or_else(invalid)?
                    .input
            }
            CallableContractTarget::Method(identity) => {
                &metadata
                    .methods
                    .iter()
                    .find(|method| &method.identity == identity)
                    .ok_or_else(invalid)?
                    .root()?
                    .params
            }
        };
        if self.parameters.is_empty()
            || self.parameters.windows(2).any(|pair| pair[0] >= pair[1])
            || self
                .resource_boundaries
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self
                .resource_boundaries
                .iter()
                .any(|slot| !self.parameters.contains(slot))
            || self.parameters.iter().any(|slot| {
                parameters
                    .get(*slot as usize)
                    .is_none_or(|ty| ty.kind != crate::TypeKind::Function)
            })
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// A public declaration is import-root relative; a method keeps its canonical
/// implementation identity when the same dependency is imported under aliases.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
#[serde(tag = "kind", content = "identity", rename_all = "snake_case")]
pub enum CallableContractTarget {
    Item(Vec<String>),
    Method(MethodId),
}

impl CallableContractTarget {
    pub fn item_path(&self) -> Option<&[String]> {
        match self {
            Self::Item(path) => Some(path),
            Self::Method(_) => None,
        }
    }
}

impl From<Vec<String>> for CallableContractTarget {
    fn from(path: Vec<String>) -> Self {
        Self::Item(path)
    }
}
