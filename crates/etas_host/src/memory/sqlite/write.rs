use super::{MemoryDatabase, schema, sqlite_error, store_path_key};
use crate::memory::MemoryMutation;
use crate::{HostError, MemoryConflict, MemoryResult, MemoryVersion, StoreRef, WriteCondition};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl MemoryDatabase<'_> {
    pub(super) fn put(
        &mut self,
        store: &StoreRef,
        key: crate::HostValue,
        value: crate::HostValue,
        condition: WriteCondition,
    ) -> Result<MemoryResult, HostError> {
        self.legacy_mutate(
            store,
            MemoryMutation::Put {
                key,
                value,
                condition,
            },
        )
    }
    pub(super) fn delete(
        &mut self,
        store: &StoreRef,
        key: crate::HostValue,
        condition: WriteCondition,
    ) -> Result<MemoryResult, HostError> {
        self.legacy_mutate(store, MemoryMutation::Delete { key, condition })
    }
    fn legacy_mutate(
        &mut self,
        store: &StoreRef,
        mutation: MemoryMutation,
    ) -> Result<MemoryResult, HostError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;
        let result = apply_mutation(&tx, store, mutation, self.limits)?;
        if let Some(operation) = self.operation {
            operation.check()?;
        }
        tx.commit().map_err(sqlite_error)?;
        Ok(result)
    }
}

pub(super) fn apply_mutation(
    tx: &rusqlite::Connection,
    store: &StoreRef,
    mutation: MemoryMutation,
    limits: &crate::StorageLimits,
) -> Result<MemoryResult, HostError> {
    let encode = |v| crate::value::tagged::encode_with_limits(v, limits);
    let (key, value, condition) = match mutation {
        MemoryMutation::Put {
            key,
            value,
            condition,
        } => (encode(&key)?, Some(encode(&value)?), condition),
        MemoryMutation::Delete { key, condition } => (encode(&key)?, None, condition),
    };
    let path = store_path_key(store)?;
    let state = schema::state(tx, store, &path)?;
    let actual = state
        .as_ref()
        .map(|state| current_version(tx, store, &path, &key, state))
        .transpose()?
        .flatten();
    if !condition.is_satisfied_by(actual.as_ref()) {
        return Ok(conflict(condition, actual));
    }
    if value.is_none() && actual.is_none() {
        return Ok(MemoryResult::Unchanged);
    }
    let mut state = match state {
        Some(state) => state,
        None => schema::ensure_store(tx, store, &path)?,
    };
    let version = schema::allocate(tx, store, &path, &mut state)?;
    if let Some(value) = value {
        tx.execute("INSERT INTO memory_entries VALUES (?1,?2,?3,?4,?5)
            ON CONFLICT(region,path,key_json) DO UPDATE SET value_json=excluded.value_json, version=excluded.version",
            params![store.region.stable_id,path,key,value,state.revision]).map_err(sqlite_error)?;
        Ok(MemoryResult::Written { version })
    } else {
        tx.execute(
            "DELETE FROM memory_entries WHERE region=?1 AND path=?2 AND key_json=?3",
            params![store.region.stable_id, path, key],
        )
        .map_err(sqlite_error)?;
        Ok(MemoryResult::Deleted { version })
    }
}

fn current_version(
    connection: &rusqlite::Connection,
    store: &StoreRef,
    path: &str,
    key: &str,
    state: &schema::StoreState,
) -> Result<Option<MemoryVersion>, HostError> {
    connection
        .query_row(
            "SELECT version FROM memory_entries WHERE region=?1 AND path=?2 AND key_json=?3",
            params![store.region.stable_id, path, key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(sqlite_error)?
        .map(|revision| state.version(revision))
        .transpose()
}

pub(super) fn conflict(condition: WriteCondition, actual: Option<MemoryVersion>) -> MemoryResult {
    MemoryResult::Conflict(MemoryConflict {
        expected: condition.expected_version().cloned(),
        actual,
        current_value: None,
    })
}
