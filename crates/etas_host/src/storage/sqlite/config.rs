use crate::{HostError, HostErrorCode, StorageLimits};
use rusqlite::{Connection, limits::Limit};
use std::{path::Path, time::Duration};

const INITIALIZATION_WAIT: Duration = Duration::from_secs(5);
const OPERATION_WAIT: Duration = Duration::from_millis(100);

pub(crate) fn open_durable(
    path: &Path,
    limits: &StorageLimits,
    migrate: impl FnOnce(&mut Connection) -> Result<(), HostError>,
) -> Result<Connection, HostError> {
    limits.validate()?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "cannot create SQLite storage directory",
            )
        })?;
    }
    let mut connection = Connection::open(path).map_err(config_error)?;
    configure_row_limit(&connection, limits)?;
    connection
        .busy_timeout(INITIALIZATION_WAIT)
        .map_err(config_error)?;
    connection
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))
        .map_err(config_error)?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(config_error)?;
    verify_durability(&connection)?;
    migrate(&mut connection)?;
    // Domain migrations cannot silently weaken the shared durability contract.
    verify_durability(&connection)?;
    Ok(connection)
}

pub(super) fn configure_worker(
    connection: &Connection,
    limits: &StorageLimits,
) -> Result<(), HostError> {
    configure_row_limit(connection, limits)?;
    connection
        .busy_timeout(OPERATION_WAIT)
        .map_err(config_error)
}

fn configure_row_limit(connection: &Connection, limits: &StorageLimits) -> Result<(), HostError> {
    // Bound SQLite's own row/BLOB allocation as well as the domain decoders.
    // Preserve a stricter preconfigured/native limit; never raise it on admission.
    let available = scratch_bytes(limits)?.min(limits.max_pending_bytes);
    let ceiling = i32::try_from(available).unwrap_or(i32::MAX);
    let effective = ceiling.min(connection.limit(Limit::SQLITE_LIMIT_LENGTH));
    if effective <= 0 {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "SQLite storage row limit must be positive",
        ));
    }
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, effective);
    if connection.limit(Limit::SQLITE_LIMIT_LENGTH) != effective {
        return Err(HostError::new(
            HostErrorCode::ProviderUnavailable,
            "SQLite storage row limit was not applied",
        ));
    }
    Ok(())
}

pub(super) fn scratch_bytes(limits: &StorageLimits) -> Result<usize, HostError> {
    limits
        .max_value_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(limits.max_result_bytes.checked_mul(3)?))
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::BudgetExceeded,
                "storage scratch reservation overflow",
            )
        })
}

fn verify_durability(connection: &Connection) -> Result<(), HostError> {
    let journal: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(config_error)?;
    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(config_error)?;
    if !journal.eq_ignore_ascii_case("wal") || synchronous != 2 {
        return Err(HostError::new(
            HostErrorCode::ProviderUnavailable,
            "SQLite durable storage requires WAL and synchronous=FULL",
        ));
    }
    Ok(())
}

fn config_error(error: rusqlite::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "SQLite storage configuration failed",
    )
    .with_detail("error", error.to_string())
}

#[cfg(test)]
mod tests;
