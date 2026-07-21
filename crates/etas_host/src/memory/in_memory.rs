use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use crate::{
    HostError, HostErrorCode, HostValue, MemoryClient, MemoryConflict, MemoryCursor, MemoryEntry,
    MemoryOperation, MemoryQuery, MemoryRequest, MemoryResponse, MemoryResult, MemoryVersion,
    MemoryWriteMode, StoreRef, host_value_to_json,
};

#[derive(Clone, Debug, Default)]
pub struct InMemoryMemoryClient {
    stores: Arc<RwLock<BTreeMap<StoreKey, BTreeMap<HostKey, VersionedValue>>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StoreKey {
    region: String,
    path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct HostKey(String);

#[derive(Clone, Debug)]
struct VersionedValue {
    key: HostValue,
    value: HostValue,
    version: u64,
}

impl InMemoryMemoryClient {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MemoryClient for InMemoryMemoryClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<MemoryResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: MemoryRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            let id = request.id;
            let result = self.execute_operation(request);
            Ok(MemoryResponse { id, result })
        })
    }
}

impl InMemoryMemoryClient {
    fn execute_operation(&self, request: MemoryRequest) -> Result<MemoryResult, HostError> {
        match request.operation {
            MemoryOperation::Get { key } => self.get(&request.store, key),
            MemoryOperation::Put {
                key,
                value,
                expected,
                mode,
            } => self.put(&request.store, key, value, expected, mode),
            MemoryOperation::Delete { key, expected } => self.delete(&request.store, key, expected),
            MemoryOperation::Scan { cursor, limit } => self.scan(&request.store, cursor, limit),
            MemoryOperation::Query { query, limit } => self.query(&request.store, query, limit),
            MemoryOperation::VectorSearch {
                embedding,
                limit,
                filter,
            } => self.vector_search(&request.store, embedding, limit, filter),
        }
    }

    fn get(&self, store: &StoreRef, key: HostValue) -> Result<MemoryResult, HostError> {
        let key = canonical_key(key)?;
        let stores = self.stores.read().map_err(lock_error)?;
        let Some(values) = stores.get(&store_key(store)) else {
            return Ok(MemoryResult::None);
        };
        Ok(values
            .get(&key)
            .map_or(MemoryResult::None, |entry| MemoryResult::Value {
                value: entry.value.clone(),
                version: version(entry.version),
            }))
    }

    fn put(
        &self,
        store: &StoreRef,
        key_value: HostValue,
        value: HostValue,
        expected: Option<MemoryVersion>,
        mode: MemoryWriteMode,
    ) -> Result<MemoryResult, HostError> {
        let key = canonical_key(key_value.clone())?;
        let mut stores = self.stores.write().map_err(lock_error)?;
        let values = stores.entry(store_key(store)).or_default();
        if let Some(conflict) = write_conflict_for(values.get(&key), expected.as_ref(), mode)? {
            return Ok(MemoryResult::Conflict(conflict));
        }
        let next = values.get(&key).map_or(1, |entry| entry.version + 1);
        values.insert(
            key,
            VersionedValue {
                key: key_value,
                value,
                version: next,
            },
        );
        Ok(MemoryResult::Written {
            version: version(next),
        })
    }

    fn delete(
        &self,
        store: &StoreRef,
        key_value: HostValue,
        expected: Option<MemoryVersion>,
    ) -> Result<MemoryResult, HostError> {
        let key = canonical_key(key_value)?;
        let mut stores = self.stores.write().map_err(lock_error)?;
        let values = stores.entry(store_key(store)).or_default();
        if let Some(conflict) = conflict_for(values.get(&key), expected.as_ref())? {
            return Ok(MemoryResult::Conflict(conflict));
        }
        let actual = values.remove(&key).map_or(1, |entry| entry.version + 1);
        Ok(MemoryResult::Deleted {
            version: version(actual),
        })
    }

    fn scan(
        &self,
        store: &StoreRef,
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
    ) -> Result<MemoryResult, HostError> {
        let offset = cursor
            .as_ref()
            .map(|cursor| {
                cursor.opaque.parse::<usize>().map_err(|error| {
                    HostError::new(HostErrorCode::InvalidRequest, "invalid memory scan cursor")
                        .with_detail("cursor", &cursor.opaque)
                        .with_detail("error", error.to_string())
                })
            })
            .transpose()?
            .unwrap_or(0);
        let limit = limit.unwrap_or(100).max(1) as usize;
        let stores = self.stores.read().map_err(lock_error)?;
        let entries = stores
            .get(&store_key(store))
            .map(|values| values.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let total = entries.len();
        let entries = entries
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|entry| MemoryEntry {
                key: entry.key,
                value: entry.value,
                version: version(entry.version),
            })
            .collect::<Vec<_>>();
        let next = (offset + entries.len() < total).then(|| MemoryCursor {
            opaque: (offset + entries.len()).to_string(),
        });
        Ok(MemoryResult::Entries {
            entries,
            cursor: next,
        })
    }

    fn query(
        &self,
        store: &StoreRef,
        query: MemoryQuery,
        limit: Option<u32>,
    ) -> Result<MemoryResult, HostError> {
        if !query.order_by.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "in-memory host does not support ordered memory query",
            ));
        }
        if query.predicate.is_none() {
            return self.scan(store, None, limit);
        }
        let limit = limit.unwrap_or(100).max(1) as usize;
        let stores = self.stores.read().map_err(lock_error)?;
        let entries = stores
            .get(&store_key(store))
            .map(|values| values.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                memory_query_matches(&entry.key, &entry.value, query.predicate.as_ref())
            })
            .take(limit)
            .map(|entry| MemoryEntry {
                key: entry.key,
                value: entry.value,
                version: version(entry.version),
            })
            .collect::<Vec<_>>();
        Ok(MemoryResult::Entries {
            entries,
            cursor: None,
        })
    }

    fn vector_search(
        &self,
        store: &StoreRef,
        embedding: Vec<f32>,
        limit: u32,
        filter: Option<HostValue>,
    ) -> Result<MemoryResult, HostError> {
        validate_query_embedding(&embedding)?;
        let limit = limit.max(1) as usize;
        let stores = self.stores.read().map_err(lock_error)?;
        let mut scored = stores
            .get(&store_key(store))
            .map(|values| values.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| memory_query_matches(&entry.key, &entry.value, filter.as_ref()))
            .filter_map(|entry| {
                let candidate = extract_embedding(&entry.value)?;
                let score = cosine_similarity(&embedding, candidate)?;
                Some((score, entry))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .total_cmp(left_score)
                .then_with(|| stable_host_key(&left.key).cmp(&stable_host_key(&right.key)))
        });
        let entries = scored
            .into_iter()
            .take(limit)
            .map(|(_, entry)| MemoryEntry {
                key: entry.key,
                value: entry.value,
                version: version(entry.version),
            })
            .collect();
        Ok(MemoryResult::Entries {
            entries,
            cursor: None,
        })
    }
}

fn memory_query_matches(key: &HostValue, value: &HostValue, predicate: Option<&HostValue>) -> bool {
    let Some(predicate) = predicate else {
        return true;
    };
    match predicate {
        HostValue::String(text) => {
            matches!(key, HostValue::String(key_text) if key_text.contains(text))
                || matches!(value, HostValue::String(value_text) if value_text.contains(text))
        }
        other => key == other || value == other,
    }
}

fn validate_query_embedding(embedding: &[f32]) -> Result<(), HostError> {
    if embedding.is_empty() || embedding.iter().any(|value| !value.is_finite()) {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "memory vector search requires a non-empty finite embedding",
        ));
    }
    Ok(())
}

fn extract_embedding(value: &HostValue) -> Option<&[HostValue]> {
    match value {
        HostValue::List(values) => Some(values),
        HostValue::Record(fields) => fields.iter().find_map(|(name, value)| {
            (name == "embedding").then_some(value).and_then(|value| {
                if let HostValue::List(values) = value {
                    Some(values.as_slice())
                } else {
                    None
                }
            })
        }),
        _ => None,
    }
}

fn cosine_similarity(query: &[f32], candidate: &[HostValue]) -> Option<f32> {
    if query.len() != candidate.len() {
        return None;
    }
    let mut dot = 0.0f32;
    let mut query_norm = 0.0f32;
    let mut candidate_norm = 0.0f32;
    for (left, right) in query.iter().copied().zip(candidate.iter()) {
        let right = host_value_to_f32(right)?;
        dot += left * right;
        query_norm += left * left;
        candidate_norm += right * right;
    }
    if query_norm == 0.0 || candidate_norm == 0.0 {
        return None;
    }
    Some(dot / (query_norm.sqrt() * candidate_norm.sqrt()))
}

fn host_value_to_f32(value: &HostValue) -> Option<f32> {
    match value {
        HostValue::Float(value) if value.is_finite() => Some(*value as f32),
        HostValue::Int(value) => Some(*value as f32),
        HostValue::UInt(value) => Some(*value as f32),
        _ => None,
    }
}

fn write_conflict_for(
    actual: Option<&VersionedValue>,
    expected: Option<&MemoryVersion>,
    mode: MemoryWriteMode,
) -> Result<Option<MemoryConflict>, HostError> {
    if let Some(conflict) = conflict_for(actual, expected)? {
        return Ok(Some(conflict));
    }
    match (mode, actual) {
        (MemoryWriteMode::Insert, Some(entry)) => Ok(Some(MemoryConflict {
            expected: None,
            actual: Some(version(entry.version)),
            current_value: Some(entry.value.clone()),
        })),
        (MemoryWriteMode::Update, None) => Ok(Some(MemoryConflict {
            expected: None,
            actual: None,
            current_value: None,
        })),
        _ => Ok(None),
    }
}

fn store_key(store: &StoreRef) -> StoreKey {
    StoreKey {
        region: store.region.stable_id.clone(),
        path: store.path.clone(),
    }
}

fn canonical_key(value: HostValue) -> Result<HostKey, HostError> {
    Ok(HostKey(host_value_to_json(&value)?.to_string()))
}

fn stable_host_key(value: &HostValue) -> String {
    host_value_to_json(value)
        .map(|value| value.to_string())
        .unwrap_or_else(|_| format!("{value:?}"))
}

fn version(value: u64) -> MemoryVersion {
    MemoryVersion {
        opaque: value.to_string(),
    }
}

fn parse_version(value: &MemoryVersion) -> Result<u64, HostError> {
    value.opaque.parse::<u64>().map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "invalid memory version token",
        )
        .with_detail("version", &value.opaque)
        .with_detail("error", error.to_string())
    })
}

fn conflict_for(
    actual: Option<&VersionedValue>,
    expected: Option<&MemoryVersion>,
) -> Result<Option<MemoryConflict>, HostError> {
    let expected_version = expected.map(parse_version).transpose()?;
    let actual_version = actual.map(|entry| entry.version);
    if expected_version == actual_version || expected.is_none() {
        return Ok(None);
    }
    Ok(Some(MemoryConflict {
        expected: expected.cloned(),
        actual: actual_version.map(version),
        current_value: actual.map(|entry| entry.value.clone()),
    }))
}

fn lock_error(_: std::sync::PoisonError<impl std::fmt::Debug>) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "in-memory memory store lock is poisoned",
    )
}
