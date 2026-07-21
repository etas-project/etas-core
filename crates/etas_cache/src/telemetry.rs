use std::{collections::BTreeMap, time::Duration};

use crate::ArtifactKey;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactTelemetryKey {
    pub namespace: String,
    pub kind: String,
}

impl ArtifactTelemetryKey {
    pub fn new(namespace: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            kind: kind.into(),
        }
    }

    pub fn from_artifact_key(key: &ArtifactKey) -> Self {
        Self::new(key.namespace.as_str(), key.kind.as_str())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactTelemetry {
    pub compute_time: Duration,
    pub compute_count: u64,
    pub serialize_time: Duration,
    pub serialize_count: u64,
    pub deserialize_time: Duration,
    pub deserialize_count: u64,
    pub compressed_bytes: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub skipped_write_count: u64,
    pub eviction_count: u64,
}

impl ArtifactTelemetry {
    fn merge(&mut self, other: &Self) {
        self.compute_time = self.compute_time.saturating_add(other.compute_time);
        self.compute_count = self.compute_count.saturating_add(other.compute_count);
        self.serialize_time = self.serialize_time.saturating_add(other.serialize_time);
        self.serialize_count = self.serialize_count.saturating_add(other.serialize_count);
        self.deserialize_time = self.deserialize_time.saturating_add(other.deserialize_time);
        self.deserialize_count = self
            .deserialize_count
            .saturating_add(other.deserialize_count);
        self.compressed_bytes = self.compressed_bytes.saturating_add(other.compressed_bytes);
        self.hit_count = self.hit_count.saturating_add(other.hit_count);
        self.miss_count = self.miss_count.saturating_add(other.miss_count);
        self.skipped_write_count = self
            .skipped_write_count
            .saturating_add(other.skipped_write_count);
        self.eviction_count = self.eviction_count.saturating_add(other.eviction_count);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CacheTelemetry {
    by_kind: BTreeMap<ArtifactTelemetryKey, ArtifactTelemetry>,
}

impl CacheTelemetry {
    pub fn by_kind(&self) -> &BTreeMap<ArtifactTelemetryKey, ArtifactTelemetry> {
        &self.by_kind
    }

    pub fn artifact_kind(&self, key: &ArtifactKey) -> Option<&ArtifactTelemetry> {
        self.by_kind
            .get(&ArtifactTelemetryKey::from_artifact_key(key))
    }

    pub fn record_compute_time(&mut self, key: &ArtifactKey, duration: Duration) {
        let entry = self.entry_for(key);
        entry.compute_time = entry.compute_time.saturating_add(duration);
        entry.compute_count = entry.compute_count.saturating_add(1);
    }

    pub fn record_serialize_time(&mut self, key: &ArtifactKey, duration: Duration) {
        let entry = self.entry_for(key);
        entry.serialize_time = entry.serialize_time.saturating_add(duration);
        entry.serialize_count = entry.serialize_count.saturating_add(1);
    }

    pub fn record_deserialize_time(&mut self, key: &ArtifactKey, duration: Duration) {
        let entry = self.entry_for(key);
        entry.deserialize_time = entry.deserialize_time.saturating_add(duration);
        entry.deserialize_count = entry.deserialize_count.saturating_add(1);
    }

    pub fn record_compressed_bytes(&mut self, key: &ArtifactKey, bytes: u64) {
        let entry = self.entry_for(key);
        entry.compressed_bytes = entry.compressed_bytes.saturating_add(bytes);
    }

    pub fn record_hit(&mut self, key: &ArtifactKey) {
        let entry = self.entry_for(key);
        entry.hit_count = entry.hit_count.saturating_add(1);
    }

    pub fn record_miss(&mut self, key: &ArtifactKey) {
        let entry = self.entry_for(key);
        entry.miss_count = entry.miss_count.saturating_add(1);
    }

    pub fn record_skipped_write(&mut self, key: &ArtifactKey) {
        let entry = self.entry_for(key);
        entry.skipped_write_count = entry.skipped_write_count.saturating_add(1);
    }

    pub fn record_eviction(&mut self, key: &ArtifactKey) {
        let entry = self.entry_for(key);
        entry.eviction_count = entry.eviction_count.saturating_add(1);
    }

    pub fn merge(&mut self, other: &Self) {
        for (key, value) in other.by_kind() {
            self.by_kind.entry(key.clone()).or_default().merge(value);
        }
    }

    fn entry_for(&mut self, key: &ArtifactKey) -> &mut ArtifactTelemetry {
        self.by_kind
            .entry(ArtifactTelemetryKey::from_artifact_key(key))
            .or_default()
    }
}
