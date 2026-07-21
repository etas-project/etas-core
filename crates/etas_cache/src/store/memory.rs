use std::{any::Any, collections::HashMap};

use crate::{
    ArtifactDependencyGraph, ArtifactKey, ArtifactMeta, CacheResult, CachedArtifact,
    InvalidationReport, InvalidationSelector,
    store::{ArtifactStore, TypedArtifactStore},
};

#[derive(Default)]
pub struct MemoryArtifactStore {
    artifacts: HashMap<ArtifactKey, MemoryArtifact>,
    dependencies: ArtifactDependencyGraph,
}

struct MemoryArtifact {
    meta: ArtifactMeta,
    value: Box<dyn Any>,
}

impl MemoryArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dependency_graph(&self) -> &ArtifactDependencyGraph {
        &self.dependencies
    }

    fn roots_for(&self, selector: InvalidationSelector) -> Vec<ArtifactKey> {
        match selector {
            InvalidationSelector::Exact(key) => vec![key],
            InvalidationSelector::Roots(keys) => keys,
            InvalidationSelector::Namespace(namespace) => self
                .artifacts
                .keys()
                .filter(|key| key.namespace == namespace)
                .cloned()
                .collect(),
            InvalidationSelector::Kind { namespace, kind } => self
                .artifacts
                .keys()
                .filter(|key| key.namespace == namespace && key.kind == kind)
                .cloned()
                .collect(),
        }
    }
}

impl ArtifactStore for MemoryArtifactStore {
    fn contains(&self, key: &ArtifactKey) -> CacheResult<bool> {
        Ok(self.artifacts.contains_key(key))
    }

    fn meta(&self, key: &ArtifactKey) -> CacheResult<Option<ArtifactMeta>> {
        Ok(self.artifacts.get(key).map(|entry| entry.meta.clone()))
    }

    fn remove(&mut self, key: &ArtifactKey) -> CacheResult<()> {
        self.artifacts.remove(key);
        self.dependencies.remove_artifact(key);
        Ok(())
    }

    fn invalidate(&mut self, selector: InvalidationSelector) -> CacheResult<InvalidationReport> {
        let roots = self.roots_for(selector);
        let invalidation = self.dependencies.invalidate_from(&roots);
        let invalidated = invalidation.into_keys();
        for key in &invalidated {
            self.remove(key)?;
        }
        Ok(InvalidationReport { roots, invalidated })
    }
}

impl TypedArtifactStore for MemoryArtifactStore {
    fn get<T: Clone + 'static>(&self, key: &ArtifactKey) -> CacheResult<Option<CachedArtifact<T>>> {
        let Some(entry) = self.artifacts.get(key) else {
            return Ok(None);
        };
        let Some(value) = entry.value.downcast_ref::<T>() else {
            return Ok(None);
        };
        Ok(Some(CachedArtifact {
            key: key.clone(),
            meta: entry.meta.clone(),
            value: value.clone(),
        }))
    }

    fn put<T: Clone + 'static>(&mut self, artifact: CachedArtifact<T>) -> CacheResult<()> {
        self.dependencies
            .set_dependencies(artifact.key.clone(), artifact.meta.dependencies.clone());
        self.artifacts.insert(
            artifact.key,
            MemoryArtifact {
                meta: artifact.meta,
                value: Box::new(artifact.value),
            },
        );
        Ok(())
    }
}
