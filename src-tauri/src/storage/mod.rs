pub mod commands;
pub mod db;
pub mod error;
pub mod migrations;
pub mod models;
pub mod pane_project_migration;
pub mod repositories;
#[cfg(test)]
pub mod test_fixtures;

use std::sync::Arc;

use tauri::Manager;

use crate::auth::{CredentialService, OAuthService};
use crate::project_scope_cache::ProjectScopeCache;
use crate::stream_persistence::StreamPersistenceService;

use db::Database;

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let database = Arc::new(Database::initialize_default().map_err(|error| {
                tauri::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error.to_string(),
                ))
            })?);
            let credentials = Arc::new(CredentialService::keychain());
            let oauth = Arc::new(OAuthService::production());
            let stream_persistence = Arc::new(StreamPersistenceService::new(Arc::clone(&database)));
            let project_scope_cache = Arc::new(ProjectScopeCache::new());
            app.manage(database);
            app.manage(credentials);
            app.manage(oauth);
            app.manage(stream_persistence);
            app.manage(project_scope_cache);

            // Register default tools for Phase 9A Tool Runtime
            if let Err(e) = crate::execution::tools::registry::register_default_tools() {
                eprintln!(
                    "[tool_registry] Warning: failed to register default tools: {}",
                    e
                );
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::provider_list,
            commands::engine_list,
            commands::builder_list,
            commands::resolve_execution,
            commands::capability_list,
            commands::workspace_create,
            commands::workspace_list,
            commands::workspace_switch,
            commands::workspace_get_active,
            commands::pane_list,
            commands::pane_create,
            commands::pane_close,
            commands::pane_set_project,
            commands::message_list,
            commands::message_append,
            commands::message_create,
            commands::message_stream_update,
            commands::message_complete,
            commands::message_error,
            commands::stream_chat,
            commands::runtime_probe_ping,
            commands::account_create_api_key,
            commands::account_list,
            commands::account_disconnect,
            commands::account_get_status,
            crate::auth::commands::oauth_start,
            crate::auth::commands::oauth_cancel,
            crate::filesystem_tools::commands::filesystem_set_approved_root,
            crate::filesystem_tools::commands::filesystem_get_approved_root,
            crate::filesystem_tools::commands::filesystem_list_directory,
            crate::filesystem_tools::commands::filesystem_read_file,
            crate::filesystem_tools::commands::filesystem_search_files,
            crate::filesystem_tools::commands::filesystem_find_files,
            crate::projects::commands::project_list,
            crate::projects::commands::project_get_active,
            crate::projects::commands::project_create_from_folder,
            crate::projects::commands::project_switch,
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod integration_tests {
    use std::fs;

    use super::*;
    use crate::storage::error::StorageResult;
    use crate::storage::test_fixtures::seed_test_project;
    use db::{test_database_path, Database};
    use models::{DEFAULT_WORKSPACE_ID, SHELL_WORKSPACE_ID};
    use repositories::messages::MessageRepository;
    use repositories::panes::PaneRepository;
    use repositories::providers::ProviderRepository;
    use repositories::workspaces::WorkspaceRepository;

    #[test]
    fn fresh_install_creates_schema_and_seeds() -> StorageResult<()> {
        let path = test_database_path("fresh-install.db")?;
        let _ = fs::remove_file(&path);

        let db = Database::initialize_at(path)?;
        db.with_connection(|conn| {
            assert_eq!(ProviderRepository::count(conn)?, 3);
            let workspace = WorkspaceRepository::get_default(conn)?;
            assert_eq!(workspace.id, DEFAULT_WORKSPACE_ID);
            Ok(())
        })
    }

    #[test]
    fn pane_and_message_persistence_survives_reopen() -> StorageResult<()> {
        let path = test_database_path("restart-persistence.db")?;
        let _ = fs::remove_file(&path);

        let pane_id = {
            let db = Database::initialize_at(path.clone())?;
            db.with_connection(|conn| {
                seed_test_project(conn, "Persist")?;
                let pane = PaneRepository::create(
                    conn,
                    models::CreatePaneRequest {
                        workspace_id: None,
                        project_id: None,
                        title: Some("Persist Me".to_string()),
                        sort_order: Some(0),
                    },
                )?;
                MessageRepository::append(
                    conn,
                    models::AppendMessageRequest {
                        pane_id: pane.id.clone(),
                        role: "user".to_string(),
                        content: "hello".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )?;
                Ok(pane.id)
            })?
        };

        let db = Database::initialize_at(path)?;
        db.with_connection(|conn| {
            let panes = PaneRepository::list_shell_open(conn)?;
            assert_eq!(panes.len(), 1);
            assert_eq!(panes[0].id, pane_id);

            let messages = MessageRepository::list_for_pane(conn, &pane_id)?;
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "hello");
            Ok(())
        })
    }

    #[test]
    fn message_lifecycle_persists_across_restart() -> StorageResult<()> {
        use models::{
            MessageCompleteRequest, MessageCreateRequest, MessageErrorRequest,
            MessageStreamUpdateRequest,
        };

        let path = test_database_path("message-lifecycle-restart.db")?;
        let _ = fs::remove_file(&path);

        let (pane_id, errored_assistant_id) = {
            let db = Database::initialize_at(path.clone())?;
            db.with_connection(|conn| {
                seed_test_project(conn, "Lifecycle")?;
                let pane = PaneRepository::create(
                    conn,
                    models::CreatePaneRequest {
                        workspace_id: None,
                        project_id: None,
                        title: Some("Lifecycle".to_string()),
                        sort_order: Some(0),
                    },
                )?;

                let completed_turn = MessageRepository::create_conversation_turn(
                    conn,
                    MessageCreateRequest {
                        pane_id: pane.id.clone(),
                        content: "Complete path".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )?;
                MessageRepository::stream_update(
                    conn,
                    MessageStreamUpdateRequest {
                        message_id: completed_turn.assistant_message.id.clone(),
                        delta: "Done".to_string(),
                    },
                )?;
                MessageRepository::mark_complete(
                    conn,
                    MessageCompleteRequest {
                        message_id: completed_turn.assistant_message.id,
                        content: None,
                        token_count_input: None,
                        token_count_output: None,
                        metadata_json: None,
                    },
                )?;

                let error_turn = MessageRepository::create_conversation_turn(
                    conn,
                    MessageCreateRequest {
                        pane_id: pane.id.clone(),
                        content: "Error path".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )?;
                MessageRepository::mark_error(
                    conn,
                    MessageErrorRequest {
                        message_id: error_turn.assistant_message.id.clone(),
                        error_code: "rate_limited".to_string(),
                        error_message: "Too many requests".to_string(),
                    },
                )?;

                Ok((pane.id, error_turn.assistant_message.id))
            })?
        };

        let db = Database::initialize_at(path)?;
        db.with_connection(|conn| {
            let messages = MessageRepository::list_for_pane(conn, &pane_id)?;
            assert_eq!(messages.len(), 4);

            let completed_assistant = messages
                .iter()
                .find(|message| message.role == "assistant" && message.status == "complete")
                .expect("completed assistant");
            assert_eq!(completed_assistant.content, "Done");

            let errored_assistant = messages
                .iter()
                .find(|message| message.id == errored_assistant_id)
                .expect("errored assistant");
            assert_eq!(errored_assistant.status, "error");
            Ok(())
        })
    }

    #[test]
    fn pane_close_soft_deletes_from_open_list() -> StorageResult<()> {
        let path = test_database_path("pane-close.db")?;
        let _ = fs::remove_file(&path);
        let db = Database::initialize_at(path)?;

        db.with_connection(|conn| {
            seed_test_project(conn, "Close")?;
            let pane = PaneRepository::create(
                conn,
                models::CreatePaneRequest {
                    workspace_id: None,
                    project_id: None,
                    title: Some("Temporary".to_string()),
                    sort_order: None,
                },
            )?;
            PaneRepository::close(conn, &pane.id)?;
            let panes = PaneRepository::list_shell_open(conn)?;
            assert!(panes.is_empty());
            Ok(())
        })
    }

    #[test]
    fn shell_workspace_lists_all_project_bound_panes() -> StorageResult<()> {
        let path = test_database_path("shell-pane-list.db")?;
        let _ = fs::remove_file(&path);

        let (project_a_id, project_b_id, pane_a_id, pane_b_id) = {
            let db = Database::initialize_at(path.clone())?;
            db.with_connection(|conn| {
                let project_a_id = seed_test_project(conn, "ProjectA")?;
                let pane_a = PaneRepository::create(
                    conn,
                    models::CreatePaneRequest {
                        workspace_id: None,
                        project_id: Some(project_a_id.clone()),
                        title: Some("Pane A".to_string()),
                        sort_order: Some(0),
                    },
                )?;
                MessageRepository::append(
                    conn,
                    models::AppendMessageRequest {
                        pane_id: pane_a.id.clone(),
                        role: "user".to_string(),
                        content: "Message A".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )?;

                let project_b_id = seed_test_project(conn, "ProjectB")?;
                let pane_b = PaneRepository::create(
                    conn,
                    models::CreatePaneRequest {
                        workspace_id: None,
                        project_id: Some(project_b_id.clone()),
                        title: Some("Pane B".to_string()),
                        sort_order: Some(1),
                    },
                )?;
                MessageRepository::append(
                    conn,
                    models::AppendMessageRequest {
                        pane_id: pane_b.id.clone(),
                        role: "user".to_string(),
                        content: "Message B".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )?;

                let panes = PaneRepository::list_shell_open(conn)?;
                assert_eq!(panes.len(), 2);
                assert!(panes.iter().any(|pane| pane.id == pane_a.id));
                assert!(panes.iter().any(|pane| pane.id == pane_b.id));
                assert!(panes
                    .iter()
                    .all(|pane| pane.workspace_id == SHELL_WORKSPACE_ID));

                let messages_a = MessageRepository::list_for_pane(conn, &pane_a.id)?;
                let messages_b = MessageRepository::list_for_pane(conn, &pane_b.id)?;
                assert_eq!(messages_a[0].workspace_id, SHELL_WORKSPACE_ID);
                assert_eq!(messages_a[0].content, "Message A");
                assert_eq!(messages_b[0].workspace_id, SHELL_WORKSPACE_ID);
                assert_eq!(messages_b[0].content, "Message B");

                Ok((project_a_id, project_b_id, pane_a.id, pane_b.id))
            })?
        };

        let db = Database::initialize_at(path)?;
        db.with_connection(|conn| {
            let panes = PaneRepository::list_shell_open(conn)?;
            assert_eq!(panes.len(), 2);
            assert!(panes.iter().any(|pane| pane.id == pane_a_id));
            assert!(panes.iter().any(|pane| pane.id == pane_b_id));
            let pane_a = panes
                .iter()
                .find(|pane| pane.id == pane_a_id)
                .expect("pane a");
            let pane_b = panes
                .iter()
                .find(|pane| pane.id == pane_b_id)
                .expect("pane b");
            assert_eq!(pane_a.project_id.as_deref(), Some(project_a_id.as_str()));
            assert_eq!(pane_b.project_id.as_deref(), Some(project_b_id.as_str()));
            Ok(())
        })
    }
}
