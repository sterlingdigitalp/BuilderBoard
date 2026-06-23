use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use super::error::{StorageError, StorageResult};
use super::migrations::MigrationRunner;
use super::repositories::providers::ProviderRepository;
use super::repositories::workspaces::WorkspaceRepository;

pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn initialize_at(path: PathBuf) -> StorageResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let existed_before_open = path.exists();
        let connection = Connection::open(&path)?;
        configure_connection(&connection)?;

        let runner = MigrationRunner::new();
        runner.run(&connection, &path, existed_before_open)?;
        verify_seeds(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    pub fn initialize_default() -> StorageResult<Self> {
        Self::initialize_at(default_database_path()?)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Migration("database lock poisoned".to_string()))?;
        operation(&connection)
    }
}

fn configure_connection(connection: &Connection) -> StorageResult<()> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )?;
    Ok(())
}

fn verify_seeds(connection: &Connection) -> StorageResult<()> {
    WorkspaceRepository::get_default(connection)?;
    let provider_count = ProviderRepository::count(connection)?;
    if provider_count != 3 {
        return Err(StorageError::Migration(format!(
            "expected 3 seeded providers, found {provider_count}"
        )));
    }
    Ok(())
}

pub fn default_database_path() -> StorageResult<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| {
        StorageError::InvalidInput("could not resolve application data directory".to_string())
    })?;
    Ok(base.join("com.builderboard.app").join("builderboard.db"))
}

#[cfg(test)]
pub fn test_database_path(name: &str) -> StorageResult<PathBuf> {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base)?;
    Ok(base.join(name))
}
