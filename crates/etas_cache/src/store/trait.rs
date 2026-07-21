use crate::{
    ArtifactKey, ArtifactMeta, CacheResult, CachedArtifact, InvalidationReport,
    InvalidationSelector,
};

pub trait ArtifactStore {
    fn contains(&self, key: &ArtifactKey) -> CacheResult<bool>;
    fn meta(&self, key: &ArtifactKey) -> CacheResult<Option<ArtifactMeta>>;
    fn remove(&mut self, key: &ArtifactKey) -> CacheResult<()>;
    fn invalidate(&mut self, selector: InvalidationSelector) -> CacheResult<InvalidationReport>;
}

pub trait TypedArtifactStore: ArtifactStore {
    fn get<T: Clone + 'static>(&self, key: &ArtifactKey) -> CacheResult<Option<CachedArtifact<T>>>;
    fn put<T: Clone + 'static>(&mut self, artifact: CachedArtifact<T>) -> CacheResult<()>;
}
