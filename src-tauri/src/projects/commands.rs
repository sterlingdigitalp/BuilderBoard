use std::sync::Arc;

use tauri::State;

use crate::project_scope_cache::ProjectScopeCache;
use crate::projects::repository::{ProjectDto, ProjectRepository};
use crate::storage::db::Database;

#[tauri::command]
pub fn project_list(database: State<'_, Arc<Database>>) -> Result<Vec<ProjectDto>, String> {
    project_list_from_database(database.inner())
}

pub fn project_list_from_database(database: &Database) -> Result<Vec<ProjectDto>, String> {
    database
        .with_connection(ProjectRepository::list)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn project_get_active(
    database: State<'_, Arc<Database>>,
) -> Result<Option<ProjectDto>, String> {
    project_get_active_from_database(database.inner())
}

pub fn project_get_active_from_database(database: &Database) -> Result<Option<ProjectDto>, String> {
    database
        .with_connection(ProjectRepository::get_active)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn project_create_from_folder(
    database: State<'_, Arc<Database>>,
    scope_cache: State<'_, Arc<ProjectScopeCache>>,
    folder_path: String,
    create_initial_pane: Option<bool>,
) -> Result<ProjectDto, String> {
    let project = project_create_from_folder_with_database(
        database.inner(),
        &folder_path,
        create_initial_pane,
    )?;
    scope_cache.invalidate_all();
    Ok(project)
}

pub fn project_create_from_folder_with_database(
    database: &Database,
    folder_path: &str,
    create_initial_pane: Option<bool>,
) -> Result<ProjectDto, String> {
    let create_initial_pane = create_initial_pane.unwrap_or(true);
    database
        .with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, folder_path, create_initial_pane)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn project_switch(
    database: State<'_, Arc<Database>>,
    scope_cache: State<'_, Arc<ProjectScopeCache>>,
    project_id: String,
) -> Result<ProjectDto, String> {
    let project = project_switch_with_database(database.inner(), &project_id)?;
    scope_cache.invalidate_project(&project_id);
    Ok(project)
}

pub fn project_switch_with_database(
    database: &Database,
    project_id: &str,
) -> Result<ProjectDto, String> {
    database
        .with_connection(|connection| ProjectRepository::switch(connection, project_id))
        .map_err(|error| error.to_string())
}
