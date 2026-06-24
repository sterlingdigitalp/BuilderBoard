use rusqlite::Connection;
use serde_json::{Map, Value};
use std::path::Path;

use crate::filesystem_tools::error::{FilesystemError, FilesystemResult};
use crate::filesystem_tools::scope::ApprovedScope;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::repositories::workspaces::WorkspaceRepository;

pub const METADATA_KEY: &str = "filesystemApprovedRoot";

pub fn get_approved_root(
    connection: &Connection,
    workspace_id: Option<&str>,
) -> StorageResult<Option<String>> {
    let workspace_id = WorkspaceRepository::resolve_workspace_id(connection, workspace_id)?;
    let workspace = WorkspaceRepository::get_by_id(connection, &workspace_id)?;
    Ok(read_metadata_value(workspace.metadata_json.as_deref()))
}

pub fn set_approved_root(
    connection: &Connection,
    workspace_id: Option<&str>,
    path: &str,
) -> StorageResult<String> {
    let workspace_id = WorkspaceRepository::resolve_workspace_id(connection, workspace_id)?;
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(StorageError::InvalidInput(
            "approved root path cannot be empty".to_string(),
        ));
    }

    ApprovedScope::new(trimmed).map_err(map_scope_error)?;
    let canonical = Path::new(trimmed)
        .canonicalize()
        .map_err(|error| StorageError::InvalidInput(error.to_string()))?
        .display()
        .to_string();

    let workspace = WorkspaceRepository::get_by_id(connection, &workspace_id)?;
    let mut metadata = parse_metadata(workspace.metadata_json.as_deref());
    metadata.insert(METADATA_KEY.to_string(), Value::String(canonical.clone()));

    let metadata_json = serde_json::to_string(&metadata)?;
    connection.execute(
        "UPDATE workspaces SET metadata_json = ?1, updated_at = datetime('now') WHERE id = ?2",
        (&metadata_json, &workspace_id),
    )?;

    Ok(canonical)
}

pub fn load_scope(
    connection: &Connection,
    workspace_id: Option<&str>,
) -> FilesystemResult<ApprovedScope> {
    let approved_root = get_approved_root(connection, workspace_id).map_err(map_storage_error)?;
    let approved_root = approved_root.ok_or(FilesystemError::NotConfigured)?;
    ApprovedScope::new(approved_root)
}

fn parse_metadata(metadata_json: Option<&str>) -> Map<String, Value> {
    metadata_json
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn read_metadata_value(metadata_json: Option<&str>) -> Option<String> {
    parse_metadata(metadata_json)
        .get(METADATA_KEY)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn map_scope_error(error: FilesystemError) -> StorageError {
    StorageError::InvalidInput(error.to_string())
}

fn map_storage_error(error: StorageError) -> FilesystemError {
    FilesystemError::InvalidInput(error.to_string())
}
