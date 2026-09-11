use crate::storage::version::{StoreGeneration, next_revision, scope_identity};
use crate::value::tagged::encode_with_limits as canonical_key;
use crate::{
    HostError, HostErrorCode, HostValue, MemoryClient, MemoryOperation, MemoryRequest,
    MemoryResponse, MemoryResult, MemoryVersion, StoreRef, WriteCondition,
};
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

mod read;
mod write;

#[derive(Clone, Debug, Default)]
pub struct InMemoryMemoryClient {
    executor: crate::storage::volatile::VolatileExecutor,
    stores: Arc<RwLock<BTreeMap<StoreKey, MemoryStore>>>,
    limits: crate::StorageLimits,
    receipts: Arc<std::sync::Mutex<BTreeMap<(StoreKey, String), super::MemoryConfirmedOutcome>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StoreKey {
    region: String,
    path: Vec<String>,
}

#[derive(Debug)]
struct MemoryStore {
    generation: StoreGeneration,
    revision: i64,
    entries: BTreeMap<String, VersionedValue>,
}
#[derive(Clone, Debug)]
struct VersionedValue {
    key: HostValue,
    value: HostValue,
    version: MemoryVersion,
}

impl MemoryStore {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            generation: StoreGeneration::new()?,
            revision: 0,
            entries: BTreeMap::new(),
        })
    }
    fn allocate(&mut self, store: &StoreRef) -> Result<MemoryVersion, HostError> {
        let next = next_revision(self.revision)?;
        let version = MemoryVersion::issue(
            &scope_identity("volatile", &store.region.stable_id, &store.path),
            &self.generation,
            next,
        )?;
        self.revision = next;
        Ok(version)
    }
}

impl InMemoryMemoryClient {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_limits(limits: crate::StorageLimits) -> Result<Self, HostError> {
        limits.validate()?;
        Ok(Self {
            executor: crate::storage::volatile::VolatileExecutor::new(limits.clone()),
            limits,
            ..Self::default()
        })
    }
    pub async fn execute_scoped(
        &self,
        request: MemoryRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<MemoryResponse, HostError> {
        self.dispatch(request, Some(operation)).await
    }

    async fn dispatch(
        &self,
        request: MemoryRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<MemoryResponse, HostError> {
        let bytes = request.storage_size(&self.limits)?;
        let client = self.clone();
        self.executor
            .execute(
                operation,
                request.id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |context| {
                    context.signal().check()?;
                    let id = request.id;
                    let result = client.execute_operation(request);
                    Ok(MemoryResponse { id, result })
                },
            )
            .await
            .map_err(crate::execution::DispatchError::into_host_error)
    }

    fn execute_operation(&self, request: MemoryRequest) -> Result<MemoryResult, HostError> {
        request.budget.check_time()?;
        match request.operation {
            MemoryOperation::Get { key } => self.get(&request.store, key),
            MemoryOperation::Put {
                key,
                value,
                condition,
            } => self.put(&request.store, key, value, condition),
            MemoryOperation::Delete { key, condition } => {
                self.delete(&request.store, key, condition)
            }
            MemoryOperation::Scan { cursor, limit } => {
                self.scan(&request.store, cursor, limit, &request.budget)
            }
            MemoryOperation::Query { query, limit } => {
                self.query(&request.store, query, limit, &request.budget)
            }
            MemoryOperation::VectorSearch {
                embedding,
                limit,
                filter,
            } => self.vector_search(&request.store, embedding, limit, filter, &request.budget),
        }
    }

    fn get(&self, store: &StoreRef, key: HostValue) -> Result<MemoryResult, HostError> {
        self.limits.value_size(&key)?;
        let key = canonical_key(&key, &self.limits)?;
        let stores = self.stores.read().map_err(lock_error)?;
        let entry = stores
            .get(&store_key(store))
            .and_then(|store| store.entries.get(&key));
        let Some(entry) = entry else {
            return Ok(MemoryResult::None);
        };
        super::selection::validate_value_result(&entry.value, &entry.version, &self.limits)?;
        Ok(MemoryResult::Value {
            value: entry.value.clone(),
            version: entry.version.clone(),
        })
    }

    fn put(
        &self,
        store: &StoreRef,
        key_value: HostValue,
        value: HostValue,
        condition: WriteCondition,
    ) -> Result<MemoryResult, HostError> {
        self.limits.value_size(&key_value)?;
        self.limits.value_size(&value)?;
        let key = canonical_key(&key_value, &self.limits)?;
        canonical_key(&value, &self.limits)?;
        let mut stores = self.stores.write().map_err(lock_error)?;
        let identity = store_key(store);
        let actual = stores
            .get(&identity)
            .and_then(|store| store.entries.get(&key))
            .map(|entry| entry.version.clone());
        if !condition.is_satisfied_by(actual.as_ref()) {
            return Ok(conflict(condition, actual));
        }
        let state = match stores.entry(identity) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => entry.insert(MemoryStore::new()?),
        };
        let version = state.allocate(store)?;
        state.entries.insert(
            key,
            VersionedValue {
                key: key_value,
                value,
                version: version.clone(),
            },
        );
        Ok(MemoryResult::Written { version })
    }

    fn delete(
        &self,
        store: &StoreRef,
        key_value: HostValue,
        condition: WriteCondition,
    ) -> Result<MemoryResult, HostError> {
        self.limits.value_size(&key_value)?;
        let key = canonical_key(&key_value, &self.limits)?;
        let mut stores = self.stores.write().map_err(lock_error)?;
        let state = stores.get_mut(&store_key(store));
        let actual = state
            .as_ref()
            .and_then(|store| store.entries.get(&key))
            .map(|entry| entry.version.clone());
        if !condition.is_satisfied_by(actual.as_ref()) {
            return Ok(conflict(condition, actual));
        }
        let Some(state) = state.filter(|_| actual.is_some()) else {
            return Ok(MemoryResult::Unchanged);
        };
        let version = state.allocate(store)?;
        state.entries.remove(&key);
        Ok(MemoryResult::Deleted { version })
    }
}

impl MemoryClient for InMemoryMemoryClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<MemoryResponse, HostError>> + Send + 'a>>;
    fn execute(&self, request: MemoryRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.dispatch(request, None))
    }
    type WriteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<super::MemoryWriteResponse, HostError>> + Send + 'a>>;
    fn write(&self, request: super::MemoryWriteRequest) -> Self::WriteFuture<'_> {
        Box::pin(self.dispatch_write(request, None))
    }
}

fn store_key(store: &StoreRef) -> StoreKey {
    StoreKey {
        region: store.region.stable_id.clone(),
        path: store.path.clone(),
    }
}
fn conflict(condition: WriteCondition, actual: Option<MemoryVersion>) -> MemoryResult {
    MemoryResult::Conflict(crate::MemoryConflict {
        expected: condition.expected_version().cloned(),
        actual,
        current_value: None,
    })
}
fn lock_error<T>(_: std::sync::PoisonError<T>) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "in-memory store lock is poisoned",
    )
}
