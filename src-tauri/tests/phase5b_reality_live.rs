use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use builderboard_lib::auth::oauth_service::resolve_openai_credentials;
use builderboard_lib::auth::{CredentialService, OAuthService};
use builderboard_lib::project_scope_cache::ProjectScopeCache;
use builderboard_lib::projects::repository::ProjectRepository;
use builderboard_lib::storage::commands::message_create_with_database;
use builderboard_lib::stream_execution::stream_chat_with_services;
use builderboard_lib::stream_persistence::StreamPersistenceService;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::error::StorageResult;
use builderboard_lib::storage::models::{CreatePaneRequest, MessageCreateRequest};
use builderboard_lib::storage::repositories::accounts::AccountRepository;
use builderboard_lib::storage::repositories::messages::MessageRepository;
use builderboard_lib::storage::repositories::panes::PaneRepository;
use builderboard_lib::storage::repositories::workspaces::WorkspaceRepository;

fn test_database_path(name: &str) -> StorageResult<PathBuf> {
    let base = std::env::temp_dir().join("builderboard-tests");
    fs::create_dir_all(&base)?;
    Ok(base.join(name))
}

#[test]
fn live_openai_oauth_uses_bundled_chatgpt_client_without_env() {
    let result = resolve_openai_credentials("openai");
    println!(
        "PHASE5B_OAUTH_CREDS_RESOLVE={}",
        if result.is_ok() {
            "ok"
        } else {
            "unexpected_fail"
        }
    );
    let credentials = result.expect("OpenAI ChatGPT login should not require env credentials");
    println!("PHASE5B_OAUTH_CLIENT_ID={}", credentials.client_id);
    println!(
        "PHASE5B_OAUTH_CLIENT_SECRET_EMPTY={}",
        credentials.client_secret.is_empty()
    );

    assert_eq!(credentials.client_id, "app_EMoamEEZ73f0CkXaXp7hrann");
    assert!(credentials.client_secret.is_empty());
}

#[test]
#[ignore = "requires real BuilderBoard OpenAI API-key account and macOS Keychain access"]
fn live_model_and_reasoning_persist_on_stream_chat() -> StorageResult<()> {
    let database = Arc::new(Database::initialize_default()?);
    let credentials = CredentialService::keychain();
    let stream_persistence = Arc::new(StreamPersistenceService::new(Arc::clone(&database)));
    let scope_cache = ProjectScopeCache::new();

    let account_id = database.with_connection(|connection| {
        let account = AccountRepository::list_active(connection, Some("openai"))?
            .into_iter()
            .find(|account| account.auth_type == "api_key")
            .ok_or_else(|| {
                builderboard_lib::storage::error::StorageError::NotFound(
                    "active OpenAI API-key account not found".to_string(),
                )
            })?;
        Ok(account.id)
    })?;

    let app = tauri::test::mock_builder()
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("mock tauri app should build");

    let scenarios = [
        ("gpt-5.5", "high"),
        ("gpt-5.4-mini", "medium"),
        ("gpt-5.3-codex-spark", "low"),
    ];

    for (model_id, reasoning_level) in scenarios {
        let pane_id = database.with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: None,
                    title: Some(format!("Phase5B {model_id}")),
                    sort_order: None,
                },
            )?;
            Ok(pane.id)
        })?;

        let turn = builderboard_lib::storage::commands::message_create_with_database(
            &database,
            MessageCreateRequest {
                pane_id: pane_id.clone(),
                content: format!("ping {model_id}"),
                content_type: Some("text".to_string()),
                metadata_json: Some(
                    serde_json::json!({
                        "modelId": model_id,
                        "reasoningLevel": reasoning_level,
                    })
                    .to_string(),
                ),
            },
        )?;

        stream_chat_with_services(
            app.handle(),
            database.as_ref(),
            &credentials,
            &OAuthService::production(),
            &stream_persistence,
            &scope_cache,
            &pane_id,
            "openai",
            &account_id,
            model_id,
            &turn.assistant_message.id,
            Some(reasoning_level),
        )
        .map_err(|error| {
            builderboard_lib::storage::error::StorageError::InvalidInput(format!(
                "stream_chat failed for {model_id}: {error}"
            ))
        })?;

        database.with_connection(|connection| {
            let pane = PaneRepository::get_by_id(connection, &pane_id)?;
            let metadata: serde_json::Value =
                serde_json::from_str(pane.metadata_json.as_deref().unwrap_or("{}"))
                    .unwrap_or_default();

            println!(
                "PHASE5B_MODEL_PERSISTED model={} stored={}",
                model_id,
                pane.model_id.as_deref().unwrap_or("")
            );
            println!(
                "PHASE5B_REASONING_PERSISTED model={} stored={}",
                model_id,
                metadata
                    .get("reasoningLevel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
            );

            assert_eq!(pane.model_id.as_deref(), Some(model_id));
            assert_eq!(
                metadata.get("reasoningLevel").and_then(|v| v.as_str()),
                Some(reasoning_level)
            );
            Ok(())
        })?;
    }

    Ok(())
}

#[test]
#[ignore = "requires real BuilderBoard OpenAI OAuth account, macOS Keychain, and ChatGPT network access"]
fn final_openai_oauth_execution_trace_hello() -> StorageResult<()> {
    std::env::set_var("BUILDERBOARD_TRACE_OPENAI_EXECUTION", "1");

    let result = (|| -> StorageResult<()> {
        let database = Arc::new(Database::initialize_default()?);
        let credentials = CredentialService::keychain();
        let stream_persistence = Arc::new(StreamPersistenceService::new(Arc::clone(&database)));
        let scope_cache = ProjectScopeCache::new();
        let model_id = "gpt-5.3-codex-spark";

        let account_id = database.with_connection(|connection| {
            let account = AccountRepository::list_active(connection, Some("openai"))?
                .into_iter()
                .find(|account| {
                    account.auth_type == "oauth"
                        && AccountRepository::credential_ref(connection, &account.id)
                            .and_then(|credential_ref| {
                                credentials.credential_exists(&credential_ref)
                            })
                            .unwrap_or(false)
                })
                .ok_or_else(|| {
                    builderboard_lib::storage::error::StorageError::NotFound(
                        "active OpenAI OAuth account with Keychain credential not found"
                            .to_string(),
                    )
                })?;
            Ok(account.id)
        })?;

        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock tauri app should build");

        let pane_id = database.with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: None,
                    title: Some("Final OpenAI OAuth trace".to_string()),
                    sort_order: None,
                },
            )?;
            Ok(pane.id)
        })?;

        let turn = message_create_with_database(
            database.as_ref(),
            MessageCreateRequest {
                pane_id: pane_id.clone(),
                content: "Hello".to_string(),
                content_type: Some("text".to_string()),
                metadata_json: Some(
                    serde_json::json!({
                        "providerId": "openai",
                        "accountId": account_id,
                        "modelId": model_id,
                    })
                    .to_string(),
                ),
            },
        )?;

        println!("FINAL_EXECUTED_USER_MESSAGE=Hello");
        println!("FINAL_ASSISTANT_MESSAGE_ID={}", turn.assistant_message.id);

        stream_chat_with_services(
            app.handle(),
            database.as_ref(),
            &credentials,
            &OAuthService::production(),
            &stream_persistence,
            &scope_cache,
            &pane_id,
            "openai",
            &account_id,
            model_id,
            &turn.assistant_message.id,
            None,
        )
        .map_err(|error| {
            builderboard_lib::storage::error::StorageError::InvalidInput(format!(
                "stream_chat failed: {error}"
            ))
        })?;

        let assistant = database.with_connection(|connection| {
            MessageRepository::get_by_id(connection, &turn.assistant_message.id)
        })?;

        println!("FINAL_ASSISTANT_STATUS={}", assistant.status);
        println!("FINAL_RESPONSE={}", assistant.content.replace('\n', " "));

        assert_eq!(assistant.status, "complete");
        assert!(!assistant.content.trim().is_empty());
        Ok(())
    })();

    std::env::remove_var("BUILDERBOARD_TRACE_OPENAI_EXECUTION");
    result
}

#[test]
#[ignore = "requires real BuilderBoard OpenAI OAuth account, macOS Keychain, ChatGPT network access, and /Users/sterlingdigital/PepFox"]
fn live_filesystem_tool_loop_trace_pepfox() -> StorageResult<()> {
    std::env::set_var("BUILDERBOARD_TRACE_OPENAI_EXECUTION", "1");

    let result = (|| -> StorageResult<()> {
        let database = Arc::new(Database::initialize_default()?);
        let credentials = CredentialService::keychain();
        let stream_persistence = Arc::new(StreamPersistenceService::new(Arc::clone(&database)));
        let scope_cache = ProjectScopeCache::new();
        let model_id = "gpt-5.3-codex-spark";

        let account_id = database.with_connection(|connection| {
            let account = AccountRepository::list_active(connection, Some("openai"))?
                .into_iter()
                .find(|account| {
                    account.auth_type == "oauth"
                        && AccountRepository::credential_ref(connection, &account.id)
                            .and_then(|credential_ref| {
                                credentials.credential_exists(&credential_ref)
                            })
                            .unwrap_or(false)
                })
                .ok_or_else(|| {
                    builderboard_lib::storage::error::StorageError::NotFound(
                        "active OpenAI OAuth account with Keychain credential not found"
                            .to_string(),
                    )
                })?;
            Ok(account.id)
        })?;

        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock tauri app should build");

        let (pane_id, approved_root) = database.with_connection(|connection| {
            let project = ProjectRepository::list(connection)?
                .into_iter()
                .find(|project| project.approved_root.contains("PepFox"))
                .ok_or_else(|| {
                    builderboard_lib::storage::error::StorageError::NotFound(
                        "PepFox project not found for filesystem trace".to_string(),
                    )
                })?;
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(project.id.clone()),
                    title: Some("Filesystem trace PepFox".to_string()),
                    sort_order: None,
                },
            )?;
            Ok((pane.id, project.approved_root))
        })?;

        let prompt = "take a look at /Users/sterlingdigital/PepFox";
        let turn = message_create_with_database(
            database.as_ref(),
            MessageCreateRequest {
                pane_id: pane_id.clone(),
                content: prompt.to_string(),
                content_type: Some("text".to_string()),
                metadata_json: Some(
                    serde_json::json!({
                        "providerId": "openai",
                        "accountId": account_id,
                        "modelId": model_id,
                    })
                    .to_string(),
                ),
            },
        )?;

        println!("LIVE_FILESYSTEM_PROMPT={prompt}");
        println!("LIVE_FILESYSTEM_APPROVED_ROOT={approved_root}");
        println!(
            "LIVE_FILESYSTEM_ASSISTANT_MESSAGE_ID={}",
            turn.assistant_message.id
        );

        stream_chat_with_services(
            app.handle(),
            database.as_ref(),
            &credentials,
            &OAuthService::production(),
            &stream_persistence,
            &scope_cache,
            &pane_id,
            "openai",
            &account_id,
            model_id,
            &turn.assistant_message.id,
            None,
        )
        .map_err(|error| {
            builderboard_lib::storage::error::StorageError::InvalidInput(format!(
                "stream_chat failed: {error}"
            ))
        })?;

        let assistant = database.with_connection(|connection| {
            MessageRepository::get_by_id(connection, &turn.assistant_message.id)
        })?;

        println!("LIVE_FILESYSTEM_ASSISTANT_STATUS={}", assistant.status);
        println!(
            "LIVE_FILESYSTEM_RESPONSE={}",
            assistant.content.replace('\n', " ")
        );

        assert_eq!(assistant.status, "complete");
        assert!(!assistant.content.trim().is_empty());
        Ok(())
    })();

    std::env::remove_var("BUILDERBOARD_TRACE_OPENAI_EXECUTION");
    result
}

#[test]
fn live_workspace_isolation_with_model_metadata_restart() -> StorageResult<()> {
    let path = test_database_path("phase5b-workspace-model-restart.db")?;
    let _ = fs::remove_file(&path);

    let (_project_a, _project_b, pane_a_id, pane_b_id) =
        {
            let db = Database::initialize_at(path.clone())?;
            db.with_connection(|connection| {
                let project_a = {
                    let root = std::env::temp_dir().join("builderboard-phase5b-a");
                    let _ = fs::remove_dir_all(&root);
                    fs::create_dir_all(&root)?;
                    ProjectRepository::create_from_folder(connection, &root.display().to_string(), true)?
                        .id
                };
                let project_b = {
                    let root = std::env::temp_dir().join("builderboard-phase5b-b");
                    let _ = fs::remove_dir_all(&root);
                    fs::create_dir_all(&root)?;
                    ProjectRepository::create_from_folder(connection, &root.display().to_string(), true)?
                        .id
                };

                let pane_a = PaneRepository::create(
                    connection,
                    CreatePaneRequest {
                        workspace_id: None,
                        project_id: Some(project_a.clone()),
                        title: Some("Pane A".to_string()),
                        sort_order: Some(0),
                    },
                )?;
                let pane_b = PaneRepository::create(
                    connection,
                    CreatePaneRequest {
                        workspace_id: None,
                        project_id: Some(project_b.clone()),
                        title: Some("Pane B".to_string()),
                        sort_order: Some(1),
                    },
                )?;

                let now = chrono::Utc::now().to_rfc3339();
                connection.execute(
                "UPDATE panes SET model_id = ?1, metadata_json = ?2, updated_at = ?3 WHERE id = ?4",
                ("gpt-5.5", r#"{"reasoningLevel":"high"}"#, &now, &pane_a.id),
            )?;
                connection.execute(
                "UPDATE panes SET model_id = ?1, metadata_json = ?2, updated_at = ?3 WHERE id = ?4",
                ("gpt-5.3-codex-spark", r#"{"reasoningLevel":"low"}"#, &now, &pane_b.id),
            )?;

                Ok((project_a, project_b, pane_a.id, pane_b.id))
            })?
        };

    let db = Database::initialize_at(path)?;
    db.with_connection(|connection| {
        let panes = PaneRepository::list_shell_open(connection)?;
        assert!(panes.len() >= 2);

        let pane_a = panes
            .iter()
            .find(|pane| pane.id == pane_a_id)
            .expect("pane a");
        let pane_b = panes
            .iter()
            .find(|pane| pane.id == pane_b_id)
            .expect("pane b");
        assert_eq!(pane_a.model_id.as_deref(), Some("gpt-5.5"));
        assert_eq!(pane_b.model_id.as_deref(), Some("gpt-5.3-codex-spark"));

        let meta_a: serde_json::Value =
            serde_json::from_str(pane_a.metadata_json.as_deref().unwrap_or("{}"))?;
        let meta_b: serde_json::Value =
            serde_json::from_str(pane_b.metadata_json.as_deref().unwrap_or("{}"))?;
        assert_eq!(meta_a["reasoningLevel"], "high");
        assert_eq!(meta_b["reasoningLevel"], "low");

        println!("PHASE5B_SHELL_PANE_ISOLATION=model_and_reasoning_preserved_after_restart");
        Ok(())
    })
}
