use std::{fmt, io};

#[derive(Debug)]
pub enum CacheError {
    Io(io::Error),
    Sqlite(rusqlite::Error),
    Unavailable(String),
    InvalidEnvelope(String),
    Integrity(String),
}

pub type CacheResult<T> = Result<T, CacheError>;

impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::Io(error) => write!(formatter, "cache I/O error: {error}"),
            CacheError::Sqlite(error) => write!(formatter, "cache SQLite error: {error}"),
            CacheError::Unavailable(message) => write!(formatter, "cache unavailable: {message}"),
            CacheError::InvalidEnvelope(message) => {
                write!(formatter, "invalid cache envelope: {message}")
            }
            CacheError::Integrity(message) => write!(formatter, "cache integrity error: {message}"),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<io::Error> for CacheError {
    fn from(error: io::Error) -> Self {
        CacheError::Io(error)
    }
}

impl From<rusqlite::Error> for CacheError {
    fn from(error: rusqlite::Error) -> Self {
        CacheError::Sqlite(error)
    }
}
