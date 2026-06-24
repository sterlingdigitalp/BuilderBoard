use std::fs;
use std::path::PathBuf;

use builderboard_lib::filesystem_tools::filesystem_read_file_with_database;
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::models::{CreatePaneRequest, SHELL_WORKSPACE_ID};
use builderboard_lib::storage::repositories::panes::PaneRepository;

fn test_database_path(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base).expect("create test base");
    base.join(name)
}

fn create_project(database: &Database, parent: &PathBuf, name: &str) -> String {
    let folder = parent.join(name);
    fs::create_dir_all(&folder).expect("create folder");
    fs::write(
        folder.join("package.json"),
        format!(r#"{{"name":"{}"}}"#, name.to_lowercase()),
    )
    .expect("write package");

    project_create_from_folder_with_database(database, &folder.display().to_string(), None)
        .expect("create project")
        .id
}

#[test]
fn rail_launch_creates_pane_bound_to_project() {
    let base = std::env::temp_dir().join("builderboard-pane-binding-launch");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base");

    let path = test_database_path("pane-binding-launch.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize");

    let pepfox_id = create_project(&database, &base, "PepFox");

    let created = database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(pepfox_id.clone()),
                    title: Some("PepFox launch".to_string()),
                    sort_order: None,
                },
            )
        })
        .expect("launch pane");

    assert_eq!(created.workspace_id, SHELL_WORKSPACE_ID);
    assert_eq!(created.project_id.as_deref(), Some(pepfox_id.as_str()));

    let package = filesystem_read_file_with_database(
        &database,
        None,
        Some(&pepfox_id),
        "package.json",
    )
    .expect("read pepfox package");
    assert!(package.content.contains("pepfox"));
}

#[test]
fn pane_project_rebind_updates_filesystem_scope() {
    let base = std::env::temp_dir().join("builderboard-pane-binding-rebind");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base");

    let path = test_database_path("pane-binding-rebind.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize");

    let pepfox_id = create_project(&database, &base, "PepFox");
    let arete_id = create_project(&database, &base, "Arete");

    let pane = database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(pepfox_id.clone()),
                    title: Some("Rebind pane".to_string()),
                    sort_order: None,
                },
            )
        })
        .expect("create pane");

    let pepfox_before = filesystem_read_file_with_database(
        &database,
        None,
        Some(&pepfox_id),
        "package.json",
    )
    .expect("read pepfox before rebind");
    assert!(pepfox_before.content.contains("pepfox"));

    let rebound = database
        .with_connection(|connection| {
            PaneRepository::set_project(connection, &pane.id, &arete_id)
        })
        .expect("rebind pane");

    assert_eq!(rebound.project_id.as_deref(), Some(arete_id.as_str()));

    let arete_after = filesystem_read_file_with_database(
        &database,
        None,
        rebound.project_id.as_deref(),
        "package.json",
    )
    .expect("read arete after rebind");
    assert!(arete_after.content.contains("arete"));
    assert!(!arete_after.content.contains("pepfox"));
}