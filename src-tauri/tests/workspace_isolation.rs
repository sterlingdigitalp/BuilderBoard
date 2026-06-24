use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use builderboard_lib::auth::{CredentialService, MemoryCredentialStore};
use builderboard_lib::filesystem_tools::filesystem_read_file_with_database;
use builderboard_lib::projects::commands::project_create_from_folder_with_database;
use builderboard_lib::storage::commands::account_create_api_key_with_service;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::error::{StorageError, StorageResult};
use builderboard_lib::storage::models::{AppendMessageRequest, SHELL_WORKSPACE_ID};
use builderboard_lib::storage::repositories::messages::MessageRepository;
use builderboard_lib::storage::repositories::panes::PaneRepository;

fn test_database_path(name: &str) -> StorageResult<PathBuf> {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base)?;
    Ok(base.join(name))
}

fn in_memory_credentials() -> CredentialService {
    CredentialService::with_store(Box::new(MemoryCredentialStore::default()))
}

fn create_project(database: &Database, parent: &PathBuf, name: &str) -> StorageResult<String> {
    let folder = parent.join(name);
    fs::create_dir_all(&folder)?;
    if name == "PepFox" {
        fs::write(folder.join("package.json"), r#"{"name":"pepfox"}"#)?;
    }
    if name == "Arete" {
        fs::write(folder.join("package.json"), r#"{"name":"arete"}"#)?;
    }
    let project = project_create_from_folder_with_database(
        database,
        &folder.display().to_string(),
        None,
    )
    .map_err(|error| StorageError::InvalidInput(error))?;
    Ok(project.id)
}

struct PaneFixture {
    path: PathBuf,
    pepfox_pane_id: String,
    arete_pane_id: String,
    pepfox_project_id: String,
    arete_project_id: String,
}

fn setup_two_project_panes(name: &str) -> StorageResult<PaneFixture> {
    let base = std::env::temp_dir().join(format!("builderboard-{name}"));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base)?;

    let path = test_database_path(name)?;
    let _ = fs::remove_file(&path);
    let database = Database::initialize_at(path.clone())?;

    let pepfox_project_id = create_project(&database, &base, "PepFox")?;
    let arete_project_id = create_project(&database, &base, "Arete")?;

    let (pepfox_pane_id, arete_pane_id) = database.with_connection(|connection| {
        let panes = PaneRepository::list_shell_open(connection)?;
        let pepfox_pane = panes
            .iter()
            .find(|pane| pane.project_id.as_deref() == Some(pepfox_project_id.as_str()))
            .expect("pepfox pane");
        let arete_pane = panes
            .iter()
            .find(|pane| pane.project_id.as_deref() == Some(arete_project_id.as_str()))
            .expect("arete pane");

        MessageRepository::append(
            connection,
            AppendMessageRequest {
                pane_id: pepfox_pane.id.clone(),
                role: "user".to_string(),
                content: "pepfox-message".to_string(),
                content_type: None,
                metadata_json: None,
            },
        )?;
        MessageRepository::append(
            connection,
            AppendMessageRequest {
                pane_id: arete_pane.id.clone(),
                role: "user".to_string(),
                content: "arete-message".to_string(),
                content_type: None,
                metadata_json: None,
            },
        )?;

        Ok((pepfox_pane.id.clone(), arete_pane.id.clone()))
    })?;

    Ok(PaneFixture {
        path,
        pepfox_pane_id,
        arete_pane_id,
        pepfox_project_id,
        arete_project_id,
    })
}

fn pane_ids(panes: &[builderboard_lib::storage::models::PaneDto]) -> HashSet<String> {
    panes.iter().map(|pane| pane.id.clone()).collect()
}

#[test]
fn shell_lists_all_project_bound_panes() -> StorageResult<()> {
    let fixture = setup_two_project_panes("shell-pane-scope.db")?;
    let db = Database::initialize_at(fixture.path)?;

    db.with_connection(|connection| {
        let panes = PaneRepository::list_shell_open(connection)?;
        assert_eq!(panes.len(), 2);
        assert!(pane_ids(&panes).contains(&fixture.pepfox_pane_id));
        assert!(pane_ids(&panes).contains(&fixture.arete_pane_id));
        assert!(panes
            .iter()
            .all(|pane| pane.workspace_id == SHELL_WORKSPACE_ID));
        Ok(())
    })
}

#[test]
fn messages_do_not_leak_across_panes() -> StorageResult<()> {
    let fixture = setup_two_project_panes("pane-message-scope.db")?;
    let db = Database::initialize_at(fixture.path)?;

    db.with_connection(|connection| {
        let pepfox_messages =
            MessageRepository::list_for_pane(connection, &fixture.pepfox_pane_id)?;
        let arete_messages =
            MessageRepository::list_for_pane(connection, &fixture.arete_pane_id)?;

        assert_eq!(pepfox_messages.len(), 1);
        assert_eq!(pepfox_messages[0].content, "pepfox-message");
        assert_eq!(arete_messages.len(), 1);
        assert_eq!(arete_messages[0].content, "arete-message");
        assert_eq!(pepfox_messages[0].workspace_id, SHELL_WORKSPACE_ID);
        assert_eq!(arete_messages[0].workspace_id, SHELL_WORKSPACE_ID);
        Ok(())
    })
}

#[test]
fn filesystem_scopes_by_project_id() -> StorageResult<()> {
    let fixture = setup_two_project_panes("pane-fs-scope.db")?;
    let db = Database::initialize_at(fixture.path)?;

    let pepfox_package = filesystem_read_file_with_database(
        &db,
        None,
        Some(&fixture.pepfox_project_id),
        "package.json",
    )
    .expect("read pepfox package");
    assert!(pepfox_package.content.contains("pepfox"));

    let arete_package = filesystem_read_file_with_database(
        &db,
        None,
        Some(&fixture.arete_project_id),
        "package.json",
    )
    .expect("read arete package");
    assert!(arete_package.content.contains("arete"));

    let cross_read = filesystem_read_file_with_database(
        &db,
        None,
        Some(&fixture.pepfox_project_id),
        "../Arete/package.json",
    );
    assert!(cross_read.is_err());

    Ok(())
}

#[test]
fn pane_close_does_not_affect_other_project_panes() -> StorageResult<()> {
    let fixture = setup_two_project_panes("pane-close-isolation.db")?;
    let db = Database::initialize_at(fixture.path)?;

    db.with_connection(|connection| {
        PaneRepository::close(connection, &fixture.pepfox_pane_id)?;
        let panes = PaneRepository::list_shell_open(connection)?;
        assert_eq!(panes.len(), 1);
        assert!(!pane_ids(&panes).contains(&fixture.pepfox_pane_id));
        assert!(pane_ids(&panes).contains(&fixture.arete_pane_id));
        Ok(())
    })
}

#[test]
fn accounts_and_credentials_unaffected_by_pane_operations() -> StorageResult<()> {
    let fixture = setup_two_project_panes("pane-account-isolation.db")?;
    let db = Database::initialize_at(fixture.path)?;
    let credentials = in_memory_credentials();

    account_create_api_key_with_service(
        &db,
        &credentials,
        "openai".to_string(),
        "OpenAI Work".to_string(),
        "test-api-key".to_string(),
        Some(true),
    )?;

    db.with_connection(|connection| {
        PaneRepository::close(connection, &fixture.arete_pane_id)?;
        let panes = PaneRepository::list_shell_open(connection)?;
        assert_eq!(panes.len(), 1);
        Ok(())
    })
}