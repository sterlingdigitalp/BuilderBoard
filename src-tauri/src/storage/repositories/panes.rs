use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{CreatePaneRequest, PaneDto};
use crate::storage::repositories::workspaces::WorkspaceRepository;

pub struct PaneRepository;

impl PaneRepository {
    pub fn list_open(
        connection: &Connection,
        workspace_id: Option<&str>,
    ) -> StorageResult<Vec<PaneDto>> {
        let resolved_workspace_id =
            WorkspaceRepository::resolve_workspace_id(connection, workspace_id)?;

        let mut statement = connection.prepare(
            "SELECT id, workspace_id, title, role_label, sort_order, width_ratio, height_ratio,
                    provider_id, account_id, model_id, status, layout_json, metadata_json,
                    created_at, updated_at
             FROM panes
             WHERE workspace_id = ?1 AND closed_at IS NULL
             ORDER BY sort_order, created_at",
        )?;

        let panes = statement
            .query_map([&resolved_workspace_id], map_pane_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(panes)
    }

    pub fn create(connection: &Connection, request: CreatePaneRequest) -> StorageResult<PaneDto> {
        let workspace_id =
            WorkspaceRepository::resolve_workspace_id(connection, request.workspace_id.as_deref())?;

        let next_sort_order = match request.sort_order {
            Some(sort_order) => sort_order,
            None => {
                let max_sort_order: Option<i32> = connection
                    .query_row(
                        "SELECT MAX(sort_order) FROM panes WHERE workspace_id = ?1 AND closed_at IS NULL",
                        [&workspace_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                max_sort_order.unwrap_or(-1) + 1
            }
        };

        let now = Utc::now().to_rfc3339();
        let pane_id = Uuid::new_v4().to_string();
        let title = request.title.unwrap_or_else(|| "New Pane".to_string());

        connection.execute(
            "INSERT INTO panes (
                id, workspace_id, title, sort_order, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'idle', ?5, ?6)",
            (&pane_id, &workspace_id, &title, next_sort_order, &now, &now),
        )?;

        Self::get_by_id(connection, &pane_id)
    }

    pub fn close(connection: &Connection, pane_id: &str) -> StorageResult<()> {
        let now = Utc::now().to_rfc3339();
        let updated = connection.execute(
            "UPDATE panes
             SET closed_at = ?1, updated_at = ?1, status = 'idle'
             WHERE id = ?2 AND closed_at IS NULL",
            (&now, pane_id),
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!(
                "open pane {pane_id} not found"
            )));
        }

        Ok(())
    }

    pub fn get_by_id(connection: &Connection, pane_id: &str) -> StorageResult<PaneDto> {
        connection
            .query_row(
                "SELECT id, workspace_id, title, role_label, sort_order, width_ratio, height_ratio,
                        provider_id, account_id, model_id, status, layout_json, metadata_json,
                        created_at, updated_at
                 FROM panes
                 WHERE id = ?1",
                [pane_id],
                map_pane_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("pane {pane_id} not found")))
    }

    pub fn get_open_by_id(connection: &Connection, pane_id: &str) -> StorageResult<PaneDto> {
        connection
            .query_row(
                "SELECT id, workspace_id, title, role_label, sort_order, width_ratio, height_ratio,
                        provider_id, account_id, model_id, status, layout_json, metadata_json,
                        created_at, updated_at
                 FROM panes
                 WHERE id = ?1 AND closed_at IS NULL",
                [pane_id],
                map_pane_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("open pane {pane_id} not found")))
    }
}

fn map_pane_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PaneDto> {
    Ok(PaneDto {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        role_label: row.get(3)?,
        sort_order: row.get(4)?,
        width_ratio: row.get(5)?,
        height_ratio: row.get(6)?,
        provider_id: row.get(7)?,
        account_id: row.get(8)?,
        model_id: row.get(9)?,
        status: row.get(10)?,
        layout_json: row.get(11)?,
        metadata_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}
