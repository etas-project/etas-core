use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheNamespace(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactKindKey(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactUnitKey(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactKey {
    pub namespace: CacheNamespace,
    pub kind: ArtifactKindKey,
    pub unit: ArtifactUnitKey,
}

impl CacheNamespace {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl ArtifactKindKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl ArtifactUnitKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl ArtifactKey {
    pub fn new(
        namespace: impl Into<String>,
        kind: impl Into<String>,
        unit: impl Into<String>,
    ) -> Self {
        Self {
            namespace: CacheNamespace::new(namespace),
            kind: ArtifactKindKey::new(kind),
            unit: ArtifactUnitKey::new(unit),
        }
    }
}

impl fmt::Display for ArtifactKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}:{}",
            self.namespace.as_str(),
            self.kind.as_str(),
            self.unit.as_str()
        )
    }
}
