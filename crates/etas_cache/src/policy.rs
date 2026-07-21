use std::collections::BTreeMap;

use crate::ArtifactKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachePolicy {
    pub eviction: EvictionPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvictionPolicy {
    Never,
    MaxArtifacts(usize),
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            eviction: EvictionPolicy::Never,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum CachePriority {
    Low,
    #[default]
    Normal,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskCacheBudgetPolicy {
    pub max_project_bytes: Option<u64>,
    pub namespace_budgets: BTreeMap<String, u64>,
    pub max_payload_bytes: Option<u64>,
    pub default_priority: CachePriority,
    pub namespace_priorities: BTreeMap<String, CachePriority>,
    pub kind_priorities: BTreeMap<(String, String), CachePriority>,
}

impl Default for DiskCacheBudgetPolicy {
    fn default() -> Self {
        Self {
            max_project_bytes: None,
            namespace_budgets: BTreeMap::new(),
            max_payload_bytes: None,
            default_priority: CachePriority::Normal,
            namespace_priorities: BTreeMap::new(),
            kind_priorities: BTreeMap::new(),
        }
    }
}

impl DiskCacheBudgetPolicy {
    pub fn with_max_project_bytes(mut self, bytes: u64) -> Self {
        self.max_project_bytes = Some(bytes);
        self
    }

    pub fn with_namespace_budget(mut self, namespace: impl Into<String>, bytes: u64) -> Self {
        self.namespace_budgets.insert(namespace.into(), bytes);
        self
    }

    pub fn with_max_payload_bytes(mut self, bytes: u64) -> Self {
        self.max_payload_bytes = Some(bytes);
        self
    }

    pub fn with_namespace_priority(
        mut self,
        namespace: impl Into<String>,
        priority: CachePriority,
    ) -> Self {
        self.namespace_priorities.insert(namespace.into(), priority);
        self
    }

    pub fn with_kind_priority(
        mut self,
        namespace: impl Into<String>,
        kind: impl Into<String>,
        priority: CachePriority,
    ) -> Self {
        self.kind_priorities
            .insert((namespace.into(), kind.into()), priority);
        self
    }

    pub fn namespace_budget(&self, namespace: &str) -> Option<u64> {
        self.namespace_budgets.get(namespace).copied()
    }

    pub fn priority_for(&self, key: &ArtifactKey) -> CachePriority {
        let namespace = key.namespace.as_str();
        let kind = key.kind.as_str();
        self.kind_priorities
            .get(&(namespace.to_owned(), kind.to_owned()))
            .copied()
            .or_else(|| self.namespace_priorities.get(namespace).copied())
            .unwrap_or(self.default_priority)
    }
}
