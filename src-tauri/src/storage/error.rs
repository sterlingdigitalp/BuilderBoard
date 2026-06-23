use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Database(rusqlite::Error),
    Migration(String),
    NotFound(String),
    InvalidInput(String),
    Io(std::io::Error),
    Keychain(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(err) => write!(f, "database error: {err}"),
            Self::Migration(msg) => write!(f, "migration error: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Keychain(msg) => write!(f, "keychain error: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidInput(format!("json error: {value}"))
    }
}

pub type StorageResult<T> = Result<T, StorageError>;
