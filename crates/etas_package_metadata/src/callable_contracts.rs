use etas_code::MethodId;

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
