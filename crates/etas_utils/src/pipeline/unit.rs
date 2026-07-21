use std::marker::PhantomData;

use super::ArtifactKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitKindKey {
    pub namespace: &'static str,
    pub name: &'static str,
}

impl UnitKindKey {
    pub const fn new(namespace: &'static str, name: &'static str) -> Self {
        Self { namespace, name }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitKey {
    pub kind: UnitKindKey,
    pub id: u64,
}

impl UnitKey {
    pub const fn new(kind: UnitKindKey, id: u64) -> Self {
        Self { kind, id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitFilterKey {
    pub namespace: &'static str,
    pub name: &'static str,
}

impl UnitFilterKey {
    pub const fn new(namespace: &'static str, name: &'static str) -> Self {
        Self { namespace, name }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitSelector {
    Kind(UnitKindKey),
    Affected(UnitKindKey),
    AffectedArtifact {
        kind: UnitKindKey,
        artifact: ArtifactKey,
        filter: Option<UnitFilterKey>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitOrder {
    Stable,
    SourceOrder,
    DependencyOrder,
    ReverseDependencyOrder,
    AffectedFirst,
    Custom(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassScope {
    Global,
    Unit(UnitKindKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassContext<C> {
    pub current_unit: Option<UnitKey>,
    marker: PhantomData<C>,
}

impl<C> PassContext<C> {
    pub fn new(current_unit: Option<UnitKey>) -> Self {
        Self {
            current_unit,
            marker: PhantomData,
        }
    }
}

pub trait UnitProvider {
    fn units(&self, selector: &UnitSelector, order: UnitOrder) -> Vec<UnitKey>;

    fn parent(&self, _unit: UnitKey) -> Option<UnitKey> {
        None
    }

    fn children(&self, _unit: UnitKey, _kind: Option<UnitKindKey>) -> Vec<UnitKey> {
        Vec::new()
    }
}
