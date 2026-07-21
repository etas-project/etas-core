use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::{ArtifactKey, InvalidationSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactDependencyGraph {
    dependencies: HashMap<ArtifactKey, BTreeSet<ArtifactKey>>,
    reverse_dependencies: HashMap<ArtifactKey, BTreeSet<ArtifactKey>>,
}

impl ArtifactDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_dependency(&mut self, artifact: ArtifactKey, depends_on: ArtifactKey) {
        self.dependencies
            .entry(artifact.clone())
            .or_default()
            .insert(depends_on.clone());
        self.reverse_dependencies
            .entry(depends_on)
            .or_default()
            .insert(artifact);
    }

    pub fn set_dependencies(&mut self, artifact: ArtifactKey, depends_on: Vec<ArtifactKey>) {
        self.remove_artifact_edges(&artifact);
        for dependency in depends_on {
            self.add_dependency(artifact.clone(), dependency);
        }
        self.dependencies.entry(artifact).or_default();
    }

    pub fn dependencies_of(&self, artifact: &ArtifactKey) -> Vec<ArtifactKey> {
        self.dependencies
            .get(artifact)
            .map(|keys| keys.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn dependents_of(&self, key: &ArtifactKey) -> Vec<ArtifactKey> {
        self.reverse_dependencies
            .get(key)
            .map(|keys| keys.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn invalidate_from(&self, roots: &[ArtifactKey]) -> InvalidationSet {
        let mut invalidated = InvalidationSet::new();
        let mut queue = VecDeque::from(roots.to_vec());
        while let Some(key) = queue.pop_front() {
            if !invalidated.insert(key.clone()) {
                continue;
            }
            for dependent in self.dependents_of(&key) {
                queue.push_back(dependent);
            }
        }
        invalidated
    }

    pub fn remove_artifact(&mut self, artifact: &ArtifactKey) {
        self.remove_artifact_edges(artifact);
        self.dependencies.remove(artifact);
        self.reverse_dependencies.remove(artifact);
        for dependents in self.reverse_dependencies.values_mut() {
            dependents.remove(artifact);
        }
        for dependencies in self.dependencies.values_mut() {
            dependencies.remove(artifact);
        }
    }

    fn remove_artifact_edges(&mut self, artifact: &ArtifactKey) {
        if let Some(old_dependencies) = self.dependencies.remove(artifact) {
            for dependency in old_dependencies {
                if let Some(dependents) = self.reverse_dependencies.get_mut(&dependency) {
                    dependents.remove(artifact);
                    if dependents.is_empty() {
                        self.reverse_dependencies.remove(&dependency);
                    }
                }
            }
        }
    }
}
