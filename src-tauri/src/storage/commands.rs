use tauri::State;

use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::{
    AppendMessageRequest, CreatePaneRequest, MessageDto, PaneDto, ProviderDto,
};
use crate::storage::repositories::messages::MessageRepository;
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::providers::ProviderRepository;

#[tauri::command]
pub fn provider_list(database: State<'_, Database>) -> Result<Vec<ProviderDto>, String> {
    provider_list_from_database(database.inner())
}

pub fn provider_list_from_database(database: &Database) -> Result<Vec<ProviderDto>, String> {
    database
        .with_connection(ProviderRepository::list_enabled)
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_list(
    database: State<'_, Database>,
    workspace_id: Option<String>,
) -> Result<Vec<PaneDto>, String> {
    database
        .with_connection(|connection| {
            PaneRepository::list_open(connection, workspace_id.as_deref())
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_create(
    database: State<'_, Database>,
    workspace_id: Option<String>,
    title: Option<String>,
    sort_order: Option<i32>,
) -> Result<PaneDto, String> {
    database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id,
                    title,
                    sort_order,
                },
            )
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_close(database: State<'_, Database>, pane_id: String) -> Result<(), String> {
    database
        .with_connection(|connection| PaneRepository::close(connection, &pane_id))
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_list(database: State<'_, Database>, pane_id: String) -> Result<Vec<MessageDto>, String> {
    database
        .with_connection(|connection| MessageRepository::list_for_pane(connection, &pane_id))
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_append(
    database: State<'_, Database>,
    pane_id: String,
    role: String,
    content: String,
    content_type: Option<String>,
    metadata_json: Option<String>,
) -> Result<MessageDto, String> {
    database
        .with_connection(|connection| {
            MessageRepository::append(
                connection,
                AppendMessageRequest {
                    pane_id,
                    role,
                    content,
                    content_type,
                    metadata_json,
                },
            )
        })
        .map_err(format_storage_error)
}

fn format_storage_error(error: StorageError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::provider_list_from_database;
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::error::StorageResult;

    #[test]
    fn provider_list_returns_seeded_providers() -> StorageResult<()> {
        let path = test_database_path("provider-list-command.db")?;
        let _ = fs::remove_file(&path);
        let db = Database::initialize_at(path)?;

        let providers = provider_list_from_database(&db).expect("provider_list should succeed");
        let provider_ids: Vec<_> = providers.iter().map(|provider| provider.id.as_str()).collect();

        assert_eq!(providers.len(), 3);
        assert!(provider_ids.contains(&"anthropic"));
        assert!(provider_ids.contains(&"openai"));
        assert!(provider_ids.contains(&"google"));
        Ok(())
    }
}
