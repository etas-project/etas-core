use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::{
    HostError, HostErrorCode, HostJsonValue, HostValue, MemoryClient, MemoryConflict, MemoryCursor,
    MemoryEntry, MemoryOperation, MemoryQuery, MemoryRequest, MemoryResponse, MemoryResult,
    MemoryVersion, MemoryWriteMode, StoreRef,
};

#[derive(Clone, Debug)]
pub struct SqliteMemoryClient {
    connection: Arc<Mutex<Connection>>,
    path: PathBuf,
}

#[derive(Clone, Debug)]
struct VersionedValue {
    value: HostValue,
    version: u64,
}

impl SqliteMemoryClient {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, HostError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to create SQLite memory directory",
                )
                .with_detail("path", parent.display().to_string())
                .with_detail("error", error.to_string())
            })?;
        }
        let connection = Connection::open(&path).map_err(sqlite_error)?;
        initialize_schema(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl MemoryClient for SqliteMemoryClient {
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

impl SqliteMemoryClient {
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
        let key_json = encode_host_value(&key)?;
        let store_path = store_path_key(store)?;
        let connection = self.connection.lock().map_err(lock_error)?;
        let entry = select_entry(&connection, store, &store_path, &key_json)?;
        Ok(
            entry.map_or(MemoryResult::None, |entry| MemoryResult::Value {
                value: entry.value,
                version: version(entry.version),
            }),
        )
    }

    fn put(
        &self,
        store: &StoreRef,
        key: HostValue,
        value: HostValue,
        expected: Option<MemoryVersion>,
        mode: MemoryWriteMode,
    ) -> Result<MemoryResult, HostError> {
        let key_json = encode_host_value(&key)?;
        let value_json = encode_host_value(&value)?;
        let store_path = store_path_key(store)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        let current = select_entry(&transaction, store, &store_path, &key_json)?;
        if let Some(conflict) = write_conflict_for(current.as_ref(), expected.as_ref(), mode)? {
            return Ok(MemoryResult::Conflict(conflict));
        }
        let next = current.as_ref().map_or(1, |entry| entry.version + 1);
        transaction
            .execute(
                "INSERT INTO memory_entries
                    (region, path, key_json, value_json, version)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(region, path, key_json) DO UPDATE SET
                    value_json = excluded.value_json,
                    version = excluded.version",
                params![
                    store.region.stable_id,
                    store_path,
                    key_json,
                    value_json,
                    i64::try_from(next).map_err(version_overflow_error)?,
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(MemoryResult::Written {
            version: version(next),
        })
    }

    fn delete(
        &self,
        store: &StoreRef,
        key: HostValue,
        expected: Option<MemoryVersion>,
    ) -> Result<MemoryResult, HostError> {
        let key_json = encode_host_value(&key)?;
        let store_path = store_path_key(store)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        let current = select_entry(&transaction, store, &store_path, &key_json)?;
        if let Some(conflict) = conflict_for(current.as_ref(), expected.as_ref())? {
            return Ok(MemoryResult::Conflict(conflict));
        }
        let next = current.as_ref().map_or(1, |entry| entry.version + 1);
        transaction
            .execute(
                "DELETE FROM memory_entries
                 WHERE region = ?1 AND path = ?2 AND key_json = ?3",
                params![store.region.stable_id, store_path, key_json],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(MemoryResult::Deleted {
            version: version(next),
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
        let store_path = store_path_key(store)?;
        let connection = self.connection.lock().map_err(lock_error)?;
        let total = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM memory_entries
                 WHERE region = ?1 AND path = ?2",
                params![store.region.stable_id, store_path],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
        let mut statement = connection
            .prepare(
                "SELECT key_json, value_json, version
                 FROM memory_entries
                 WHERE region = ?1 AND path = ?2
                 ORDER BY key_json
                 LIMIT ?3 OFFSET ?4",
            )
            .map_err(sqlite_error)?;
        let entries = statement
            .query_map(
                params![
                    store.region.stable_id,
                    store_path,
                    i64::try_from(limit).map_err(version_overflow_error)?,
                    i64::try_from(offset).map_err(version_overflow_error)?,
                ],
                |row| {
                    let key_json: String = row.get(0)?;
                    let value_json: String = row.get(1)?;
                    let version: i64 = row.get(2)?;
                    Ok((key_json, value_json, version))
                },
            )
            .map_err(sqlite_error)?
            .map(|row| {
                let (key_json, value_json, stored_version) = row.map_err(sqlite_error)?;
                Ok(MemoryEntry {
                    key: decode_host_value(&key_json)?,
                    value: decode_host_value(&value_json)?,
                    version: version(u64::try_from(stored_version).map_err(version_parse_error)?),
                })
            })
            .collect::<Result<Vec<_>, HostError>>()?;
        let total = usize::try_from(total).map_err(version_parse_error)?;
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
                "SQLite memory host does not support ordered memory query",
            ));
        }
        if query.predicate.is_none() {
            return self.scan(store, None, limit);
        }
        let limit = limit.unwrap_or(100).max(1) as usize;
        let store_path = store_path_key(store)?;
        let connection = self.connection.lock().map_err(lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT key_json, value_json, version
                 FROM memory_entries
                 WHERE region = ?1 AND path = ?2
                 ORDER BY key_json",
            )
            .map_err(sqlite_error)?;
        let entries = statement
            .query_map(params![store.region.stable_id, store_path], |row| {
                let key_json: String = row.get(0)?;
                let value_json: String = row.get(1)?;
                let version: i64 = row.get(2)?;
                Ok((key_json, value_json, version))
            })
            .map_err(sqlite_error)?
            .map(|row| {
                let (key_json, value_json, stored_version) = row.map_err(sqlite_error)?;
                Ok((
                    decode_host_value(&key_json)?,
                    decode_host_value(&value_json)?,
                    stored_version,
                ))
            })
            .filter_map(|entry| match entry {
                Ok((key, value, stored_version))
                    if memory_query_matches(&key, &value, query.predicate.as_ref()) =>
                {
                    match u64::try_from(stored_version).map_err(version_parse_error) {
                        Ok(stored_version) => Some(Ok(MemoryEntry {
                            key,
                            value,
                            version: version(stored_version),
                        })),
                        Err(error) => Some(Err(error)),
                    }
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .take(limit)
            .collect::<Result<Vec<_>, HostError>>()?;
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
        let store_path = store_path_key(store)?;
        let connection = self.connection.lock().map_err(lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT key_json, value_json, version
                 FROM memory_entries
                 WHERE region = ?1 AND path = ?2
                 ORDER BY key_json",
            )
            .map_err(sqlite_error)?;
        let mut scored = statement
            .query_map(params![store.region.stable_id, store_path], |row| {
                let key_json: String = row.get(0)?;
                let value_json: String = row.get(1)?;
                let version: i64 = row.get(2)?;
                Ok((key_json, value_json, version))
            })
            .map_err(sqlite_error)?
            .map(|row| {
                let (key_json, value_json, stored_version) = row.map_err(sqlite_error)?;
                Ok((
                    decode_host_value(&key_json)?,
                    decode_host_value(&value_json)?,
                    stored_version,
                ))
            })
            .filter_map(|entry| match entry {
                Ok((key, value, stored_version))
                    if memory_query_matches(&key, &value, filter.as_ref()) =>
                {
                    let score = extract_embedding(&value)
                        .and_then(|candidate| cosine_similarity(&embedding, candidate));
                    score.map(|score| Ok((score, key, value, stored_version)))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, HostError>>()?;
        scored.sort_by(
            |(left_score, left_key, _, _), (right_score, right_key, _, _)| {
                right_score.total_cmp(left_score).then_with(|| {
                    stable_host_key(left_key)
                        .unwrap_or_else(|_| format!("{left_key:?}"))
                        .cmp(
                            &stable_host_key(right_key)
                                .unwrap_or_else(|_| format!("{right_key:?}")),
                        )
                })
            },
        );
        let entries = scored
            .into_iter()
            .take(limit)
            .map(|(_, key, value, stored_version)| {
                Ok(MemoryEntry {
                    key,
                    value,
                    version: version(u64::try_from(stored_version).map_err(version_parse_error)?),
                })
            })
            .collect::<Result<Vec<_>, HostError>>()?;
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

fn initialize_schema(connection: &Connection) -> Result<(), HostError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS memory_entries (
                region TEXT NOT NULL,
                path TEXT NOT NULL,
                key_json TEXT NOT NULL,
                value_json TEXT NOT NULL,
                version INTEGER NOT NULL,
                PRIMARY KEY(region, path, key_json)
             );",
        )
        .map(|_| ())
        .map_err(sqlite_error)
}

fn select_entry(
    connection: &Connection,
    store: &StoreRef,
    store_path: &str,
    key_json: &str,
) -> Result<Option<VersionedValue>, HostError> {
    connection
        .query_row(
            "SELECT value_json, version
             FROM memory_entries
             WHERE region = ?1 AND path = ?2 AND key_json = ?3",
            params![store.region.stable_id, store_path, key_json],
            |row| {
                let value_json: String = row.get(0)?;
                let version: i64 = row.get(1)?;
                Ok((value_json, version))
            },
        )
        .optional()
        .map_err(sqlite_error)?
        .map(|(value_json, stored_version)| {
            Ok(VersionedValue {
                value: decode_host_value(&value_json)?,
                version: u64::try_from(stored_version).map_err(version_parse_error)?,
            })
        })
        .transpose()
}

fn store_path_key(store: &StoreRef) -> Result<String, HostError> {
    serde_json::to_string(&store.path).map_err(json_error)
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

fn encode_host_value(value: &HostValue) -> Result<String, HostError> {
    tagged_host_value_json(value)
        .and_then(|value| serde_json::to_string(&value).map_err(json_error))
}

fn stable_host_key(value: &HostValue) -> Result<String, HostError> {
    encode_host_value(value)
}

fn decode_host_value(value: &str) -> Result<HostValue, HostError> {
    let value = serde_json::from_str::<Value>(value).map_err(json_error)?;
    tagged_host_value_from_json(&value)
}

fn tagged_host_value_json(value: &HostValue) -> Result<Value, HostError> {
    Ok(match value {
        HostValue::Unit => json!({ "kind": "unit" }),
        HostValue::Bool(value) => json!({ "kind": "bool", "value": value }),
        HostValue::Int(value) => json!({ "kind": "int", "value": value.to_string() }),
        HostValue::UInt(value) => json!({ "kind": "uint", "value": value.to_string() }),
        HostValue::Float(value) if value.is_finite() => {
            json!({ "kind": "float", "value": value })
        }
        HostValue::Float(_) => {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "non-finite floating-point memory value cannot be stored",
            ));
        }
        HostValue::String(value) => json!({ "kind": "string", "value": value }),
        HostValue::Bytes(value) => json!({ "kind": "bytes", "value": value }),
        HostValue::List(values) => json!({
            "kind": "list",
            "items": values.iter().map(tagged_host_value_json).collect::<Result<Vec<_>, _>>()?,
        }),
        HostValue::Map(entries) => json!({
            "kind": "map",
            "entries": entries.iter().map(|(key, value)| {
                Ok(json!({
                    "key": tagged_host_value_json(key)?,
                    "value": tagged_host_value_json(value)?,
                }))
            }).collect::<Result<Vec<_>, HostError>>()?,
        }),
        HostValue::Record(fields) => json!({
            "kind": "record",
            "fields": fields.iter().map(|(name, value)| {
                Ok(json!({
                    "name": name,
                    "value": tagged_host_value_json(value)?,
                }))
            }).collect::<Result<Vec<_>, HostError>>()?,
        }),
        HostValue::Variant { name, fields } => json!({
            "kind": "variant",
            "name": name,
            "fields": fields.iter().map(tagged_host_value_json).collect::<Result<Vec<_>, _>>()?,
        }),
        HostValue::Json(value) => json!({
            "kind": "json",
            "value": host_json_value_json(value),
        }),
    })
}

fn tagged_host_value_from_json(value: &Value) -> Result<HostValue, HostError> {
    let kind = string_field(value, "kind")?;
    match kind {
        "unit" => Ok(HostValue::Unit),
        "bool" => bool_field(value, "value").map(HostValue::Bool),
        "int" => string_field(value, "value")?
            .parse::<i128>()
            .map(HostValue::Int)
            .map_err(json_error),
        "uint" => string_field(value, "value")?
            .parse::<u128>()
            .map(HostValue::UInt)
            .map_err(json_error),
        "float" => number_field(value, "value").and_then(|number| {
            if number.is_finite() {
                Ok(HostValue::Float(number))
            } else {
                Err(HostError::new(
                    HostErrorCode::SchemaMismatch,
                    "stored memory float is not finite",
                ))
            }
        }),
        "string" => string_field(value, "value").map(|value| HostValue::String(value.to_owned())),
        "bytes" => value
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| schema_error("stored memory bytes field is missing value array"))?
            .iter()
            .map(|byte| {
                byte.as_u64()
                    .and_then(|byte| u8::try_from(byte).ok())
                    .ok_or_else(|| schema_error("stored memory byte is outside u8 range"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(HostValue::Bytes),
        "list" => array_field(value, "items")?
            .iter()
            .map(tagged_host_value_from_json)
            .collect::<Result<Vec<_>, _>>()
            .map(HostValue::List),
        "map" => array_field(value, "entries")?
            .iter()
            .map(|entry| {
                Ok((
                    tagged_host_value_from_json(required_field(entry, "key")?)?,
                    tagged_host_value_from_json(required_field(entry, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>, HostError>>()
            .map(HostValue::Map),
        "record" => array_field(value, "fields")?
            .iter()
            .map(|field| {
                Ok((
                    string_field(field, "name")?.to_owned(),
                    tagged_host_value_from_json(required_field(field, "value")?)?,
                ))
            })
            .collect::<Result<Vec<_>, HostError>>()
            .map(HostValue::Record),
        "variant" => Ok(HostValue::Variant {
            name: string_field(value, "name")?.to_owned(),
            fields: array_field(value, "fields")?
                .iter()
                .map(tagged_host_value_from_json)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        "json" => host_json_value_from_json(required_field(value, "value")?).map(HostValue::Json),
        other => Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "stored memory value has unknown kind",
        )
        .with_detail("kind", other)),
    }
}

fn host_json_value_json(value: &HostJsonValue) -> Value {
    match value {
        HostJsonValue::Null => Value::Null,
        HostJsonValue::Bool(value) => Value::Bool(*value),
        HostJsonValue::Number(value) => json!(value),
        HostJsonValue::String(value) => Value::String(value.clone()),
        HostJsonValue::Array(values) => {
            Value::Array(values.iter().map(host_json_value_json).collect())
        }
        HostJsonValue::Object(entries) => Value::Object(
            entries
                .iter()
                .map(|(name, value)| (name.clone(), host_json_value_json(value)))
                .collect(),
        ),
    }
}

fn host_json_value_from_json(value: &Value) -> Result<HostJsonValue, HostError> {
    Ok(match value {
        Value::Null => HostJsonValue::Null,
        Value::Bool(value) => HostJsonValue::Bool(*value),
        Value::Number(value) => HostJsonValue::Number(
            value
                .as_f64()
                .ok_or_else(|| schema_error("stored JSON number cannot be represented as f64"))?,
        ),
        Value::String(value) => HostJsonValue::String(value.clone()),
        Value::Array(values) => HostJsonValue::Array(
            values
                .iter()
                .map(host_json_value_from_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(entries) => HostJsonValue::Object(
            entries
                .iter()
                .map(|(name, value)| Ok((name.clone(), host_json_value_from_json(value)?)))
                .collect::<Result<Vec<_>, HostError>>()?,
        ),
    })
}

fn required_field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, HostError> {
    value
        .get(name)
        .ok_or_else(|| schema_error(format!("stored memory value is missing `{name}` field")))
}

fn string_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, HostError> {
    required_field(value, name)?
        .as_str()
        .ok_or_else(|| schema_error(format!("stored memory `{name}` field must be a string")))
}

fn bool_field(value: &Value, name: &str) -> Result<bool, HostError> {
    required_field(value, name)?
        .as_bool()
        .ok_or_else(|| schema_error(format!("stored memory `{name}` field must be a bool")))
}

fn number_field(value: &Value, name: &str) -> Result<f64, HostError> {
    required_field(value, name)?
        .as_f64()
        .ok_or_else(|| schema_error(format!("stored memory `{name}` field must be a number")))
}

fn array_field<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], HostError> {
    required_field(value, name)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| schema_error(format!("stored memory `{name}` field must be an array")))
}

fn sqlite_error(error: rusqlite::Error) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, "SQLite memory error")
        .with_detail("error", error.to_string())
}

fn json_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "SQLite memory JSON codec error",
    )
    .with_detail("error", error.to_string())
}

fn schema_error(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::SchemaMismatch, message)
}

fn version_overflow_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "memory version is outside SQLite integer range",
    )
    .with_detail("error", error.to_string())
}

fn version_parse_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "stored SQLite memory version is invalid",
    )
    .with_detail("error", error.to_string())
}

fn lock_error(_: std::sync::PoisonError<impl std::fmt::Debug>) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "SQLite memory store lock is poisoned",
    )
}
