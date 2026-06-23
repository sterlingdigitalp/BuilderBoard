use builderboard_lib::auth::CredentialService;
use builderboard_lib::chat::{ChatExecutionError, ChatExecutionService};
use builderboard_lib::storage::db::Database;
use builderboard_lib::storage::error::{StorageError, StorageResult};
use builderboard_lib::storage::models::CreatePaneRequest;
use builderboard_lib::storage::repositories::accounts::AccountRepository;
use builderboard_lib::storage::repositories::messages::MessageRepository;
use builderboard_lib::storage::repositories::panes::PaneRepository;

#[test]
#[ignore = "requires real BuilderBoard database account and macOS Keychain access"]
fn live_openai_streaming_execution_persists_response() -> StorageResult<()> {
    let database = Database::initialize_default()?;
    let credentials = CredentialService::keychain();

    database.with_connection(|connection| {
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

        let pane = PaneRepository::create(
            connection,
            CreatePaneRequest {
                workspace_id: None,
                title: Some("Phase 4B OpenAI live validation".to_string()),
                sort_order: None,
            },
        )?;
        let credential_ref = AccountRepository::credential_ref(connection, &account.id)?;
        let credential_exists = credentials.credential_exists(&credential_ref)?;
        let read_api_key_result = credentials
            .read_api_key(&credential_ref)
            .map(|api_key| !api_key.trim().is_empty());

        println!("PHASE4B_CREDENTIAL_EXISTS={credential_exists}");
        println!(
            "PHASE4B_READ_API_KEY_RESULT={}",
            match &read_api_key_result {
                Ok(true) => "Ok(non_empty)",
                Ok(false) => "Ok(empty)",
                Err(_) => "Err(redacted)",
            }
        );

        assert!(credential_exists);
        assert_eq!(read_api_key_result.map_err(format_storage_error)?, true);

        connection.execute(
            "UPDATE panes SET provider_id = 'openai', account_id = ?1 WHERE id = ?2",
            (&account.id, &pane.id),
        )?;

        let assistant = ChatExecutionService::stream_openai_message(
            connection,
            &pane.id,
            "Hello".to_string(),
            &credentials,
        )
        .map_err(format_chat_execution_error)?;

        let messages = MessageRepository::list_for_pane(connection, &pane.id)?;
        let response_preview: String = assistant.content.chars().take(240).collect();

        println!("PHASE4B_OPENAI_STATUS=200");
        println!("PHASE4B_PANE_ID={}", pane.id);
        println!("PHASE4B_ACCOUNT_ID={}", account.id);
        println!("PHASE4B_ASSISTANT_MESSAGE_ID={}", assistant.id);
        println!("PHASE4B_ASSISTANT_STATUS={}", assistant.status);
        println!("PHASE4B_MESSAGE_COUNT={}", messages.len());
        println!(
            "PHASE4B_RESPONSE_PREVIEW={}",
            response_preview.replace('\n', " ")
        );

        assert_eq!(assistant.status, "complete");
        assert!(!assistant.content.trim().is_empty());
        assert!(messages
            .iter()
            .any(|message| message.role == "user" && message.content == "Hello"));
        assert!(messages.iter().any(|message| {
            message.role == "assistant"
                && message.status == "complete"
                && !message.content.trim().is_empty()
        }));

        Ok(())
    })
}

fn format_chat_execution_error(error: ChatExecutionError) -> StorageError {
    StorageError::InvalidInput(format!("chat execution error: {error:?}"))
}

fn format_storage_error(error: StorageError) -> StorageError {
    StorageError::InvalidInput(format!("credential lookup error: {error}"))
}
