use std::path::Path;

use chrono::Utc;
use rusqlite::Connection;

use crate::filesystem_tools::error::{FilesystemError, FilesystemResult};
use crate::filesystem_tools::scope::ApprovedScope;
use crate::projects::metadata::{
    allocate_project_code, parse_metadata, project_code_base, project_metadata_map,
    APPROVED_ROOT_KEY, PROJECT_CODE_KEY, PROJECT_KIND_KEY, PROJECT_NAME_KEY,
};
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{CreatePaneRequest, SHELL_WORKSPACE_ID};
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::workspaces::WorkspaceRepository;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub code: String,
    pub approved_root: String,
    pub is_active: bool,
}

pub struct ProjectRepository;

fn project_from_metadata(
    workspace_id: &str,
    workspace_name: &str,
    metadata_json: Option<&str>,
    is_active: bool,
) -> Option<ProjectDto> {
    let metadata = parse_metadata(metadata_json);
    let approved_root = metadata
        .get(APPROVED_ROOT_KEY)
        .and_then(|value| value.as_str())
        .map(str::to_string)?;
    let project_kind = metadata
        .get(PROJECT_KIND_KEY)
        .and_then(|value| value.as_str())
        .unwrap_or("folder");
    if project_kind != "folder" {
        return None;
    }

    let project_name = metadata
        .get(PROJECT_NAME_KEY)
        .and_then(|value| value.as_str())
        .unwrap_or(workspace_name)
        .to_string();
    let project_code = metadata
        .get(PROJECT_CODE_KEY)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| project_code_base(&project_name));

    Some(ProjectDto {
        id: workspace_id.to_string(),
        workspace_id: workspace_id.to_string(),
        name: project_name,
        code: project_code,
        approved_root,
        is_active,
    })
}

impl ProjectRepository {
    pub fn get_by_id(connection: &Connection, project_id: &str) -> StorageResult<ProjectDto> {
        let workspace = WorkspaceRepository::get_by_id(connection, project_id)?;
        let active = WorkspaceRepository::get_active(connection)?;
        project_from_metadata(
            &workspace.id,
            &workspace.name,
            workspace.metadata_json.as_deref(),
            workspace.id == active.id,
        )
        .ok_or_else(|| {
            StorageError::InvalidInput(format!("workspace {project_id} is not a project"))
        })
    }

    pub fn get_approved_root(connection: &Connection, project_id: &str) -> StorageResult<String> {
        let project = Self::get_by_id(connection, project_id)?;
        Ok(project.approved_root)
    }

    pub fn load_scope(
        connection: &Connection,
        project_id: &str,
    ) -> FilesystemResult<ApprovedScope> {
        let approved_root = Self::get_approved_root(connection, project_id)
            .map_err(|error| FilesystemError::InvalidInput(error.to_string()))?;
        ApprovedScope::new(approved_root)
    }

    pub fn resolve_focused_project_id(connection: &Connection) -> StorageResult<String> {
        let active = WorkspaceRepository::get_active(connection)?;
        if let Some(project) = project_from_metadata(
            &active.id,
            &active.name,
            active.metadata_json.as_deref(),
            true,
        ) {
            return Ok(project.id);
        }

        let projects = Self::list(connection)?;
        projects
            .first()
            .map(|project| project.id.clone())
            .ok_or_else(|| {
                StorageError::InvalidInput(
                    "no focused project is available for pane creation".to_string(),
                )
            })
    }

    pub fn list(connection: &Connection) -> StorageResult<Vec<ProjectDto>> {
        let active = WorkspaceRepository::get_active(connection)?;
        let workspaces = WorkspaceRepository::list_active(connection)?;

        let mut projects = workspaces
            .iter()
            .filter_map(|workspace| {
                project_from_metadata(
                    &workspace.id,
                    &workspace.name,
                    workspace.metadata_json.as_deref(),
                    workspace.id == active.id,
                )
            })
            .collect::<Vec<_>>();

        projects.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        Ok(projects)
    }

    pub fn get_active(connection: &Connection) -> StorageResult<Option<ProjectDto>> {
        let active = WorkspaceRepository::get_active(connection)?;
        Ok(project_from_metadata(
            &active.id,
            &active.name,
            active.metadata_json.as_deref(),
            true,
        ))
    }

    pub fn create_from_folder(
        connection: &Connection,
        folder_path: &str,
        create_initial_pane: bool,
    ) -> StorageResult<ProjectDto> {
        let trimmed = folder_path.trim();
        if trimmed.is_empty() {
            return Err(StorageError::InvalidInput(
                "project folder path cannot be empty".to_string(),
            ));
        }

        ApprovedScope::new(trimmed).map_err(|error| {
            StorageError::InvalidInput(format!("invalid project folder: {error}"))
        })?;
        let canonical_root = Path::new(trimmed)
            .canonicalize()
            .map_err(|error| StorageError::InvalidInput(error.to_string()))?;
        let project_name = canonical_root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                StorageError::InvalidInput("project folder must have a name".to_string())
            })?
            .to_string();

        let existing_codes = Self::existing_project_codes(connection)?;
        let project_code = allocate_project_code(&project_name, &existing_codes);
        let metadata = project_metadata_map(
            &project_name,
            &project_code,
            &canonical_root.display().to_string(),
        );
        let metadata_json = serde_json::to_string(&metadata)?;

        let workspace = WorkspaceRepository::create(connection, &project_name)?;
        let now = Utc::now().to_rfc3339();
        connection.execute(
            "UPDATE workspaces SET metadata_json = ?1, updated_at = ?2 WHERE id = ?3",
            (&metadata_json, &now, &workspace.id),
        )?;

        WorkspaceRepository::switch_active(connection, &workspace.id)?;

        if create_initial_pane {
            let shell_panes = PaneRepository::list_shell_open(connection)?;
            let has_project_pane = shell_panes
                .iter()
                .any(|pane| pane.project_id.as_deref() == Some(workspace.id.as_str()));
            if !has_project_pane {
                PaneRepository::create(
                    connection,
                    CreatePaneRequest {
                        workspace_id: Some(SHELL_WORKSPACE_ID.to_string()),
                        project_id: Some(workspace.id.clone()),
                        title: Some(project_name.clone()),
                        sort_order: None,
                    },
                )?;
            }
        }

        project_from_metadata(&workspace.id, &project_name, Some(&metadata_json), true)
            .ok_or_else(|| StorageError::InvalidInput("failed to build project dto".to_string()))
    }

    pub fn switch(connection: &Connection, project_id: &str) -> StorageResult<ProjectDto> {
        let workspace = WorkspaceRepository::switch_active(connection, project_id)?;
        project_from_metadata(
            &workspace.id,
            &workspace.name,
            workspace.metadata_json.as_deref(),
            true,
        )
        .ok_or_else(|| {
            StorageError::InvalidInput(format!("workspace {project_id} is not a project"))
        })
    }

    fn existing_project_codes(connection: &Connection) -> StorageResult<Vec<String>> {
        let workspaces = WorkspaceRepository::list_active(connection)?;
        Ok(workspaces
            .iter()
            .filter_map(|workspace| {
                parse_metadata(workspace.metadata_json.as_deref())
                    .get(PROJECT_CODE_KEY)
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .collect())
    }

    pub fn is_project_metadata(metadata_json: Option<&str>) -> bool {
        let metadata = parse_metadata(metadata_json);
        metadata.get(PROJECT_KIND_KEY).and_then(|v| v.as_str()) == Some("folder")
            && metadata
                .get(APPROVED_ROOT_KEY)
                .and_then(|v| v.as_str())
                .is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::storage::db::test_database_path;
    use crate::storage::db::Database;

    #[test]
    fn create_project_sets_workspace_root_and_code() -> StorageResult<()> {
        let root = std::env::temp_dir().join("builderboard-project-create");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;

        let path = test_database_path("project-create.db")?;
        let _ = fs::remove_file(&path);
        let database = Database::initialize_at(path)?;

        let project = database.with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, &root.display().to_string(), true)
        })?;

        assert_eq!(project.name, root.file_name().unwrap().to_str().unwrap());
        assert!(!project.code.is_empty());
        assert!(project
            .approved_root
            .contains("builderboard-project-create"));
        assert!(project.is_active);

        let listed = database.with_connection(ProjectRepository::list)?;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, project.id);

        Ok(())
    }

    #[test]
    fn project_code_collision_uses_numeric_suffix() -> StorageResult<()> {
        let base = std::env::temp_dir().join("builderboard-project-collision");
        let first = base.join("PepFox");
        let second = base.join("PeopleOps");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&first)?;
        fs::create_dir_all(&second)?;

        let path = test_database_path("project-collision.db")?;
        let _ = fs::remove_file(&path);
        let database = Database::initialize_at(path)?;

        let pepfox = database.with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, &first.display().to_string(), true)
        })?;
        let peopleops = database.with_connection(|connection| {
            ProjectRepository::create_from_folder(connection, &second.display().to_string(), true)
        })?;

        assert_eq!(pepfox.code, "Pe");
        assert_eq!(peopleops.code, "Pe2");

        Ok(())
    }
}
