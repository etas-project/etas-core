use crate::storage::sqlite::SqliteOperation;
use crate::{
    HostError, HostErrorCode, HostValue, MemoryCursor, MemoryEntry, MemoryOperation, MemoryQuery,
    MemoryRequest, MemoryResult, StoreRef,
};
use rusqlite::Connection;
mod client;
pub use client::SqliteMemoryClient;
struct MemoryDatabase<'a> {
    connection: &'a mut Connection,
    operation: Option<&'a SqliteOperation>,
    limits: &'a crate::StorageLimits,
}
impl MemoryDatabase<'_> {
    fn execute_operation(&mut self, request: MemoryRequest) -> Result<MemoryResult, HostError> {
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
            MemoryOperation::Scan { cursor, limit } => self.scan(&request.store, cursor, limit),
            MemoryOperation::Query { query, limit } => self.query(&request.store, query, limit),
            MemoryOperation::VectorSearch {
                embedding,
                limit,
                filter,
            } => self.vector_search(&request.store, embedding, limit, filter),
        }
    }
}

mod query;
mod read;
mod receipt;
mod scan;
mod schema;
mod write;

use crate::value::tagged::{
    decode_with_limits as decode_host_value, encode_with_limits as encode_host_value,
};

fn store_path_key(store: &StoreRef) -> Result<String, HostError> {
    serde_json::to_string(&store.path)
        .map_err(|_| HostError::new(HostErrorCode::InvalidRequest, "invalid Store path"))
}
fn sqlite_error(error: rusqlite::Error) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, "SQLite memory error")
        .with_detail("error", error.to_string())
}
