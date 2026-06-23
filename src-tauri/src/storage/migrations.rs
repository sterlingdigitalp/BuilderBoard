use std::fs;
use std::path::Path;

use chrono::Utc;
use rusqlite::Connection;

use super::error::{StorageError, StorageResult};

const MIGRATION_0001: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/0001_initial_schema.sql"));
const MIGRATION_0002: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/0002_accounts_is_default.sql"));
const MIGRATION_0003: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/0003_google_oauth_config.sql"));

#[cfg(test)]
pub(crate) const MIGRATION_0001_FOR_TEST: &str = MIGRATION_0001;
#[cfg(test)]
pub(crate) const MIGRATIONS_FOR_TEST: &str = concat!(
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/0001_initial_schema.sql")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/0002_accounts_is_default.sql")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/0003_google_oauth_config.sql")),
);

pub struct Migration {
    pub version: &'static str,
    pub sql: &'static str,
}

pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self {
            migrations: vec![
                Migration {
                    version: "0001_initial_schema",
                    sql: MIGRATION_0001,
                },
                Migration {
                    version: "0002_accounts_is_default",
                    sql: MIGRATION_0002,
                },
                Migration {
                    version: "0003_google_oauth_config",
                    sql: MIGRATION_0003,
                },
            ],
        }
    }

    pub fn run(
        &self,
        connection: &Connection,
        database_path: &Path,
        existed_before_open: bool,
    ) -> StorageResult<()> {
        self.ensure_ledger_table(connection)?;

        let mut pending = Vec::new();
        for migration in &self.migrations {
            if !self.is_applied(connection, migration.version)? {
                pending.push(migration);
            }
        }

        if pending.is_empty() {
            return Ok(());
        }

        if existed_before_open {
            self.backup_database(database_path)?;
        }

        for migration in pending {
            self.apply_migration(connection, migration)?;
        }

        Ok(())
    }

    fn ensure_ledger_table(&self, connection: &Connection) -> StorageResult<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    fn is_applied(&self, connection: &Connection, version: &str) -> StorageResult<bool> {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            [version],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn apply_migration(&self, connection: &Connection, migration: &Migration) -> StorageResult<()> {
        let applied_at = Utc::now().to_rfc3339();

        let tx = connection
            .unchecked_transaction()
            .map_err(StorageError::Database)?;

        tx.execute_batch(migration.sql).map_err(|err| {
            StorageError::Migration(format!(
                "failed to apply migration {}: {err}",
                migration.version
            ))
        })?;

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            (migration.version, applied_at),
        )
        .map_err(|err| {
            StorageError::Migration(format!(
                "failed to record migration {}: {err}",
                migration.version
            ))
        })?;

        tx.commit().map_err(StorageError::Database)?;
        Ok(())
    }

    fn backup_database(&self, database_path: &Path) -> StorageResult<()> {
        let Some(parent) = database_path.parent() else {
            return Ok(());
        };

        let backups_dir = parent.join("backups");
        fs::create_dir_all(&backups_dir)?;

        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup_name = format!("builderboard.db.{timestamp}.bak");
        let backup_path = backups_dir.join(backup_name);

        fs::copy(database_path, backup_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::models::DEFAULT_WORKSPACE_ID;
    use crate::storage::repositories::providers::ProviderRepository;
    use crate::storage::repositories::workspaces::WorkspaceRepository;

    #[test]
    fn migration_is_idempotent() -> StorageResult<()> {
        let path = test_database_path("migration-idempotent.db")?;
        let _ = fs::remove_file(&path);

        let db = Database::initialize_at(path.clone())?;
        let provider_count = db.with_connection(|conn| ProviderRepository::count(conn))?;
        assert_eq!(provider_count, 3);

        drop(db);
        let db = Database::initialize_at(path)?;
        let provider_count = db.with_connection(|conn| ProviderRepository::count(conn))?;
        assert_eq!(provider_count, 3);

        let workspace = db.with_connection(|conn| WorkspaceRepository::get_default(conn))?;
        assert_eq!(workspace.id, DEFAULT_WORKSPACE_ID);

        Ok(())
    }
}
