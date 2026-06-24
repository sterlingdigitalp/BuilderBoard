use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::WorkspaceDto;

const ACTIVE_WORKSPACE_KEY: &str = "active_workspace_id";

pub struct WorkspaceRepository;

impl WorkspaceRepository {
    pub fn create(connection: &Connection, name: &str) -> StorageResult<WorkspaceDto> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidInput(
                "workspace name cannot be empty".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let workspace_id = Uuid::new_v4().to_string();
        let slug = unique_slug(connection, name)?;

        connection.execute(
            "INSERT INTO workspaces (
                id, name, slug, is_default, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 0, ?4, ?5)",
            (&workspace_id, name, &slug, &now, &now),
        )?;

        Self::get_by_id(connection, &workspace_id)
    }

    pub fn list_active(connection: &Connection) -> StorageResult<Vec<WorkspaceDto>> {
        let mut statement = connection.prepare(
            "SELECT id, name, slug, is_default, layout_json, metadata_json, created_at, updated_at
             FROM workspaces
             WHERE archived_at IS NULL
             ORDER BY is_default DESC, created_at, name",
        )?;

        let workspaces = statement
            .query_map([], map_workspace_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(workspaces)
    }

    pub fn get_default(connection: &Connection) -> StorageResult<WorkspaceDto> {
        connection
            .query_row(
                "SELECT id, name, slug, is_default, layout_json, metadata_json, created_at, updated_at
                 FROM workspaces
                 WHERE is_default = 1 AND archived_at IS NULL
                 LIMIT 1",
                [],
                map_workspace_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound("default workspace not found".to_string()))
    }

    pub fn get_active(connection: &Connection) -> StorageResult<WorkspaceDto> {
        let active_workspace_id: Option<String> = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                [ACTIVE_WORKSPACE_KEY],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(active_workspace_id) = active_workspace_id {
            match Self::get_by_id(connection, &active_workspace_id) {
                Ok(workspace) => return Ok(workspace),
                Err(StorageError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }

        let workspace = Self::get_default(connection)?;
        Self::switch_active(connection, &workspace.id)
    }

    pub fn switch_active(
        connection: &Connection,
        workspace_id: &str,
    ) -> StorageResult<WorkspaceDto> {
        let workspace = Self::get_by_id(connection, workspace_id)?;
        let now = Utc::now().to_rfc3339();

        connection.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            (ACTIVE_WORKSPACE_KEY, &workspace.id, &now),
        )?;

        Ok(workspace)
    }

    pub fn get_by_id(connection: &Connection, workspace_id: &str) -> StorageResult<WorkspaceDto> {
        connection
            .query_row(
                "SELECT id, name, slug, is_default, layout_json, metadata_json, created_at, updated_at
                 FROM workspaces
                 WHERE id = ?1 AND archived_at IS NULL",
                [workspace_id],
                map_workspace_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("workspace {workspace_id} not found")))
    }

    pub fn resolve_workspace_id(
        connection: &Connection,
        workspace_id: Option<&str>,
    ) -> StorageResult<String> {
        match workspace_id {
            Some(id) => {
                Self::get_by_id(connection, id)?;
                Ok(id.to_string())
            }
            None => Ok(Self::get_active(connection)?.id),
        }
    }
}

fn unique_slug(connection: &Connection, name: &str) -> StorageResult<String> {
    let base = slug_base(name);
    let mut slug = base.clone();
    let mut suffix = 2;

    while slug_exists(connection, &slug)? {
        slug = format!("{base}-{suffix}");
        suffix += 1;
    }

    Ok(slug)
}

fn slug_base(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "workspace".to_string()
    } else {
        slug
    }
}

fn slug_exists(connection: &Connection, slug: &str) -> StorageResult<bool> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM workspaces WHERE slug = ?1",
        [slug],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn map_workspace_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceDto> {
    Ok(WorkspaceDto {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        is_default: row.get::<_, i64>(3)? == 1,
        layout_json: row.get(4)?,
        metadata_json: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::DEFAULT_WORKSPACE_ID;

    #[test]
    fn default_workspace_is_seeded() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATION_0001_FOR_TEST)?;
        let workspace = WorkspaceRepository::get_default(&conn)?;
        assert_eq!(workspace.id, DEFAULT_WORKSPACE_ID);
        assert!(workspace.is_default);
        Ok(())
    }

    #[test]
    fn workspace_create_list_switch_and_active_restore() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;

        let default = WorkspaceRepository::get_active(&conn)?;
        assert_eq!(default.id, DEFAULT_WORKSPACE_ID);

        let first = WorkspaceRepository::create(&conn, "Client Work")?;
        let duplicate = WorkspaceRepository::create(&conn, "Client Work")?;
        assert_eq!(first.slug, "client-work");
        assert_eq!(duplicate.slug, "client-work-2");

        let workspaces = WorkspaceRepository::list_active(&conn)?;
        assert_eq!(workspaces.len(), 3);

        let active = WorkspaceRepository::switch_active(&conn, &first.id)?;
        assert_eq!(active.id, first.id);
        assert_eq!(WorkspaceRepository::get_active(&conn)?.id, first.id);

        Ok(())
    }
}
