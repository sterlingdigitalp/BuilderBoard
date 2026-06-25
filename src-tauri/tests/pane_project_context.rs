use std::fs;
use std::path::PathBuf;

use builderboard_lib::filesystem_tools::filesystem_read_file_with_database;
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::models::{CreatePaneRequest, SHELL_WORKSPACE_ID};
use builderboard_lib::storage::repositories::messages::MessageRepository;
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
fn multi_project_panes_remain_visible_with_independent_filesystem_scope() {
    let base = std::env::temp_dir().join("builderboard-pane-project-matrix");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base");

    let path = test_database_path("pane-project-matrix.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize");

    let pepfox_id = create_project(&database, &base, "PepFox");
    let arete_id = create_project(&database, &base, "Arete");
    let agenthive_id = create_project(&database, &base, "AgentHive");

    database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(agenthive_id.clone()),
                    title: Some("AgentHive Builder B".to_string()),
                    sort_order: Some(3),
                },
            )
        })
        .expect("second agenthive pane");

    let panes = database
        .with_connection(PaneRepository::list_shell_open)
        .expect("list panes");
    assert_eq!(panes.len(), 4);
    assert!(panes
        .iter()
        .all(|pane| pane.workspace_id == SHELL_WORKSPACE_ID));

    let project_ids: Vec<_> = panes
        .iter()
        .filter_map(|pane| pane.project_id.clone())
        .collect();
    assert_eq!(project_ids.iter().filter(|id| *id == &pepfox_id).count(), 1);
    assert_eq!(project_ids.iter().filter(|id| *id == &arete_id).count(), 1);
    assert_eq!(
        project_ids.iter().filter(|id| *id == &agenthive_id).count(),
        2
    );

    let pepfox_package =
        filesystem_read_file_with_database(&database, None, Some(&pepfox_id), "package.json")
            .expect("read pepfox");
    assert!(pepfox_package.content.contains("pepfox"));

    let arete_package =
        filesystem_read_file_with_database(&database, None, Some(&arete_id), "package.json")
            .expect("read arete");
    assert!(arete_package.content.contains("arete"));

    let agenthive_package =
        filesystem_read_file_with_database(&database, None, Some(&agenthive_id), "package.json")
            .expect("read agenthive");
    assert!(agenthive_package.content.contains("agenthive"));

    let pepfox_pane = panes
        .iter()
        .find(|pane| pane.project_id.as_deref() == Some(pepfox_id.as_str()))
        .expect("pepfox pane");
    database
        .with_connection(|connection| {
            MessageRepository::append(
                connection,
                builderboard_lib::storage::models::AppendMessageRequest {
                    pane_id: pepfox_pane.id.clone(),
                    role: "user".to_string(),
                    content: "independent".to_string(),
                    content_type: None,
                    metadata_json: None,
                },
            )
        })
        .expect("append");

    let messages = database
        .with_connection(|connection| MessageRepository::list_for_pane(connection, &pepfox_pane.id))
        .expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "independent");
}
