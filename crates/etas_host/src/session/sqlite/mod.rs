use crate::storage::sqlite::SqliteOperation;
use crate::{
    ContextPolicy, HostError, HostErrorCode, RetentionPolicy, SessionConfig, SessionCursor,
    SessionMessage, SessionMessageRole, SessionOperation, SessionRef, SessionResult,
    SessionSummary,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
mod client;
mod context;
pub use client::SqliteSessionClient;
struct SessionDatabase<'a> {
    connection: &'a mut Connection,
    operation: Option<&'a SqliteOperation>,
    limits: &'a crate::StorageLimits,
}
impl SessionDatabase<'_> {
    fn execute_operation(
        &mut self,
        operation: SessionOperation,
    ) -> Result<SessionResult, HostError> {
        match operation {
            SessionOperation::Resolve { config } => self.resolve(config),
            SessionOperation::Append { message } => self.append(message),
            SessionOperation::Load {
                session,
                context,
                cursor,
                limit,
            } => self.load(session, context, cursor, limit),
        }
    }
}
fn invalid_request(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, message)
}

fn sqlite_error(error: rusqlite::Error) -> HostError {
    if let rusqlite::Error::FromSqlConversionFailure(_, _, ref source) = error
        && let Some(error) = source.downcast_ref::<HostError>()
    {
        return error.clone();
    }
    HostError::new(HostErrorCode::ProviderUnavailable, "SQLite session error")
        .with_detail("error", error.to_string())
}

fn json_error(error: serde_json::Error) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "invalid SQLite session JSON")
        .with_detail("error", error.to_string())
}

fn count_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "session count cannot be represented",
    )
    .with_detail("error", error.to_string())
}

mod append;
mod codec;
mod fence;
mod history;
mod maintenance;
mod messages;
mod paging;
mod receipt;
mod schema;
use codec::*;
use messages::*;
use schema::*;
