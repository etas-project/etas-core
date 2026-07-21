use std::collections::BTreeSet;

use super::{UnitKey, UnitKindKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactKey {
    pub namespace: &'static str,
    pub name: &'static str,
}

impl ArtifactKey {
    pub const fn new(namespace: &'static str, name: &'static str) -> Self {
        Self { namespace, name }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactScope {
    Global,
    Unit(UnitKey),
    UnitKind(UnitKindKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactRef {
    pub key: ArtifactKey,
    pub scope: ArtifactScope,
}

impl ArtifactRef {
    pub const fn global(key: ArtifactKey) -> Self {
        Self {
            key,
            scope: ArtifactScope::Global,
        }
    }

    pub const fn unit(key: ArtifactKey, unit: UnitKey) -> Self {
        Self {
            key,
            scope: ArtifactScope::Unit(unit),
        }
    }

    pub const fn unit_kind(key: ArtifactKey, unit_kind: UnitKindKey) -> Self {
        Self {
            key,
            scope: ArtifactScope::UnitKind(unit_kind),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactSet {
    artifacts: BTreeSet<ArtifactRef>,
}

impl ArtifactSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn one(key: ArtifactKey) -> Self {
        Self::from_iter([key])
    }

    pub fn insert(&mut self, key: ArtifactKey) -> bool {
        self.insert_ref(ArtifactRef::global(key))
    }

    pub fn insert_ref(&mut self, artifact: ArtifactRef) -> bool {
        self.artifacts.insert(artifact)
    }

    pub fn contains(&self, key: ArtifactKey) -> bool {
        self.contains_ref(ArtifactRef::global(key))
    }

    pub fn contains_ref(&self, artifact: ArtifactRef) -> bool {
        self.artifacts.contains(&artifact)
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ArtifactRef> + '_ {
        self.artifacts.iter().copied()
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = ArtifactKey> + '_ {
        self.artifacts.iter().map(|artifact| artifact.key)
    }

    pub fn iter_refs(&self) -> impl Iterator<Item = ArtifactRef> + '_ {
        self.iter()
    }

    pub fn extend(&mut self, other: &ArtifactSet) {
        self.artifacts.extend(other.iter_refs());
    }

    pub fn difference(&self, preserved: &ArtifactSet) -> ArtifactSet {
        self.artifacts
            .difference(&preserved.artifacts)
            .copied()
            .collect()
    }
}

impl<const N: usize> From<[ArtifactKey; N]> for ArtifactSet {
    fn from(keys: [ArtifactKey; N]) -> Self {
        Self::from_iter(keys)
    }
}

impl FromIterator<ArtifactKey> for ArtifactSet {
    fn from_iter<T: IntoIterator<Item = ArtifactKey>>(iter: T) -> Self {
        Self {
            artifacts: iter.into_iter().map(ArtifactRef::global).collect(),
        }
    }
}

impl FromIterator<ArtifactRef> for ArtifactSet {
    fn from_iter<T: IntoIterator<Item = ArtifactRef>>(iter: T) -> Self {
        Self {
            artifacts: iter.into_iter().collect(),
        }
    }
}
