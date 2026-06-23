use rusqlite::{Connection, OptionalExtension};

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::WorkspaceDto;

pub struct WorkspaceRepository;

impl WorkspaceRepository {
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
            None => Ok(Self::get_default(connection)?.id),
        }
    }
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
}