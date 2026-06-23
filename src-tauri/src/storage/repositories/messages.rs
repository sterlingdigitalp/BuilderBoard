use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{AppendMessageRequest, MessageDto};
use crate::storage::repositories::panes::PaneRepository;

const VALID_ROLES: &[&str] = &["user", "assistant", "system", "tool"];

pub struct MessageRepository;

impl MessageRepository {
    pub fn list_for_pane(connection: &Connection, pane_id: &str) -> StorageResult<Vec<MessageDto>> {
        PaneRepository::get_open_by_id(connection, pane_id)?;

        let mut statement = connection.prepare(
            "SELECT id, workspace_id, pane_id, parent_id, role, content, content_type, status,
                    provider_id, account_id, model_id, metadata_json, created_at, updated_at
             FROM messages
             WHERE pane_id = ?1
             ORDER BY created_at",
        )?;

        let messages = statement
            .query_map([pane_id], map_message_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    pub fn append(connection: &Connection, request: AppendMessageRequest) -> StorageResult<MessageDto> {
        if !VALID_ROLES.contains(&request.role.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "invalid message role: {}",
                request.role
            )));
        }

        let pane = PaneRepository::get_open_by_id(connection, &request.pane_id)?;
        let now = Utc::now().to_rfc3339();
        let message_id = Uuid::new_v4().to_string();
        let content_type = request.content_type.unwrap_or_else(|| "text".to_string());
        let metadata_json = request.metadata_json.unwrap_or_else(|| "{}".to_string());

        connection.execute(
            "INSERT INTO messages (
                id, workspace_id, pane_id, role, content, content_type, status,
                provider_id, account_id, model_id, metadata_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, 'complete',
                ?7, ?8, ?9, ?10, ?11, ?12
             )",
            (
                &message_id,
                &pane.workspace_id,
                &pane.id,
                &request.role,
                &request.content,
                &content_type,
                pane.provider_id.as_deref(),
                pane.account_id.as_deref(),
                pane.model_id.as_deref(),
                &metadata_json,
                &now,
                &now,
            ),
        )?;

        connection
            .query_row(
                "SELECT id, workspace_id, pane_id, parent_id, role, content, content_type, status,
                        provider_id, account_id, model_id, metadata_json, created_at, updated_at
                 FROM messages
                 WHERE id = ?1",
                [&message_id],
                map_message_row,
            )
            .map_err(StorageError::from)
    }
}

fn map_message_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageDto> {
    Ok(MessageDto {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        pane_id: row.get(2)?,
        parent_id: row.get(3)?,
        role: row.get(4)?,
        content: row.get(5)?,
        content_type: row.get(6)?,
        status: row.get(7)?,
        provider_id: row.get(8)?,
        account_id: row.get(9)?,
        model_id: row.get(10)?,
        metadata_json: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}