use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use builderboard_lib::filesystem_tools::{
    filesystem_find_files_with_database, is_default_ignored_dir, FilesystemService, ScanContext,
};
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::projects::repository::ProjectRepository;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::error::StorageError;

fn test_database_path(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base).expect("create test base");
    base.join(name)
}

fn setup_project_with_heavy_node_modules(name: &str) -> (Database, String, PathBuf) {
    let root = std::env::temp_dir().join(format!("builderboard-fs-perf-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("package.json"), r#"{"name":"perf-fixture"}"#).expect("write package");
    fs::write(root.join("src/index.ts"), "export const OAuth = true;").expect("write src");

    let node_modules = root.join("node_modules");
    for index in 0..200 {
        let pkg = node_modules.join(format!("pkg-{index}"));
        fs::create_dir_all(&pkg).expect("create pkg");
        fs::write(pkg.join("index.ts"), format!("export const v = {index};")).expect("write dep");
    }

    let path = test_database_path(&format!("{name}.db"));
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize database");
    let project = project_create_from_folder_with_database(&database, &root.display().to_string(), None)
        .expect("create project");

    (database, project.id, root)
}

#[test]
fn ignored_directories_are_skipped_during_find_files() {
    let (database, project_id, _root) = setup_project_with_heavy_node_modules("ignore-find");

    let scope = database
        .with_connection(|connection| {
            ProjectRepository::load_scope(connection, &project_id)
                .map_err(|error| StorageError::InvalidInput(error.to_string()))
        })
        .expect("load scope");

    let mut ignored_context = ScanContext::for_tool("Find all TypeScript files.", ".");
    let ignored = FilesystemService::find_files_with_context(&scope, ".", "*.ts", &mut ignored_context)
        .expect("find with ignore");

    assert_eq!(ignored.matches, vec!["src/index.ts".to_string()]);
    assert!(!ignored
        .matches
        .iter()
        .any(|path| path.contains("node_modules")));

    let mut explicit_context =
        ScanContext::for_tool("Search node_modules for TypeScript.", "node_modules");
    let explicit = FilesystemService::find_files_with_context(
        &scope,
        "node_modules",
        "*.ts",
        &mut explicit_context,
    )
    .expect("find in node_modules");
    assert!(!explicit.matches.is_empty());
    assert!(explicit
        .matches
        .iter()
        .any(|path| path.contains("node_modules")));
}

#[test]
fn find_files_with_ignore_list_completes_quickly_on_heavy_node_modules() {
    let (database, project_id, _root) = setup_project_with_heavy_node_modules("ignore-speed");
    let scope = database
        .with_connection(|connection| {
            ProjectRepository::load_scope(connection, &project_id)
                .map_err(|error| StorageError::InvalidInput(error.to_string()))
        })
        .expect("load scope");

    let started = Instant::now();
    let mut context = ScanContext::for_tool("Find all TypeScript files.", ".");
    let result =
        FilesystemService::find_files_with_context(&scope, ".", "*.ts", &mut context).expect("find");
    let elapsed_ms = started.elapsed().as_millis();

    assert_eq!(result.matches, vec!["src/index.ts".to_string()]);
    assert!(
        elapsed_ms < 500,
        "expected fast scan with ignore list, took {elapsed_ms}ms"
    );
}

#[test]
fn direct_api_find_files_skips_node_modules_by_default() {
    let (database, project_id, _root) = setup_project_with_heavy_node_modules("ignore-api");
    let result = filesystem_find_files_with_database(&database, None, Some(&project_id), ".", "*.ts")
        .expect("api find");
    assert_eq!(result.matches, vec!["src/index.ts".to_string()]);
}

#[test]
fn default_ignore_list_contains_expected_entries() {
    assert!(is_default_ignored_dir("node_modules"));
    assert!(is_default_ignored_dir(".git"));
    assert!(is_default_ignored_dir("dist"));
}

#[test]
#[ignore = "manual benchmark against local project folders"]
fn benchmark_review_prompt_payload_on_local_projects() {
    let projects = [
        ("BuilderBoard", "/Users/sterlingdigital/BuilderBoard"),
        ("director-desk", "/Users/sterlingdigital/director-desk"),
        ("Polymath", "/Users/sterlingdigital/Polymath"),
    ];

    for (label, root) in projects {
        let root_path = PathBuf::from(root);
        if !root_path.is_dir() {
            println!("SKIP {label}: missing {root}");
            continue;
        }

        let path = test_database_path(&format!("benchmark-{label}.db"));
        let _ = fs::remove_file(&path);
        let database = Database::initialize_at(path).expect("initialize database");
        let project = project_create_from_folder_with_database(
            &database,
            &root_path.display().to_string(),
            None,
        )
        .expect("create project");

        let started = Instant::now();
        let find = filesystem_find_files_with_database(&database, None, Some(&project.id), ".", "*.ts")
            .expect("find ts");
        let find_ms = started.elapsed().as_millis();

        println!(
            "PROJECT={label} FIND_TS_COUNT={} FIND_MS={find_ms}",
            find.matches.len()
        );
    }
}