use std::sync::Arc;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::auth::CredentialService;
use crate::chat::ProviderResolutionService;
use crate::models::{Conversation, Message, MessageRole, Model};
use crate::providers::{ProviderRequest, StreamChunk};
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::{
    AccountDto, AccountStatusDto, AppendMessageRequest, CreatePaneRequest, MessageCompleteRequest,
    MessageCreateRequest, MessageCreateResult, MessageDto, MessageErrorRequest,
    MessageStreamUpdateRequest, PaneDto, ProviderDto,
};
use crate::storage::repositories::accounts::AccountRepository;
use crate::storage::repositories::messages::MessageRepository;
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::providers::ProviderRepository;

#[tauri::command]
pub fn provider_list(database: State<'_, Arc<Database>>) -> Result<Vec<ProviderDto>, String> {
    provider_list_from_database(database.inner())
}

pub fn provider_list_from_database(database: &Database) -> Result<Vec<ProviderDto>, String> {
    database
        .with_connection(ProviderRepository::list_enabled)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn pane_list(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
) -> Result<Vec<PaneDto>, String> {
    database
        .with_connection(|connection| {
            PaneRepository::list_open(connection, workspace_id.as_deref())
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_create(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    title: Option<String>,
    sort_order: Option<i32>,
) -> Result<PaneDto, String> {
    database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id,
                    title,
                    sort_order,
                },
            )
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_close(database: State<'_, Arc<Database>>, pane_id: String) -> Result<(), String> {
    database
        .with_connection(|connection| PaneRepository::close(connection, &pane_id))
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_list(
    database: State<'_, Arc<Database>>,
    pane_id: String,
) -> Result<Vec<MessageDto>, String> {
    database
        .with_connection(|connection| MessageRepository::list_for_pane(connection, &pane_id))
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_append(
    database: State<'_, Arc<Database>>,
    pane_id: String,
    role: String,
    content: String,
    content_type: Option<String>,
    metadata_json: Option<String>,
) -> Result<MessageDto, String> {
    database
        .with_connection(|connection| {
            MessageRepository::append(
                connection,
                AppendMessageRequest {
                    pane_id,
                    role,
                    content,
                    content_type,
                    metadata_json,
                },
            )
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_create(
    database: State<'_, Arc<Database>>,
    pane_id: String,
    content: String,
    content_type: Option<String>,
    metadata_json: Option<String>,
) -> Result<MessageCreateResult, String> {
    message_create_with_database(
        database.inner(),
        MessageCreateRequest {
            pane_id,
            content,
            content_type,
            metadata_json,
        },
    )
    .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_stream_update(
    database: State<'_, Arc<Database>>,
    message_id: String,
    delta: String,
) -> Result<MessageDto, String> {
    database
        .with_connection(|connection| {
            MessageRepository::stream_update(
                connection,
                MessageStreamUpdateRequest { message_id, delta },
            )
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_complete(
    database: State<'_, Arc<Database>>,
    message_id: String,
    content: Option<String>,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
    metadata_json: Option<String>,
) -> Result<MessageDto, String> {
    database
        .with_connection(|connection| {
            MessageRepository::mark_complete(
                connection,
                MessageCompleteRequest {
                    message_id,
                    content,
                    token_count_input,
                    token_count_output,
                    metadata_json,
                },
            )
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn message_error(
    database: State<'_, Arc<Database>>,
    message_id: String,
    error_code: String,
    error_message: String,
) -> Result<MessageDto, String> {
    database
        .with_connection(|connection| {
            MessageRepository::mark_error(
                connection,
                MessageErrorRequest {
                    message_id,
                    error_code,
                    error_message,
                },
            )
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn stream_chat(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    credentials: State<'_, Arc<CredentialService>>,
    pane_id: String,
    provider_id: String,
    account_id: String,
    model_id: String,
    assistant_message_id: String,
) -> Result<(), String> {
    if provider_id != "openai" {
        let message = "Only OpenAI execution is supported in Phase 4B.".to_string();
        emit_stream_error(
            &app,
            &pane_id,
            &assistant_message_id,
            "unsupported_provider",
            &message,
        );
        return Err(message);
    }

    match stream_chat_with_services(
        &app,
        database.inner(),
        credentials.inner(),
        &pane_id,
        &provider_id,
        &account_id,
        &model_id,
        &assistant_message_id,
    ) {
        Ok(()) => Ok(()),
        Err(error) => {
            let message = error.to_string();
            let _ = database.with_connection(|connection| {
                MessageRepository::mark_error(
                    connection,
                    MessageErrorRequest {
                        message_id: assistant_message_id.clone(),
                        error_code: "provider_execution_failed".to_string(),
                        error_message: message.clone(),
                    },
                )
            });
            emit_stream_error(
                &app,
                &pane_id,
                &assistant_message_id,
                "provider_execution_failed",
                &message,
            );
            Err(message)
        }
    }
}

pub fn stream_chat_with_services<R: Runtime>(
    app: &AppHandle<R>,
    database: &Database,
    credentials: &CredentialService,
    pane_id: &str,
    provider_id: &str,
    account_id: &str,
    model_id: &str,
    assistant_message_id: &str,
) -> Result<(), StorageError> {
    database.with_connection(|connection| {
        PaneRepository::get_open_by_id(connection, pane_id)?;
        let now = chrono::Utc::now().to_rfc3339();
        let updated = connection.execute(
            "UPDATE panes
             SET provider_id = ?1,
                 account_id = ?2,
                 model_id = ?3,
                 updated_at = ?4
             WHERE id = ?5 AND closed_at IS NULL",
            (provider_id, account_id, model_id, &now, pane_id),
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!(
                "open pane {pane_id} not found"
            )));
        }

        let resolved =
            ProviderResolutionService::resolve_for_pane_execution(connection, pane_id, credentials)
                .map_err(|error| {
                    StorageError::InvalidInput(format!("provider resolution error: {error:?}"))
                })?;
        let conversation = conversation_for_stream(connection, pane_id, model_id)?;
        let stream = resolved
            .provider
            .stream(ProviderRequest::new(conversation))
            .map_err(|error| {
                StorageError::InvalidInput(format!("provider stream error: {error:?}"))
            })?;

        for chunk in stream {
            match chunk {
                Ok(chunk) => {
                    apply_stream_chunk(app, connection, pane_id, assistant_message_id, chunk)?
                }
                Err(error) => {
                    let message = format!("{error:?}");
                    let _ = MessageRepository::mark_error(
                        connection,
                        MessageErrorRequest {
                            message_id: assistant_message_id.to_string(),
                            error_code: "provider_error".to_string(),
                            error_message: message.clone(),
                        },
                    );
                    return Err(StorageError::InvalidInput(message));
                }
            }
        }

        let latest = MessageRepository::get_by_id(connection, assistant_message_id)?;
        if latest.status != "complete" {
            MessageRepository::mark_complete(
                connection,
                MessageCompleteRequest {
                    message_id: assistant_message_id.to_string(),
                    content: None,
                    token_count_input: None,
                    token_count_output: None,
                    metadata_json: None,
                },
            )?;
            emit_stream_complete(app, pane_id, assistant_message_id);
        }

        Ok(())
    })
}

fn apply_stream_chunk<R: Runtime>(
    app: &AppHandle<R>,
    connection: &rusqlite::Connection,
    pane_id: &str,
    assistant_message_id: &str,
    chunk: StreamChunk,
) -> Result<(), StorageError> {
    if chunk.is_complete {
        MessageRepository::mark_complete(
            connection,
            MessageCompleteRequest {
                message_id: assistant_message_id.to_string(),
                content: None,
                token_count_input: None,
                token_count_output: None,
                metadata_json: None,
            },
        )?;
        emit_stream_complete(app, pane_id, assistant_message_id);
    } else if !chunk.content_delta.is_empty() {
        MessageRepository::stream_update(
            connection,
            MessageStreamUpdateRequest {
                message_id: assistant_message_id.to_string(),
                delta: chunk.content_delta.clone(),
            },
        )?;
        emit_stream_chunk(app, pane_id, assistant_message_id, &chunk.content_delta);
    }

    Ok(())
}

fn conversation_for_stream(
    connection: &rusqlite::Connection,
    pane_id: &str,
    model_id: &str,
) -> Result<Conversation, StorageError> {
    let mut conversation = Conversation::new(pane_id, model_from_id(model_id));
    for message in MessageRepository::list_for_pane(connection, pane_id)? {
        if message.role == "assistant" && message.status == "pending" && message.content.is_empty()
        {
            continue;
        }
        let role = match message.role.as_str() {
            "system" => Some(MessageRole::System),
            "user" => Some(MessageRole::User),
            "assistant" => Some(MessageRole::Assistant),
            _ => None,
        };
        if let Some(role) = role {
            conversation = conversation.with_message(Message::new(role, message.content));
        }
    }
    Ok(conversation)
}

fn model_from_id(model_id: &str) -> Model {
    match model_id {
        "OpenAIGpt" | "gpt-4o-mini" => Model::OpenAIGpt,
        other => Model::Custom(other.to_string()),
    }
}

fn emit_stream_chunk<R: Runtime>(app: &AppHandle<R>, pane_id: &str, message_id: &str, delta: &str) {
    let _ = app.emit(
        "message_stream_chunk",
        serde_json::json!({
            "paneId": pane_id,
            "messageId": message_id,
            "delta": delta,
        }),
    );
}

fn emit_stream_complete<R: Runtime>(app: &AppHandle<R>, pane_id: &str, message_id: &str) {
    let _ = app.emit(
        "message_stream_complete",
        serde_json::json!({
            "paneId": pane_id,
            "messageId": message_id,
        }),
    );
}

fn emit_stream_error<R: Runtime>(
    app: &AppHandle<R>,
    pane_id: &str,
    message_id: &str,
    error_code: &str,
    message: &str,
) {
    let _ = app.emit(
        "message_stream_error",
        serde_json::json!({
            "paneId": pane_id,
            "messageId": message_id,
            "errorCode": error_code,
            "message": message,
        }),
    );
}

pub fn message_create_with_database(
    database: &Database,
    request: MessageCreateRequest,
) -> Result<MessageCreateResult, StorageError> {
    database.with_connection(|connection| {
        MessageRepository::create_conversation_turn(connection, request)
    })
}

#[tauri::command]
pub fn account_create_api_key(
    database: State<'_, Arc<Database>>,
    credentials: State<'_, Arc<CredentialService>>,
    provider_id: String,
    label: String,
    api_key: String,
    is_default: Option<bool>,
) -> Result<AccountDto, String> {
    account_create_api_key_with_service(
        database.inner(),
        credentials.inner(),
        provider_id,
        label,
        api_key,
        is_default,
    )
    .map_err(format_storage_error)
}

#[tauri::command]
pub fn account_list(
    database: State<'_, Arc<Database>>,
    provider_id: Option<String>,
) -> Result<Vec<AccountDto>, String> {
    account_list_from_database(database.inner(), provider_id).map_err(format_storage_error)
}

#[tauri::command]
pub fn account_disconnect(
    database: State<'_, Arc<Database>>,
    credentials: State<'_, Arc<CredentialService>>,
    account_id: String,
) -> Result<(), String> {
    account_disconnect_with_service(database.inner(), credentials.inner(), account_id)
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn account_get_status(
    database: State<'_, Arc<Database>>,
    account_id: String,
) -> Result<AccountStatusDto, String> {
    database
        .with_connection(|connection| AccountRepository::get_status(connection, &account_id))
        .map_err(format_storage_error)
}

pub fn account_create_api_key_with_service(
    database: &Database,
    credentials: &CredentialService,
    provider_id: String,
    label: String,
    api_key: String,
    is_default: Option<bool>,
) -> Result<AccountDto, StorageError> {
    let credential_ref = CredentialService::generate_credential_ref();

    credentials.store_api_key(&credential_ref, &label, &provider_id, &api_key)?;

    match database.with_connection(|connection| {
        AccountRepository::create_api_key_account(
            connection,
            &provider_id,
            &label,
            &credential_ref,
            is_default.unwrap_or(false),
        )
    }) {
        Ok(account) => Ok(account),
        Err(error) => {
            let _ = credentials.delete_credential(&credential_ref);
            Err(error)
        }
    }
}

pub fn account_list_from_database(
    database: &Database,
    provider_id: Option<String>,
) -> Result<Vec<AccountDto>, StorageError> {
    database.with_connection(|connection| {
        AccountRepository::list_active(connection, provider_id.as_deref())
    })
}

pub fn account_disconnect_with_service(
    database: &Database,
    credentials: &CredentialService,
    account_id: String,
) -> Result<(), StorageError> {
    let credential_ref = database
        .with_connection(|connection| AccountRepository::revoke(connection, &account_id))?;

    credentials.delete_credential(&credential_ref)
}

fn format_storage_error(error: StorageError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        account_create_api_key_with_service, account_disconnect_with_service,
        account_list_from_database, provider_list_from_database,
    };
    use crate::auth::CredentialService;
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::error::StorageResult;
    use crate::storage::repositories::accounts::AccountRepository;

    #[test]
    fn provider_list_returns_seeded_providers() -> StorageResult<()> {
        let path = test_database_path("provider-list-command.db")?;
        let _ = fs::remove_file(&path);
        let db = Database::initialize_at(path)?;

        let providers = provider_list_from_database(&db).expect("provider_list should succeed");
        let provider_ids: Vec<_> = providers
            .iter()
            .map(|provider| provider.id.as_str())
            .collect();

        assert_eq!(providers.len(), 3);
        assert!(provider_ids.contains(&"anthropic"));
        assert!(provider_ids.contains(&"openai"));
        assert!(provider_ids.contains(&"google"));
        Ok(())
    }

    fn setup_services(name: &str) -> StorageResult<(Database, CredentialService)> {
        let path = test_database_path(name)?;
        let _ = fs::remove_file(&path);
        let database = Database::initialize_at(path)?;
        let credentials = CredentialService::in_memory();
        Ok((database, credentials))
    }

    fn sqlite_contains_api_key(database: &Database, api_key: &str) -> StorageResult<bool> {
        database.with_connection(|connection| {
            let mut statement =
                connection.prepare("SELECT label, credential_ref, status FROM accounts")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for row in rows {
                let (label, credential_ref, status) = row?;
                if label.contains(api_key)
                    || credential_ref.contains(api_key)
                    || status.contains(api_key)
                {
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }

    #[test]
    fn create_openai_anthropic_and_google_accounts() -> StorageResult<()> {
        let (database, credentials) = setup_services("account-create-providers.db")?;

        for (provider_id, label, api_key) in [
            ("openai", "OpenAI Work", "sk-openai-test"),
            ("anthropic", "Anthropic Work", "sk-ant-test"),
            ("google", "Google Work", "sk-google-test"),
        ] {
            let account = account_create_api_key_with_service(
                &database,
                &credentials,
                provider_id.to_string(),
                label.to_string(),
                api_key.to_string(),
                Some(true),
            )?;
            assert_eq!(account.provider_id, provider_id);
            assert_eq!(account.auth_type, "api_key");
            assert!(account.is_default);

            let credential_ref = database.with_connection(|connection| {
                AccountRepository::credential_ref(connection, &account.id)
            })?;
            assert!(credentials.credential_exists(&credential_ref)?);
        }

        let accounts = account_list_from_database(&database, None)?;
        assert_eq!(accounts.len(), 3);
        Ok(())
    }

    #[test]
    fn api_key_is_stored_in_keychain_not_sqlite() -> StorageResult<()> {
        let (database, credentials) = setup_services("account-keychain-only.db")?;
        let api_key = "sk-secret-not-in-sqlite";

        let account = account_create_api_key_with_service(
            &database,
            &credentials,
            "openai".to_string(),
            "Secret".to_string(),
            api_key.to_string(),
            None,
        )?;

        assert!(!sqlite_contains_api_key(&database, api_key)?);

        let credential_ref = database.with_connection(|connection| {
            AccountRepository::credential_ref(connection, &account.id)
        })?;
        assert!(credentials.credential_exists(&credential_ref)?);
        Ok(())
    }

    #[test]
    fn set_default_account_switches_provider_default() -> StorageResult<()> {
        let (database, credentials) = setup_services("account-default.db")?;

        let first = account_create_api_key_with_service(
            &database,
            &credentials,
            "openai".to_string(),
            "First".to_string(),
            "sk-first".to_string(),
            Some(true),
        )?;
        let second = account_create_api_key_with_service(
            &database,
            &credentials,
            "openai".to_string(),
            "Second".to_string(),
            "sk-second".to_string(),
            Some(false),
        )?;

        database
            .with_connection(|connection| AccountRepository::set_default(connection, &second.id))?;

        let accounts = account_list_from_database(&database, Some("openai".to_string()))?;
        let first = accounts
            .iter()
            .find(|account| account.id == first.id)
            .unwrap();
        let second = accounts
            .iter()
            .find(|account| account.id == second.id)
            .unwrap();
        assert!(!first.is_default);
        assert!(second.is_default);
        Ok(())
    }

    #[test]
    fn disconnect_removes_keychain_entry_and_revokes_status() -> StorageResult<()> {
        let (database, credentials) = setup_services("account-disconnect.db")?;

        let account = account_create_api_key_with_service(
            &database,
            &credentials,
            "anthropic".to_string(),
            "Disconnect Me".to_string(),
            "sk-disconnect".to_string(),
            None,
        )?;
        let credential_ref = database.with_connection(|connection| {
            AccountRepository::credential_ref(connection, &account.id)
        })?;
        assert!(credentials.credential_exists(&credential_ref)?);

        account_disconnect_with_service(&database, &credentials, account.id.clone())?;

        assert!(!credentials.credential_exists(&credential_ref)?);
        let status = database
            .with_connection(|connection| AccountRepository::get_status(connection, &account.id))?;
        assert_eq!(status.status, "revoked");
        Ok(())
    }
}
