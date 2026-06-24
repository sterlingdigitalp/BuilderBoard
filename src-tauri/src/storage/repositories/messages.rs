use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{
    AppendMessageRequest, MessageCompleteRequest, MessageCreateRequest, MessageCreateResult,
    MessageDto, MessageErrorRequest, MessageStreamUpdateRequest,
};
use crate::storage::repositories::panes::PaneRepository;

const VALID_ROLES: &[&str] = &["user", "assistant", "system", "tool"];
const MUTABLE_ASSISTANT_STATUSES: &[&str] = &["pending", "streaming"];

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

    pub fn get_by_id(connection: &Connection, message_id: &str) -> StorageResult<MessageDto> {
        connection
            .query_row(
                "SELECT id, workspace_id, pane_id, parent_id, role, content, content_type, status,
                        provider_id, account_id, model_id, metadata_json, created_at, updated_at
                 FROM messages
                 WHERE id = ?1",
                [message_id],
                map_message_row,
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StorageError::NotFound(format!("message not found: {message_id}"))
                }
                other => StorageError::from(other),
            })
    }

    pub fn create_conversation_turn(
        connection: &Connection,
        request: MessageCreateRequest,
    ) -> StorageResult<MessageCreateResult> {
        let pane = PaneRepository::get_open_by_id(connection, &request.pane_id)?;
        let now = Utc::now().to_rfc3339();
        let user_message_id = Uuid::new_v4().to_string();
        let assistant_message_id = Uuid::new_v4().to_string();
        let content_type = request.content_type.unwrap_or_else(|| "text".to_string());
        let user_metadata_json = request.metadata_json.unwrap_or_else(|| "{}".to_string());
        let assistant_metadata_json = user_metadata_json.clone();

        let tx = connection.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO messages (
                id, workspace_id, pane_id, role, content, content_type, status,
                provider_id, account_id, model_id, metadata_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, 'user', ?4, ?5, 'complete',
                ?6, ?7, ?8, ?9, ?10, ?11
             )",
            (
                &user_message_id,
                &pane.workspace_id,
                &pane.id,
                &request.content,
                &content_type,
                pane.provider_id.as_deref(),
                pane.account_id.as_deref(),
                pane.model_id.as_deref(),
                &user_metadata_json,
                &now,
                &now,
            ),
        )?;

        tx.execute(
            "INSERT INTO messages (
                id, workspace_id, pane_id, parent_id, role, content, content_type, status,
                provider_id, account_id, model_id, metadata_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, 'assistant', '', ?5, 'pending',
                ?6, ?7, ?8, ?9, ?10, ?11
             )",
            (
                &assistant_message_id,
                &pane.workspace_id,
                &pane.id,
                &user_message_id,
                &content_type,
                pane.provider_id.as_deref(),
                pane.account_id.as_deref(),
                pane.model_id.as_deref(),
                &assistant_metadata_json,
                &now,
                &now,
            ),
        )?;

        tx.commit()?;

        Ok(MessageCreateResult {
            user_message: Self::get_by_id(connection, &user_message_id)?,
            assistant_message: Self::get_by_id(connection, &assistant_message_id)?,
        })
    }

    pub fn stream_update(
        connection: &Connection,
        request: MessageStreamUpdateRequest,
    ) -> StorageResult<MessageDto> {
        let message = Self::get_by_id(connection, &request.message_id)?;
        Self::ensure_mutable_assistant(&message)?;
        Self::append_stream_delta(connection, &request.message_id, &request.delta)?;
        Self::get_by_id(connection, &request.message_id)
    }

    /// Hot-path stream append without pre-read or post-read round trips.
    pub fn append_stream_delta(
        connection: &Connection,
        message_id: &str,
        delta: &str,
    ) -> StorageResult<()> {
        if delta.is_empty() {
            return Ok(());
        }

        let now = Utc::now().to_rfc3339();
        let updated = connection.execute(
            "UPDATE messages
             SET content = content || ?1,
                 status = 'streaming',
                 updated_at = ?2
             WHERE id = ?3
               AND role = 'assistant'
               AND status IN ('pending', 'streaming')",
            (delta, &now, message_id),
        )?;

        if updated == 0 {
            return Err(StorageError::InvalidInput(format!(
                "message cannot accept stream update: {message_id}"
            )));
        }

        Ok(())
    }

    pub fn mark_complete(
        connection: &Connection,
        request: MessageCompleteRequest,
    ) -> StorageResult<MessageDto> {
        let message = Self::get_by_id(connection, &request.message_id)?;
        Self::ensure_mutable_assistant(&message)?;

        let now = Utc::now().to_rfc3339();
        let metadata_json = request
            .metadata_json
            .unwrap_or_else(|| message.metadata_json.clone());

        let updated = if let Some(content) = request.content {
            connection.execute(
                "UPDATE messages
                 SET content = ?1,
                     status = 'complete',
                     token_count_input = ?2,
                     token_count_output = ?3,
                     metadata_json = ?4,
                     completed_at = ?5,
                     updated_at = ?5
                 WHERE id = ?6
                   AND role = 'assistant'
                   AND status IN ('pending', 'streaming')",
                (
                    &content,
                    request.token_count_input,
                    request.token_count_output,
                    &metadata_json,
                    &now,
                    &request.message_id,
                ),
            )?
        } else {
            connection.execute(
                "UPDATE messages
                 SET status = 'complete',
                     token_count_input = ?1,
                     token_count_output = ?2,
                     metadata_json = ?3,
                     completed_at = ?4,
                     updated_at = ?4
                 WHERE id = ?5
                   AND role = 'assistant'
                   AND status IN ('pending', 'streaming')",
                (
                    request.token_count_input,
                    request.token_count_output,
                    &metadata_json,
                    &now,
                    &request.message_id,
                ),
            )?
        };

        if updated == 0 {
            return Err(StorageError::InvalidInput(format!(
                "message cannot be completed: {}",
                request.message_id
            )));
        }

        Self::get_by_id(connection, &request.message_id)
    }

    pub fn mark_error(
        connection: &Connection,
        request: MessageErrorRequest,
    ) -> StorageResult<MessageDto> {
        let message = Self::get_by_id(connection, &request.message_id)?;
        Self::ensure_mutable_assistant(&message)?;

        let now = Utc::now().to_rfc3339();
        let updated = connection.execute(
            "UPDATE messages
             SET status = 'error',
                 error_code = ?1,
                 error_message = ?2,
                 completed_at = ?3,
                 updated_at = ?3
             WHERE id = ?4
               AND role = 'assistant'
               AND status IN ('pending', 'streaming')",
            (
                &request.error_code,
                &request.error_message,
                &now,
                &request.message_id,
            ),
        )?;

        if updated == 0 {
            return Err(StorageError::InvalidInput(format!(
                "message cannot be marked as error: {}",
                request.message_id
            )));
        }

        Self::get_by_id(connection, &request.message_id)
    }

    pub fn append(
        connection: &Connection,
        request: AppendMessageRequest,
    ) -> StorageResult<MessageDto> {
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

        Self::get_by_id(connection, &message_id)
    }

    fn ensure_mutable_assistant(message: &MessageDto) -> StorageResult<()> {
        if message.role != "assistant" {
            return Err(StorageError::InvalidInput(format!(
                "message lifecycle applies to assistant messages only: {}",
                message.id
            )));
        }

        if !MUTABLE_ASSISTANT_STATUSES.contains(&message.status.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "message is not in a mutable state: {} ({})",
                message.id, message.status
            )));
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::error::StorageResult;
    use crate::storage::models::CreatePaneRequest;
    use crate::storage::repositories::panes::PaneRepository;

    fn setup_pane(name: &str) -> StorageResult<(Database, String)> {
        let path = test_database_path(name)?;
        let _ = fs::remove_file(&path);
        let database = Database::initialize_at(path)?;
        let pane_id = database.with_connection(|connection| {
            crate::storage::test_fixtures::seed_test_project(connection, "Messages")?;
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: None,
                    title: Some("Chat".to_string()),
                    sort_order: Some(0),
                },
            )?;
            Ok(pane.id)
        })?;
        Ok((database, pane_id))
    }

    #[test]
    fn create_conversation_turn_persists_user_and_assistant_placeholder() -> StorageResult<()> {
        let (database, pane_id) = setup_pane("message-create-turn.db")?;

        database.with_connection(|connection| {
            let result = MessageRepository::create_conversation_turn(
                connection,
                MessageCreateRequest {
                    pane_id: pane_id.clone(),
                    content: "Hello".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )?;

            assert_eq!(result.user_message.role, "user");
            assert_eq!(result.user_message.content, "Hello");
            assert_eq!(result.user_message.status, "complete");

            assert_eq!(result.assistant_message.role, "assistant");
            assert_eq!(result.assistant_message.content, "");
            assert_eq!(result.assistant_message.status, "pending");
            assert_eq!(
                result.assistant_message.parent_id.as_deref(),
                Some(result.user_message.id.as_str())
            );

            let messages = MessageRepository::list_for_pane(connection, &pane_id)?;
            assert_eq!(messages.len(), 2);
            Ok(())
        })
    }

    #[test]
    fn stream_update_appends_content_and_sets_streaming_status() -> StorageResult<()> {
        let (database, pane_id) = setup_pane("message-stream-update.db")?;

        database.with_connection(|connection| {
            let turn = MessageRepository::create_conversation_turn(
                connection,
                MessageCreateRequest {
                    pane_id,
                    content: "Ping".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )?;

            let first = MessageRepository::stream_update(
                connection,
                MessageStreamUpdateRequest {
                    message_id: turn.assistant_message.id.clone(),
                    delta: "Hel".to_string(),
                },
            )?;
            assert_eq!(first.content, "Hel");
            assert_eq!(first.status, "streaming");

            let second = MessageRepository::stream_update(
                connection,
                MessageStreamUpdateRequest {
                    message_id: turn.assistant_message.id,
                    delta: "lo".to_string(),
                },
            )?;
            assert_eq!(second.content, "Hello");
            assert_eq!(second.status, "streaming");
            Ok(())
        })
    }

    #[test]
    fn mark_complete_finalizes_assistant_message() -> StorageResult<()> {
        let (database, pane_id) = setup_pane("message-complete.db")?;

        database.with_connection(|connection| {
            let turn = MessageRepository::create_conversation_turn(
                connection,
                MessageCreateRequest {
                    pane_id,
                    content: "Question".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )?;

            MessageRepository::stream_update(
                connection,
                MessageStreamUpdateRequest {
                    message_id: turn.assistant_message.id.clone(),
                    delta: "Answer".to_string(),
                },
            )?;

            let completed = MessageRepository::mark_complete(
                connection,
                MessageCompleteRequest {
                    message_id: turn.assistant_message.id,
                    content: None,
                    token_count_input: Some(12),
                    token_count_output: Some(4),
                    metadata_json: Some(r#"{"finish_reason":"stop"}"#.to_string()),
                },
            )?;

            assert_eq!(completed.content, "Answer");
            assert_eq!(completed.status, "complete");
            assert_eq!(completed.metadata_json, r#"{"finish_reason":"stop"}"#);
            Ok(())
        })
    }

    #[test]
    fn mark_error_persists_error_fields() -> StorageResult<()> {
        let (database, pane_id) = setup_pane("message-error.db")?;

        database.with_connection(|connection| {
            let turn = MessageRepository::create_conversation_turn(
                connection,
                MessageCreateRequest {
                    pane_id,
                    content: "Fail me".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )?;

            let errored = MessageRepository::mark_error(
                connection,
                MessageErrorRequest {
                    message_id: turn.assistant_message.id,
                    error_code: "provider_timeout".to_string(),
                    error_message: "Request timed out".to_string(),
                },
            )?;

            assert_eq!(errored.status, "error");
            Ok(())
        })
    }

    #[test]
    fn completed_message_rejects_stream_update() -> StorageResult<()> {
        let (database, pane_id) = setup_pane("message-reject-update.db")?;

        database.with_connection(|connection| {
            let turn = MessageRepository::create_conversation_turn(
                connection,
                MessageCreateRequest {
                    pane_id,
                    content: "Done".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )?;

            MessageRepository::mark_complete(
                connection,
                MessageCompleteRequest {
                    message_id: turn.assistant_message.id.clone(),
                    content: Some("Final".to_string()),
                    token_count_input: None,
                    token_count_output: None,
                    metadata_json: None,
                },
            )?;

            let result = MessageRepository::stream_update(
                connection,
                MessageStreamUpdateRequest {
                    message_id: turn.assistant_message.id,
                    delta: "nope".to_string(),
                },
            );

            assert!(result.is_err());
            Ok(())
        })
    }

    #[test]
    fn conversation_lifecycle_survives_database_reopen() -> StorageResult<()> {
        let path = test_database_path("message-restart-persistence.db")?;
        let _ = fs::remove_file(&path);

        let (pane_id, assistant_id) = {
            let database = Database::initialize_at(path.clone())?;
            database.with_connection(|connection| {
                crate::storage::test_fixtures::seed_test_project(connection, "Restart")?;
                let pane = PaneRepository::create(
                    connection,
                    CreatePaneRequest {
                        workspace_id: None,
                        project_id: None,
                        title: Some("Restart".to_string()),
                        sort_order: Some(0),
                    },
                )?;

                let turn = MessageRepository::create_conversation_turn(
                    connection,
                    MessageCreateRequest {
                        pane_id: pane.id.clone(),
                        content: "Persist this".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )?;

                MessageRepository::stream_update(
                    connection,
                    MessageStreamUpdateRequest {
                        message_id: turn.assistant_message.id.clone(),
                        delta: "Partial ".to_string(),
                    },
                )?;

                MessageRepository::mark_complete(
                    connection,
                    MessageCompleteRequest {
                        message_id: turn.assistant_message.id.clone(),
                        content: None,
                        token_count_input: None,
                        token_count_output: None,
                        metadata_json: None,
                    },
                )?;

                Ok((pane.id, turn.assistant_message.id))
            })?
        };

        let database = Database::initialize_at(path)?;
        database.with_connection(|connection| {
            let messages = MessageRepository::list_for_pane(connection, &pane_id)?;
            assert_eq!(messages.len(), 2);
            assert_eq!(messages[0].content, "Persist this");
            assert_eq!(messages[0].status, "complete");

            let assistant = messages
                .iter()
                .find(|message| message.id == assistant_id)
                .expect("assistant message should exist");
            assert_eq!(assistant.content, "Partial ");
            assert_eq!(assistant.status, "complete");
            Ok(())
        })
    }
}
