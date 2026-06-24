use rusqlite::Connection;

use crate::projects::metadata::{parse_metadata, APPROVED_ROOT_KEY};
use crate::projects::repository::ProjectRepository;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::DEFAULT_WORKSPACE_ID;
use crate::storage::repositories::workspaces::WorkspaceRepository;

const BACKFILL_SETTING_KEY: &str = "pane_project_backfill_v1";

pub fn run_after_migrations(connection: &Connection) -> StorageResult<()> {
    if !pane_project_column_exists(connection)? {
        return Ok(());
    }

    if backfill_already_done(connection)? {
        return Ok(());
    }

    backfill_project_ids(connection)?;
    rehome_to_shell_workspace(connection)?;
    mark_backfill_done(connection)?;
    Ok(())
}

fn pane_project_column_exists(connection: &Connection) -> StorageResult<bool> {
    let mut statement = connection.prepare("PRAGMA table_info(panes)")?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names.iter().any(|name| name == "project_id"))
}

fn backfill_already_done(connection: &Connection) -> StorageResult<bool> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM app_settings WHERE key = ?1",
            [BACKFILL_SETTING_KEY],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(count > 0)
}

fn mark_backfill_done(connection: &Connection) -> StorageResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO app_settings (key, value, updated_at)
         VALUES (?1, 'done', ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        (BACKFILL_SETTING_KEY, now),
    )?;
    Ok(())
}

fn backfill_project_ids(connection: &Connection) -> StorageResult<()> {
    let root_to_project = build_root_to_project_map(connection)?;

    let mut statement = connection.prepare(
        "SELECT id, workspace_id
         FROM panes
         WHERE closed_at IS NULL AND project_id IS NULL",
    )?;

    let panes = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if panes.is_empty() {
        return Ok(());
    }

    let focused_project_id = resolve_focused_project_id(connection, &root_to_project)?;

    for (pane_id, workspace_id) in panes {
        let workspace = WorkspaceRepository::get_by_id(connection, &workspace_id)?;
        let project_id = if ProjectRepository::is_project_metadata(workspace.metadata_json.as_deref())
        {
            workspace_id.clone()
        } else {
            let workspace_root = read_workspace_approved_root(workspace.metadata_json.as_deref());
            workspace_root
                .as_deref()
                .and_then(|root| root_to_project.get(root).cloned())
                .unwrap_or_else(|| focused_project_id.clone())
        };

        ProjectRepository::get_by_id(connection, &project_id)?;
        connection.execute(
            "UPDATE panes SET project_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            (&project_id, &pane_id),
        )?;
    }

    let remaining: i64 = connection.query_row(
        "SELECT COUNT(*) FROM panes WHERE closed_at IS NULL AND project_id IS NULL",
        [],
        |row| row.get(0),
    )?;
    if remaining > 0 {
        return Err(StorageError::Migration(format!(
            "pane project backfill incomplete: {remaining} open panes still lack project_id"
        )));
    }

    Ok(())
}

fn rehome_to_shell_workspace(connection: &Connection) -> StorageResult<()> {
    WorkspaceRepository::get_by_id(connection, DEFAULT_WORKSPACE_ID)?;

    let pane_ids: Vec<String> = connection
        .prepare("SELECT id FROM panes WHERE closed_at IS NULL")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if pane_ids.is_empty() {
        return Ok(());
    }

    connection.execute(
        "UPDATE panes
         SET workspace_id = ?1, updated_at = datetime('now')
         WHERE closed_at IS NULL",
        [DEFAULT_WORKSPACE_ID],
    )?;

    for pane_id in &pane_ids {
        connection.execute(
            "UPDATE messages SET workspace_id = ?1, updated_at = datetime('now') WHERE pane_id = ?2",
            (DEFAULT_WORKSPACE_ID, pane_id),
        )?;
    }

    Ok(())
}

fn build_root_to_project_map(connection: &Connection) -> StorageResult<std::collections::HashMap<String, String>> {
    let workspaces = WorkspaceRepository::list_active(connection)?;
    let mut map = std::collections::HashMap::new();

    for workspace in workspaces {
        if !ProjectRepository::is_project_metadata(workspace.metadata_json.as_deref()) {
            continue;
        }
        if let Some(root) = read_workspace_approved_root(workspace.metadata_json.as_deref()) {
            map.entry(root).or_insert_with(|| workspace.id.clone());
        }
    }

    Ok(map)
}

fn resolve_focused_project_id(
    connection: &Connection,
    root_to_project: &std::collections::HashMap<String, String>,
) -> StorageResult<String> {
    if let Ok(active) = WorkspaceRepository::get_active(connection) {
        if ProjectRepository::is_project_metadata(active.metadata_json.as_deref()) {
            return Ok(active.id);
        }
        if let Some(root) = read_workspace_approved_root(active.metadata_json.as_deref()) {
            if let Some(project_id) = root_to_project.get(&root) {
                return Ok(project_id.clone());
            }
        }
    }

    if let Some((_, project_id)) = root_to_project.iter().next() {
        return Ok(project_id.clone());
    }

    Err(StorageError::Migration(
        "pane project backfill requires at least one registered project".to_string(),
    ))
}

fn read_workspace_approved_root(metadata_json: Option<&str>) -> Option<String> {
    parse_metadata(metadata_json)
        .get(APPROVED_ROOT_KEY)
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::projects::repository::ProjectRepository;
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::models::CreatePaneRequest;
    use crate::storage::repositories::messages::MessageRepository;
    use crate::storage::repositories::panes::PaneRepository;

    #[test]
    fn backfill_assigns_matching_project_and_preserves_messages() -> StorageResult<()> {
        let base = std::env::temp_dir().join("builderboard-pane-backfill");
        let pepfox_dir = base.join("PepFox");
        let arete_dir = base.join("Arete");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&pepfox_dir)?;
        fs::create_dir_all(&arete_dir)?;

        let path = test_database_path("pane-backfill.db")?;
        let _ = fs::remove_file(&path);
        let database = Database::initialize_at(path.clone())?;

        let pepfox_project_id = database.with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, &pepfox_dir.display().to_string(), true)
                .map(|project| project.id)
        })?;

        let (pane_id, message_count) = database.with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: Some(DEFAULT_WORKSPACE_ID.to_string()),
                    project_id: Some(pepfox_project_id.clone()),
                    title: Some("History".to_string()),
                    sort_order: Some(0),
                },
            )?;
            MessageRepository::append(
                connection,
                crate::storage::models::AppendMessageRequest {
                    pane_id: pane.id.clone(),
                    role: "user".to_string(),
                    content: "preserved".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )?;
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM messages WHERE pane_id = ?1",
                [&pane.id],
                |row| row.get(0),
            )?;
            connection.execute("DROP TRIGGER IF EXISTS panes_require_project_id_update", [])?;
            connection.execute(
                "UPDATE panes SET project_id = NULL WHERE id = ?1",
                [&pane.id],
            )?;
            connection.execute("DELETE FROM app_settings WHERE key = ?1", [BACKFILL_SETTING_KEY])?;
            backfill_project_ids(connection)?;
            rehome_to_shell_workspace(connection)?;
            Ok((pane.id, count))
        })?;

        assert_eq!(message_count, 1);

        database.with_connection(|connection| {
            let pane = PaneRepository::get_by_id(connection, &pane_id)?;
            assert_eq!(pane.workspace_id, DEFAULT_WORKSPACE_ID);
            assert_eq!(
                pane.project_id.as_deref(),
                Some(pepfox_project_id.as_str())
            );

            let messages = MessageRepository::list_for_pane(connection, &pane_id)?;
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "preserved");
            assert_eq!(messages[0].workspace_id, DEFAULT_WORKSPACE_ID);
            Ok(())
        })
    }
}