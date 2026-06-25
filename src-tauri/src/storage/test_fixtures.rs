#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use rusqlite::Connection;

#[cfg(test)]
use crate::projects::metadata::{project_code_base, project_metadata_map};
#[cfg(test)]
use crate::projects::repository::ProjectRepository;
#[cfg(test)]
use crate::storage::error::StorageResult;
#[cfg(test)]
use crate::storage::pane_project_migration;
#[cfg(test)]
use crate::storage::repositories::workspaces::WorkspaceRepository;

#[cfg(test)]
pub fn initialize_test_connection() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database");
    connection
        .execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)
        .expect("test schema");
    pane_project_migration::run_after_migrations(&connection).expect("pane project migration");
    connection
}

#[cfg(test)]
pub fn seed_test_project(connection: &Connection, name: &str) -> StorageResult<String> {
    if let Ok(project_id) = ProjectRepository::resolve_focused_project_id(connection) {
        if ProjectRepository::get_by_id(connection, &project_id).is_ok() {
            return Ok(project_id);
        }
    }

    let root = std::env::temp_dir().join(format!("builderboard-test-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;
    let canonical = root.canonicalize()?;
    let metadata = project_metadata_map(
        name,
        &project_code_base(name),
        &canonical.display().to_string(),
    );
    let metadata_json = serde_json::to_string(&metadata)?;
    let workspace = WorkspaceRepository::create(connection, name)?;
    connection.execute(
        "UPDATE workspaces SET metadata_json = ?1, updated_at = datetime('now') WHERE id = ?2",
        (&metadata_json, &workspace.id),
    )?;
    WorkspaceRepository::switch_active(connection, &workspace.id)?;
    Ok(workspace.id)
}

#[cfg(test)]
pub fn test_project_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("builderboard-test-{name}"))
}
