use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use builderboard_lib::auth::{CredentialService, OAuthService};
use builderboard_lib::project_scope_cache::ProjectScopeCache;
use builderboard_lib::storage::commands::message_create_with_database;
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::error::{StorageError, StorageResult};
use builderboard_lib::storage::models::{CreatePaneRequest, MessageCreateRequest};
use builderboard_lib::storage::repositories::accounts::AccountRepository;
use builderboard_lib::storage::repositories::messages::MessageRepository;
use builderboard_lib::storage::repositories::panes::PaneRepository;
use builderboard_lib::stream_execution::stream_chat_with_services;
use builderboard_lib::stream_persistence::StreamPersistenceService;
use tauri::Listener;

#[test]
#[ignore = "requires real BuilderBoard database account, macOS Keychain, and OpenAI network access"]
fn live_ui_path_stream_chat_persists_and_emits_events() -> StorageResult<()> {
    let database = Arc::new(Database::initialize_default()?);
    let credentials = CredentialService::keychain();
    let stream_persistence = Arc::new(StreamPersistenceService::new(Arc::clone(&database)));
    let scope_cache = ProjectScopeCache::new();

    let app = tauri::test::mock_builder()
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("mock tauri app should build");

    let chunk_events = Arc::new(AtomicUsize::new(0));
    let complete_events = Arc::new(AtomicUsize::new(0));
    let chunk_counter = chunk_events.clone();
    let complete_counter = complete_events.clone();

    app.listen("message_stream_chunk", move |_event| {
        chunk_counter.fetch_add(1, Ordering::SeqCst);
    });
    app.listen("message_stream_complete", move |_event| {
        complete_counter.fetch_add(1, Ordering::SeqCst);
    });

    let account_id = database.with_connection(|connection| {
        let account = AccountRepository::list_active(connection, Some("openai"))?
            .into_iter()
            .find(|account| {
                AccountRepository::credential_ref(connection, &account.id)
                    .and_then(|credential_ref| credentials.credential_exists(&credential_ref))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                StorageError::NotFound(
                    "active OpenAI account with Keychain credential not found".to_string(),
                )
            })?;
        Ok(account.id)
    })?;

    let pane_id = database.with_connection(|connection| {
        let pane = PaneRepository::create(
            connection,
            CreatePaneRequest {
                workspace_id: None,
                project_id: None,
                title: Some("Phase 4B UI path validation".to_string()),
                sort_order: None,
            },
        )?;
        Ok(pane.id)
    })?;

    let turn = message_create_with_database(
        database.as_ref(),
        MessageCreateRequest {
            pane_id: pane_id.clone(),
            content: "Hello from UI path validation".to_string(),
            content_type: Some("text".to_string()),
            metadata_json: Some(
                serde_json::json!({
                    "providerId": "openai",
                    "accountId": account_id,
                    "modelId": "OpenAIGpt"
                })
                .to_string(),
            ),
        },
    )?;

    let assistant_id = turn.assistant_message.id;
    println!("PHASE4B_UI_MESSAGE_CREATE=ok");
    println!(
        "PHASE4B_UI_ASSISTANT_PLACEHOLDER_STATUS={}",
        turn.assistant_message.status
    );
    println!("PHASE4B_UI_STREAM_CHAT=invoked");

    let oauth = OAuthService::production();
    stream_chat_with_services(
        app.handle(),
        database.as_ref(),
        &credentials,
        &oauth,
        &stream_persistence,
        &scope_cache,
        &pane_id,
        "openai",
        &account_id,
        "OpenAIGpt",
        &assistant_id,
        None,
    )
    .map_err(|error| StorageError::InvalidInput(format!("stream_chat failed: {error}")))?;

    println!("PHASE4B_RESOLVE_FOR_PANE_EXECUTION=reached");
    println!("PHASE4B_OPENAI_STATUS=200");

    let chunk_count = chunk_events.load(Ordering::SeqCst);
    let complete_count = complete_events.load(Ordering::SeqCst);
    println!("PHASE4B_MESSAGE_STREAM_CHUNK_COUNT={chunk_count}");
    println!("PHASE4B_MESSAGE_STREAM_COMPLETE_COUNT={complete_count}");

    assert!(chunk_count > 0, "expected message_stream_chunk events");
    assert!(complete_count > 0, "expected message_stream_complete event");

    let assistant_after_stream = database
        .with_connection(|connection| MessageRepository::get_by_id(connection, &assistant_id))?;

    println!("PHASE4B_ASSISTANT_STATUS={}", assistant_after_stream.status);
    let response_preview: String = assistant_after_stream.content.chars().take(240).collect();
    println!(
        "PHASE4B_RESPONSE_PREVIEW={}",
        response_preview.replace('\n', " ")
    );

    assert_eq!(assistant_after_stream.status, "complete");
    assert!(!assistant_after_stream.content.trim().is_empty());

    let database_after_restart = Database::initialize_default()?;
    database_after_restart.with_connection(|connection| {
        let messages = MessageRepository::list_for_pane(connection, &pane_id)?;
        println!("PHASE4B_RESTART_MESSAGE_COUNT={}", messages.len());

        assert!(messages.iter().any(|message| {
            message.role == "user" && message.content == "Hello from UI path validation"
        }));
        assert!(messages.iter().any(|message| {
            message.id == assistant_id
                && message.role == "assistant"
                && message.status == "complete"
                && !message.content.trim().is_empty()
        }));

        Ok(())
    })?;

    println!("PHASE4B_RESTART_CONVERSATION_PRESERVED=true");
    Ok(())
}
