use std::collections::BTreeSet;

use crate::{ArtifactKey, ArtifactKindKey, CacheNamespace};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvalidationSelector {
    Exact(ArtifactKey),
    Roots(Vec<ArtifactKey>),
    Namespace(CacheNamespace),
    Kind {
        namespace: CacheNamespace,
        kind: ArtifactKindKey,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InvalidationSet {
    seen: BTreeSet<ArtifactKey>,
    keys: Vec<ArtifactKey>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InvalidationReport {
    pub roots: Vec<ArtifactKey>,
    pub invalidated: Vec<ArtifactKey>,
}

impl InvalidationSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: ArtifactKey) -> bool {
        if !self.seen.insert(key.clone()) {
            return false;
        }
        self.keys.push(key);
        true
    }

    pub fn contains(&self, key: &ArtifactKey) -> bool {
        self.seen.contains(key)
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &ArtifactKey> {
        self.keys.iter()
    }

    pub fn into_keys(self) -> Vec<ArtifactKey> {
        self.keys
    }
}
