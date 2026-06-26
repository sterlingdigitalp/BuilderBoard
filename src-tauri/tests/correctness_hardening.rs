use std::fs;

use builderboard_lib::execution::tools::context::ToolContext;
use builderboard_lib::execution::tools::directory::CreateTool;
use builderboard_lib::execution::tools::filesystem::WriteTool;
use builderboard_lib::execution::tools::traits::Tool;
use builderboard_lib::filesystem_tools::filesystem_read_file_with_database;
use builderboard_lib::filesystem_tools::scope::ApprovedScope;
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::projects::repository::ProjectRepository;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::models::{CreatePaneRequest, SHELL_WORKSPACE_ID};
use builderboard_lib::storage::repositories::panes::PaneRepository;

fn temp_database(name: &str) -> Database {
    let path = std::env::temp_dir().join("builderboard-tests").join(name);
    let _ = fs::remove_file(&path);
    Database::initialize_at(path).expect("initialize database")
}

#[test]
fn invalid_project_id_fails_filesystem_lookup_safely() {
    let database = temp_database("correctness-invalid-project.db");
    let result = filesystem_read_file_with_database(
        &database,
        None,
        Some("not-a-real-project-id"),
        "package.json",
    );

    let err = result
        .expect_err("invalid project should fail")
        .to_lowercase();
    assert!(
        err.contains("not found") || err.contains("project"),
        "expected project lookup failure, got: {err}"
    );
}

#[test]
fn invalid_pane_id_fails_stream_prepare_safely() {
    let database = temp_database("correctness-invalid-pane.db");
    let result = database.with_connection(|connection| {
        PaneRepository::get_open_by_id(connection, "missing-pane-id")
    });

    assert!(result.is_err());
}

#[test]
fn pane_create_requires_valid_project_binding() {
    let database = temp_database("correctness-pane-project.db");
    let err = database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some("missing-project-id".to_string()),
                    title: Some("Broken pane".to_string()),
                    sort_order: None,
                },
            )
        })
        .expect_err("invalid project binding should fail");

    assert!(
        err.to_string().to_lowercase().contains("not found")
            || err.to_string().to_lowercase().contains("project")
    );
}

#[test]
fn missing_project_directory_fails_project_creation_safely() {
    let database = temp_database("correctness-missing-root.db");
    let missing = std::env::temp_dir().join("builderboard-missing-project-root-8g");
    let _ = fs::remove_dir_all(&missing);

    let result = project_create_from_folder_with_database(
        &database,
        &missing.display().to_string(),
        Some(false),
    );

    assert!(result.is_err());
}

#[test]
fn active_project_id_is_project_not_shell_workspace() {
    let database = temp_database("correctness-active-project.db");
    let root = std::env::temp_dir().join("builderboard-correctness-active-root");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    fs::write(root.join("package.json"), r#"{"name":"active-test"}"#).expect("write package");

    let project = project_create_from_folder_with_database(
        &database,
        &root.display().to_string(),
        Some(true),
    )
    .expect("create project");

    let active = database
        .with_connection(ProjectRepository::get_active)
        .expect("get active project")
        .expect("active project should exist");

    assert_eq!(active.id, project.id);
    assert_ne!(active.id, SHELL_WORKSPACE_ID);
}

#[test]
fn closed_pane_cannot_be_reopened_by_get_open_by_id() {
    let database = temp_database("correctness-pane-close.db");
    let root = std::env::temp_dir().join("builderboard-correctness-close-root");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    fs::write(root.join("package.json"), r#"{"name":"close-test"}"#).expect("write package");

    let project = project_create_from_folder_with_database(
        &database,
        &root.display().to_string(),
        Some(false),
    )
    .expect("create project");

    let pane_id = database
        .with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(project.id),
                    title: Some("Close me".to_string()),
                    sort_order: None,
                },
            )?;
            PaneRepository::close(connection, &pane.id)?;
            Ok(pane.id)
        })
        .expect("create and close pane");

    let result =
        database.with_connection(|connection| PaneRepository::get_open_by_id(connection, &pane_id));
    assert!(result.is_err());
}

fn scoped_tool_context(root: &std::path::Path) -> ToolContext {
    let mut ctx = ToolContext::local("correctness-scope-test");
    ctx.project_root = Some(root.to_path_buf());
    ctx.filesystem_scope = Some(ApprovedScope::new(root).expect("approved scope"));
    ctx
}

#[test]
fn filesystem_write_can_create_new_file_inside_approved_scope() {
    let root = std::env::temp_dir().join("builderboard-correctness-write-create");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");

    let tool = WriteTool;
    let result = tool
        .execute(
            scoped_tool_context(&root),
            serde_json::json!({
                "path": "docs/test.md",
                "content": "created"
            }),
            &|_| {},
        )
        .expect("write tool should create new path");

    assert!(result.success);
    assert_eq!(
        fs::read_to_string(root.join("docs/test.md")).expect("read created file"),
        "created"
    );
}

#[test]
fn directory_create_can_create_nested_directory_inside_approved_scope() {
    let root = std::env::temp_dir().join("builderboard-correctness-dir-create");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");

    let tool = CreateTool;
    assert!(tool
        .validate(&serde_json::json!({ "path": "docs/nested" }))
        .is_ok());
    let result = tool
        .execute(
            scoped_tool_context(&root),
            serde_json::json!({ "path": "docs/nested" }),
            &|_| {},
        )
        .expect("directory create should create new path");

    assert!(result.success);
    assert!(root.join("docs/nested").is_dir());
}

#[test]
fn filesystem_write_rejects_traversal_create_path() {
    let root = std::env::temp_dir().join("builderboard-correctness-write-traversal");
    let outside = std::env::temp_dir().join("builderboard-correctness-write-traversal-outside.md");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&outside);
    fs::create_dir_all(&root).expect("create root");

    let tool = WriteTool;
    let result = tool.execute(
        scoped_tool_context(&root),
        serde_json::json!({
            "path": "../builderboard-correctness-write-traversal-outside.md",
            "content": "escape"
        }),
        &|_| {},
    );

    assert!(result.is_err());
    assert!(!outside.exists());
}

#[test]
fn filesystem_write_rejects_symlink_parent_escape() {
    let root = std::env::temp_dir().join("builderboard-correctness-write-symlink");
    let outside = std::env::temp_dir().join("builderboard-correctness-write-outside");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&root).expect("create root");
    fs::create_dir_all(&outside).expect("create outside");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&outside, root.join("escape")).expect("create symlink");
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        symlink_dir(&outside, root.join("escape")).expect("create symlink");
    }

    let tool = WriteTool;
    let result = tool.execute(
        scoped_tool_context(&root),
        serde_json::json!({
            "path": "escape/new.md",
            "content": "escape"
        }),
        &|_| {},
    );

    assert!(result.is_err());
    assert!(!outside.join("new.md").exists());
}
