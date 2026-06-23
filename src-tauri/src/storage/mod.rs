pub mod commands;
pub mod db;
pub mod error;
pub mod migrations;
pub mod models;
pub mod repositories;

use tauri::Manager;

use db::Database;

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let database = Database::initialize_default().map_err(|error| {
                tauri::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error.to_string(),
                ))
            })?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::provider_list,
            commands::pane_list,
            commands::pane_create,
            commands::pane_close,
            commands::message_list,
            commands::message_append,
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod integration_tests {
    use std::fs;

    use super::*;
    use crate::storage::error::StorageResult;
    use db::{test_database_path, Database};
    use models::DEFAULT_WORKSPACE_ID;
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
                let pane = PaneRepository::create(
                    conn,
                    models::CreatePaneRequest {
                        workspace_id: None,
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
            let panes = PaneRepository::list_open(conn, None)?;
            assert_eq!(panes.len(), 1);
            assert_eq!(panes[0].id, pane_id);

            let messages = MessageRepository::list_for_pane(conn, &pane_id)?;
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "hello");
            Ok(())
        })
    }

    #[test]
    fn pane_close_soft_deletes_from_open_list() -> StorageResult<()> {
        let path = test_database_path("pane-close.db")?;
        let _ = fs::remove_file(&path);
        let db = Database::initialize_at(path)?;

        db.with_connection(|conn| {
            let pane = PaneRepository::create(
                conn,
                models::CreatePaneRequest {
                    workspace_id: None,
                    title: Some("Temporary".to_string()),
                    sort_order: None,
                },
            )?;
            PaneRepository::close(conn, &pane.id)?;
            let panes = PaneRepository::list_open(conn, None)?;
            assert!(panes.is_empty());
            Ok(())
        })
    }
}
