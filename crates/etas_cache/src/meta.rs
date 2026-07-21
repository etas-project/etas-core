use crate::{ArtifactFingerprint, ArtifactKey, ContentHash, ProjectRevision};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactMeta {
    pub revision: ProjectRevision,
    pub fingerprint: ArtifactFingerprint,
    pub payload_hash: Option<ContentHash>,
    pub payload_size: Option<u64>,
    pub dependencies: Vec<ArtifactKey>,
    pub compiler_version: String,
    pub std_version: Option<String>,
    pub options_hash: Option<String>,
    pub cache_schema_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedArtifact<T> {
    pub key: ArtifactKey,
    pub meta: ArtifactMeta,
    pub value: T,
}

impl ArtifactMeta {
    pub fn new(
        revision: ProjectRevision,
        fingerprint: ArtifactFingerprint,
        compiler_version: impl Into<String>,
        cache_schema_version: u32,
    ) -> Self {
        Self {
            revision,
            fingerprint,
            payload_hash: None,
            payload_size: None,
            dependencies: Vec::new(),
            compiler_version: compiler_version.into(),
            std_version: None,
            options_hash: None,
            cache_schema_version,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<ArtifactKey>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_payload(mut self, payload_hash: ContentHash, payload_size: u64) -> Self {
        self.payload_hash = Some(payload_hash);
        self.payload_size = Some(payload_size);
        self
    }

    pub fn with_std_version(mut self, std_version: impl Into<String>) -> Self {
        self.std_version = Some(std_version.into());
        self
    }

    pub fn with_options_hash(mut self, options_hash: impl Into<String>) -> Self {
        self.options_hash = Some(options_hash.into());
        self
    }
}
