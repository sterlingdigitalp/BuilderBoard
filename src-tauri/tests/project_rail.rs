use std::fs;
use std::path::PathBuf;

use builderboard_lib::filesystem_tools::{
    filesystem_get_approved_root_with_database, filesystem_list_directory_with_database,
    filesystem_read_file_with_database,
};
use builderboard_lib::projects::commands::{
    project_create_from_folder_with_database, project_list_from_database,
    project_switch_with_database,
};
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::repositories::messages::MessageRepository;
use builderboard_lib::storage::repositories::panes::PaneRepository;
use builderboard_lib::storage::repositories::workspaces::WorkspaceRepository;

fn test_database_path(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base).expect("create test base");
    base.join(name)
}

fn create_named_project(database: &Database, parent: &PathBuf, name: &str) -> String {
    let folder = parent.join(name);
    fs::create_dir_all(&folder).expect("create project folder");
    if name == "PepFox" {
        fs::write(folder.join("package.json"), r#"{"name":"pepfox"}"#).expect("write package");
    }

    let project =
        project_create_from_folder_with_database(database, &folder.display().to_string(), None)
            .expect("create project");

    project.id
}

#[test]
fn project_rail_codes_and_switching() {
    let base = std::env::temp_dir().join("builderboard-project-rail-codes");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base");

    let path = test_database_path("project-rail-codes.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize");

    let names = ["PepFox", "Assymetry", "AgentHive", "Arete", "Longevity"];
    let mut ids = Vec::new();
    for name in names {
        ids.push(create_named_project(&database, &base, name));
    }

    let projects = project_list_from_database(&database).expect("list");
    assert_eq!(projects.len(), 5);
    let codes: Vec<_> = projects
        .iter()
        .map(|project| project.code.as_str())
        .collect();
    assert!(codes.contains(&"Pe"));
    assert!(codes.contains(&"As"));
    assert!(codes.contains(&"Ag"));
    assert!(codes.contains(&"Ar"));
    assert!(codes.contains(&"Lo"));

    let pepfox_id = projects
        .iter()
        .find(|project| project.name == "PepFox")
        .expect("pepfox")
        .id
        .clone();
    let switched = project_switch_with_database(&database, &pepfox_id).expect("switch");
    assert!(switched.is_active);
    assert!(switched.approved_root.contains("PepFox"));

    let approved = filesystem_get_approved_root_with_database(&database, None, Some(&pepfox_id))
        .expect("approved root")
        .approved_root
        .expect("root");
    assert!(approved.contains("PepFox"));

    let listing = filesystem_list_directory_with_database(&database, None, Some(&pepfox_id), ".")
        .expect("list");
    assert!(listing
        .entries
        .iter()
        .any(|entry| entry.name == "package.json"));

    let package =
        filesystem_read_file_with_database(&database, None, Some(&pepfox_id), "package.json")
            .expect("read");
    assert!(package.content.contains("pepfox"));

    let blocked =
        filesystem_read_file_with_database(&database, None, Some(&pepfox_id), "/etc/passwd");
    assert!(blocked.is_err());
}

#[test]
fn project_rail_persists_across_database_reopen() {
    let base = std::env::temp_dir().join("builderboard-project-rail-restart");
    let folder = base.join("RestartProject");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&folder).expect("create folder");
    fs::write(folder.join("README.md"), "# restart").expect("write readme");

    let path = test_database_path("project-rail-restart.db");
    let _ = fs::remove_file(&path);

    let (project_id, pane_id) = {
        let database = Database::initialize_at(path.clone()).expect("initialize");
        let project = project_create_from_folder_with_database(
            &database,
            &folder.display().to_string(),
            None,
        )
        .expect("create");

        let pane_id = database
            .with_connection(|connection| {
                let panes = PaneRepository::list_shell_open(connection)?;
                let pane = panes
                    .iter()
                    .find(|pane| pane.project_id.as_deref() == Some(project.id.as_str()))
                    .expect("project pane");
                Ok(pane.id.clone())
            })
            .expect("pane");

        database
            .with_connection(|connection| {
                MessageRepository::append(
                    connection,
                    builderboard_lib::storage::models::AppendMessageRequest {
                        pane_id: pane_id.clone(),
                        role: "user".to_string(),
                        content: "persisted".to_string(),
                        content_type: None,
                        metadata_json: None,
                    },
                )
            })
            .expect("message");

        (project.id, pane_id)
    };

    let database = Database::initialize_at(path).expect("reopen");
    let projects = project_list_from_database(&database).expect("list");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project_id);
    assert!(projects[0].approved_root.contains("RestartProject"));

    let active = database
        .with_connection(WorkspaceRepository::get_active)
        .expect("active");
    assert_eq!(active.id, project_id);

    let messages = database
        .with_connection(|connection| MessageRepository::list_for_pane(connection, &pane_id))
        .expect("messages");
    assert_eq!(messages[0].content, "persisted");
}

#[test]
fn project_rail_supports_many_projects() {
    let base = std::env::temp_dir().join("builderboard-project-rail-many");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base");

    let path = test_database_path("project-rail-many.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize");

    for index in 0..25 {
        create_named_project(&database, &base, &format!("Project{index:02}"));
    }

    let projects = project_list_from_database(&database).expect("list");
    assert_eq!(projects.len(), 25);
}

#[test]
#[ignore = "requires local PepFox checkout"]
fn project_rail_pepfox_live_folder() {
    let pepfox = "/Users/sterlingdigital/PepFox";
    if !PathBuf::from(pepfox).is_dir() {
        return;
    }

    let path = test_database_path("project-rail-pepfox-live.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize");

    let project =
        project_create_from_folder_with_database(&database, pepfox, None).expect("create pepfox");
    assert_eq!(project.name, "PepFox");
    assert_eq!(project.code, "Pe");
    assert_eq!(
        project.approved_root,
        fs::canonicalize(pepfox).unwrap().display().to_string()
    );

    let find_ts = builderboard_lib::filesystem_tools::filesystem_find_files_with_database(
        &database,
        None,
        Some(&project.id),
        ".",
        "*.ts",
    )
    .expect("find");
    assert!(!find_ts.matches.is_empty());
}
