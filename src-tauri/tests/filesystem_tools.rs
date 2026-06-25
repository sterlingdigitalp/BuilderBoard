use std::fs;
use std::path::PathBuf;

use builderboard_lib::filesystem_tools::{
    filesystem_find_files_with_database, filesystem_get_approved_root_with_database,
    filesystem_list_directory_with_database, filesystem_read_file_with_database,
    filesystem_search_files_with_database,
};
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::storage::db::Database;

const PEPFOX_ROOT: &str = "/Users/sterlingdigital/PepFox";

fn test_database_path(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base).expect("create test base");
    base.join(name)
}

fn pepfox_available() -> bool {
    PathBuf::from(PEPFOX_ROOT).is_dir()
}

fn setup_database_with_project(name: &str, root: &PathBuf) -> (Database, String) {
    let path = test_database_path(name);
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize database");
    let project =
        project_create_from_folder_with_database(&database, &root.display().to_string(), None)
            .expect("create project");
    (database, project.id)
}

#[test]
fn filesystem_rejects_unconfigured_workspace() {
    let path = test_database_path("filesystem-unconfigured.db");
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path).expect("initialize database");

    let result = filesystem_read_file_with_database(&database, None, None, "package.json");
    assert!(result.is_err());
    assert!(result
        .expect_err("expected not configured")
        .contains("project_id is required"));
}

#[test]
fn filesystem_get_approved_root_returns_project_metadata_value() {
    let root = std::env::temp_dir().join("builderboard-fs-get-root");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    let (database, project_id) =
        setup_database_with_project("filesystem-get-approved-root.db", &root);

    let approved_root =
        filesystem_get_approved_root_with_database(&database, None, Some(&project_id))
            .expect("get approved root");

    assert_eq!(
        approved_root.approved_root.as_deref(),
        Some(
            root.canonicalize()
                .expect("canonical root")
                .to_str()
                .expect("utf-8 root")
        )
    );
}

#[test]
fn filesystem_security_rejects_traversal_and_escape() {
    let root = std::env::temp_dir().join("builderboard-fs-security-root");
    let outside = std::env::temp_dir().join("builderboard-fs-security-outside");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(root.join("nested")).expect("create nested");
    fs::create_dir_all(&outside).expect("create outside");
    fs::write(outside.join("secret.txt"), "secret").expect("write outside");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&outside, root.join("escape")).expect("create symlink");
    }

    let (database, project_id) = setup_database_with_project("filesystem-security.db", &root);

    for bad_path in ["../secret", "../../secret", "/etc/passwd", "/Users"] {
        let read = filesystem_read_file_with_database(&database, None, Some(&project_id), bad_path);
        assert!(read.is_err(), "expected read failure for {bad_path}");
        let search = filesystem_search_files_with_database(
            &database,
            None,
            Some(&project_id),
            bad_path,
            "secret",
        );
        assert!(search.is_err(), "expected search failure for {bad_path}");
    }

    let symlink_read =
        filesystem_read_file_with_database(&database, None, Some(&project_id), "escape/secret.txt");
    assert!(symlink_read.is_err(), "expected symlink escape rejection");
}

#[test]
fn filesystem_local_fixture_operations() {
    let root = std::env::temp_dir().join("builderboard-fs-fixture");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("package.json"), r#"{"name":"fixture"}"#).expect("write package");
    fs::write(root.join("README.md"), "# Fixture").expect("write readme");
    fs::write(root.join("src/index.ts"), "export const OAuth = true;").expect("write ts");
    fs::write(
        root.join("src/Button.tsx"),
        "export const Button = () => null;",
    )
    .expect("write tsx");

    let (database, project_id) = setup_database_with_project("filesystem-fixture.db", &root);

    let listing = filesystem_list_directory_with_database(&database, None, Some(&project_id), ".")
        .expect("list");
    assert!(listing
        .entries
        .iter()
        .any(|entry| entry.name == "package.json"));
    assert!(listing.entries.iter().any(|entry| entry.name == "src"));

    let package =
        filesystem_read_file_with_database(&database, None, Some(&project_id), "package.json")
            .expect("read");
    assert!(package.content.contains("fixture"));

    let ts_files =
        filesystem_find_files_with_database(&database, None, Some(&project_id), ".", "*.ts")
            .expect("find");
    assert_eq!(ts_files.matches, vec!["src/index.ts".to_string()]);

    let oauth_hits =
        filesystem_search_files_with_database(&database, None, Some(&project_id), ".", "OAuth")
            .expect("search");
    assert_eq!(oauth_hits.matches.len(), 1);
    assert_eq!(oauth_hits.matches[0].path, "src/index.ts");

    let traversal =
        filesystem_read_file_with_database(&database, None, Some(&project_id), "../../secret");
    assert!(traversal.is_err());
}

#[test]
#[ignore = "requires local PepFox checkout"]
fn filesystem_pepfox_validation_scenarios() {
    if !pepfox_available() {
        println!("SKIP: PepFox not available at {PEPFOX_ROOT}");
        return;
    }

    let pepfox_root = PathBuf::from(PEPFOX_ROOT);
    let (database, project_id) = setup_database_with_project("filesystem-pepfox.db", &pepfox_root);

    let listing = filesystem_list_directory_with_database(&database, None, Some(&project_id), ".")
        .expect("list root");
    assert!(
        !listing.entries.is_empty(),
        "root listing should not be empty"
    );

    let package =
        filesystem_read_file_with_database(&database, None, Some(&project_id), "package.json");
    assert!(package.is_ok(), "package.json should be readable");

    let ts_files =
        filesystem_find_files_with_database(&database, None, Some(&project_id), ".", "*.ts")
            .expect("find");
    assert!(!ts_files.matches.is_empty(), "should find .ts files");

    let oauth_hits =
        filesystem_search_files_with_database(&database, None, Some(&project_id), ".", "OAuth");
    assert!(
        oauth_hits.is_ok(),
        "search should complete without error: {:?}",
        oauth_hits.err()
    );

    for bad_path in ["/etc/passwd", "../../secret", "/Users"] {
        let read = filesystem_read_file_with_database(&database, None, Some(&project_id), bad_path);
        assert!(read.is_err(), "expected failure for {bad_path}");
    }
}
