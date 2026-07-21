use std::{any::Any, collections::HashMap};

use super::{ArtifactKey, ArtifactSet, PreservedArtifacts};

pub trait Analysis<C> {
    type Output: 'static;

    fn key(&self) -> ArtifactKey;
    fn run(&self, context: &C) -> Self::Output;
}

#[derive(Default)]
pub struct AnalysisCache {
    entries: HashMap<ArtifactKey, Box<dyn Any>>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, key: ArtifactKey) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn insert<T: 'static>(&mut self, key: ArtifactKey, value: T) {
        self.entries.insert(key, Box::new(value));
    }

    pub fn get<T: 'static>(&self, key: ArtifactKey) -> Option<&T> {
        self.entries.get(&key)?.downcast_ref()
    }

    pub fn get_mut<T: 'static>(&mut self, key: ArtifactKey) -> Option<&mut T> {
        self.entries.get_mut(&key)?.downcast_mut()
    }

    pub fn get_or_run<C, A>(&mut self, analysis: &A, context: &C) -> &A::Output
    where
        A: Analysis<C>,
    {
        let key = analysis.key();
        if !self.entries.contains_key(&key) {
            self.insert(key, analysis.run(context));
        }
        self.get(key)
            .expect("analysis cache entry should have requested output type")
    }

    pub fn invalidate(&mut self, preserved: &PreservedArtifacts) -> ArtifactSet {
        match preserved {
            PreservedArtifacts::All => ArtifactSet::new(),
            PreservedArtifacts::None => {
                let invalidated = self.entries.keys().copied().collect();
                self.entries.clear();
                invalidated
            }
            PreservedArtifacts::Some(keys) => {
                let invalidated = self
                    .entries
                    .keys()
                    .copied()
                    .filter(|key| !keys.contains(*key))
                    .collect::<ArtifactSet>();
                for artifact in invalidated.iter() {
                    self.entries.remove(&artifact.key);
                }
                invalidated
            }
        }
    }
}
