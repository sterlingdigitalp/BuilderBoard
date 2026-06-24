use std::sync::Arc;

use rusqlite::Connection;
use tauri::State;

use crate::filesystem_tools::approved_root::set_approved_root;
use crate::filesystem_tools::error::FilesystemError;
use crate::filesystem_tools::models::{
    ApprovedRootResult, FindFilesResult, ListDirectoryResult, ReadFileResult, SearchFilesResult,
};
use crate::filesystem_tools::scope::ApprovedScope;
use crate::filesystem_tools::service::FilesystemService;
use crate::projects::repository::ProjectRepository;
use crate::storage::db::Database;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::repositories::workspaces::WorkspaceRepository;

#[tauri::command]
pub fn filesystem_set_approved_root(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    path: String,
) -> Result<String, String> {
    filesystem_set_approved_root_with_database(database.inner(), workspace_id.as_deref(), &path)
}

pub fn filesystem_set_approved_root_with_database(
    database: &Database,
    workspace_id: Option<&str>,
    path: &str,
) -> Result<String, String> {
    database
        .with_connection(|connection| set_approved_root(connection, workspace_id, path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn filesystem_get_approved_root(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    project_id: Option<String>,
) -> Result<ApprovedRootResult, String> {
    filesystem_get_approved_root_with_database(
        database.inner(),
        workspace_id.as_deref(),
        project_id.as_deref(),
    )
}

pub fn filesystem_get_approved_root_with_database(
    database: &Database,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
) -> Result<ApprovedRootResult, String> {
    database
        .with_connection(|connection| {
            let resolved_project_id = resolve_project_id(connection, workspace_id, project_id)?;
            let approved_root = ProjectRepository::get_approved_root(connection, &resolved_project_id)?;
            Ok(ApprovedRootResult {
                workspace_id: resolved_project_id.clone(),
                approved_root: Some(approved_root),
            })
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn filesystem_list_directory(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    project_id: Option<String>,
    path: String,
) -> Result<ListDirectoryResult, String> {
    filesystem_list_directory_with_database(
        database.inner(),
        workspace_id.as_deref(),
        project_id.as_deref(),
        &path,
    )
}

pub fn filesystem_list_directory_with_database(
    database: &Database,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
    path: &str,
) -> Result<ListDirectoryResult, String> {
    database
        .with_connection(|connection| {
            let scope = resolve_scope(connection, workspace_id, project_id)
                .map_err(map_filesystem_error)?;
            FilesystemService::list_directory(&scope, path).map_err(map_filesystem_error)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn filesystem_read_file(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    project_id: Option<String>,
    path: String,
) -> Result<ReadFileResult, String> {
    filesystem_read_file_with_database(
        database.inner(),
        workspace_id.as_deref(),
        project_id.as_deref(),
        &path,
    )
}

pub fn filesystem_read_file_with_database(
    database: &Database,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
    path: &str,
) -> Result<ReadFileResult, String> {
    database
        .with_connection(|connection| {
            let scope = resolve_scope(connection, workspace_id, project_id)
                .map_err(map_filesystem_error)?;
            FilesystemService::read_file(&scope, path).map_err(map_filesystem_error)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn filesystem_search_files(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    project_id: Option<String>,
    path: String,
    query: String,
) -> Result<SearchFilesResult, String> {
    filesystem_search_files_with_database(
        database.inner(),
        workspace_id.as_deref(),
        project_id.as_deref(),
        &path,
        &query,
    )
}

pub fn filesystem_search_files_with_database(
    database: &Database,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
    path: &str,
    query: &str,
) -> Result<SearchFilesResult, String> {
    database
        .with_connection(|connection| {
            let scope = resolve_scope(connection, workspace_id, project_id)
                .map_err(map_filesystem_error)?;
            FilesystemService::search_files(&scope, path, query).map_err(map_filesystem_error)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn filesystem_find_files(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    project_id: Option<String>,
    path: String,
    pattern: String,
) -> Result<FindFilesResult, String> {
    filesystem_find_files_with_database(
        database.inner(),
        workspace_id.as_deref(),
        project_id.as_deref(),
        &path,
        &pattern,
    )
}

pub fn filesystem_find_files_with_database(
    database: &Database,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
    path: &str,
    pattern: &str,
) -> Result<FindFilesResult, String> {
    database
        .with_connection(|connection| {
            let scope = resolve_scope(connection, workspace_id, project_id)
                .map_err(map_filesystem_error)?;
            FilesystemService::find_files(&scope, path, pattern).map_err(map_filesystem_error)
        })
        .map_err(|error| error.to_string())
}

fn resolve_project_id(
    connection: &Connection,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
) -> StorageResult<String> {
    if let Some(project_id) = project_id {
        ProjectRepository::get_by_id(connection, project_id)?;
        return Ok(project_id.to_string());
    }

    if let Some(workspace_id) = workspace_id {
        let workspace = WorkspaceRepository::get_by_id(connection, workspace_id)?;
        if ProjectRepository::is_project_metadata(workspace.metadata_json.as_deref()) {
            return Ok(workspace_id.to_string());
        }
    }

    let active = WorkspaceRepository::get_active(connection)?;
    if ProjectRepository::is_project_metadata(active.metadata_json.as_deref()) {
        return Ok(active.id);
    }

    Err(StorageError::InvalidInput(
        "project_id is required for filesystem scope".to_string(),
    ))
}

fn resolve_scope(
    connection: &Connection,
    workspace_id: Option<&str>,
    project_id: Option<&str>,
) -> Result<ApprovedScope, FilesystemError> {
    let resolved_project_id =
        resolve_project_id(connection, workspace_id, project_id).map_err(map_storage_error)?;
    ProjectRepository::load_scope(connection, &resolved_project_id)
}

fn map_filesystem_error(error: FilesystemError) -> StorageError {
    StorageError::InvalidInput(error.to_string())
}

fn map_storage_error(error: StorageError) -> FilesystemError {
    FilesystemError::InvalidInput(error.to_string())
}