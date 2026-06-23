use chrono::Utc;
use rusqlite::Connection;

use crate::auth::{CredentialHandle, CredentialService};
use crate::models::{Conversation, Message, MessageRole, Model};
use crate::providers::{
    resolve_openai_provider_with_api_key, resolve_provider_with_credential, LLMProvider,
    ProviderError, ProviderRequest, ProviderResolutionError, ResolvedProvider,
};
use crate::storage::error::StorageError;
use crate::storage::models::{
    AccountDto, MessageCompleteRequest, MessageCreateRequest, MessageDto, MessageErrorRequest,
    MessageStreamUpdateRequest, ProviderDto,
};
use crate::storage::repositories::accounts::AccountRepository;
use crate::storage::repositories::messages::MessageRepository;
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::providers::ProviderRepository;

pub struct ProviderResolutionService;

#[derive(Debug)]
pub enum ChatExecutionError {
    Resolution(ProviderResolutionError),
    Provider(ProviderError),
    Storage(StorageError),
}

impl From<ProviderResolutionError> for ChatExecutionError {
    fn from(value: ProviderResolutionError) -> Self {
        Self::Resolution(value)
    }
}

impl From<ProviderError> for ChatExecutionError {
    fn from(value: ProviderError) -> Self {
        Self::Provider(value)
    }
}

impl From<StorageError> for ChatExecutionError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

pub struct ChatExecutionService;

impl ChatExecutionService {
    pub fn stream_openai_message(
        connection: &Connection,
        pane_id: &str,
        content: String,
        credentials: &CredentialService,
    ) -> Result<MessageDto, ChatExecutionError> {
        let resolved = ProviderResolutionService::resolve_for_pane_execution(
            connection,
            pane_id,
            credentials,
        )?;
        Self::stream_message_with_provider(connection, pane_id, content, resolved.provider.as_ref())
    }

    pub fn stream_message_with_provider(
        connection: &Connection,
        pane_id: &str,
        content: String,
        provider: &dyn LLMProvider,
    ) -> Result<MessageDto, ChatExecutionError> {
        let turn = MessageRepository::create_conversation_turn(
            connection,
            MessageCreateRequest {
                pane_id: pane_id.to_string(),
                content,
                content_type: None,
                metadata_json: None,
            },
        )?;
        let conversation = conversation_for_pane(connection, pane_id)?;
        let mut latest = turn.assistant_message;
        let stream = provider.stream(ProviderRequest::new(conversation))?;

        for chunk in stream {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    let _ = MessageRepository::mark_error(
                        connection,
                        MessageErrorRequest {
                            message_id: latest.id.clone(),
                            error_code: "provider_error".to_string(),
                            error_message: format!("{error:?}"),
                        },
                    );
                    return Err(ChatExecutionError::Provider(error));
                }
            };

            if chunk.is_complete {
                latest = MessageRepository::mark_complete(
                    connection,
                    MessageCompleteRequest {
                        message_id: latest.id,
                        content: None,
                        token_count_input: None,
                        token_count_output: None,
                        metadata_json: None,
                    },
                )?;
            } else if !chunk.content_delta.is_empty() {
                latest = MessageRepository::stream_update(
                    connection,
                    MessageStreamUpdateRequest {
                        message_id: latest.id,
                        delta: chunk.content_delta,
                    },
                )?;
            }
        }

        if latest.status != "complete" {
            latest = MessageRepository::mark_complete(
                connection,
                MessageCompleteRequest {
                    message_id: latest.id,
                    content: None,
                    token_count_input: None,
                    token_count_output: None,
                    metadata_json: None,
                },
            )?;
        }

        Ok(latest)
    }
}

impl ProviderResolutionService {
    pub fn resolve_for_pane(
        connection: &Connection,
        pane_id: &str,
    ) -> Result<ResolvedProvider, ProviderResolutionError> {
        let (provider, credential) = Self::resolve_provider_and_credential(connection, pane_id)?;

        resolve_provider_with_credential(&provider, credential)
    }

    pub fn resolve_for_pane_execution(
        connection: &Connection,
        pane_id: &str,
        credentials: &CredentialService,
    ) -> Result<ResolvedProvider, ProviderResolutionError> {
        let (provider, credential) = Self::resolve_provider_and_credential(connection, pane_id)?;

        if provider.provider_type != "openai" {
            return Err(ProviderResolutionError::unsupported_provider(
                provider.id,
                provider.provider_type,
            ));
        }

        if credential.auth_type != "api_key" {
            return Err(ProviderResolutionError::storage(format!(
                "OpenAI execution requires api_key auth, got {}",
                credential.auth_type
            )));
        }

        let api_key = credentials
            .read_api_key(&credential.credential_ref)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;

        resolve_openai_provider_with_api_key(&provider, &credential, api_key)
    }

    fn resolve_provider_and_credential(
        connection: &Connection,
        pane_id: &str,
    ) -> Result<(ProviderDto, CredentialHandle), ProviderResolutionError> {
        let pane = PaneRepository::get_open_by_id(connection, pane_id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;
        let provider_id = pane
            .provider_id
            .as_deref()
            .ok_or_else(ProviderResolutionError::provider_not_configured)?;

        let provider = ProviderRepository::get_enabled_by_id(connection, provider_id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;

        let credential =
            Self::resolve_credential(connection, provider_id, pane.account_id.as_deref())?;

        Ok((provider, credential))
    }

    fn resolve_credential(
        connection: &Connection,
        provider_id: &str,
        account_id: Option<&str>,
    ) -> Result<CredentialHandle, ProviderResolutionError> {
        let account = match account_id {
            Some(account_id) => {
                AccountRepository::get_by_id(connection, account_id).map_err(|error| {
                    if matches!(error, StorageError::NotFound(_)) {
                        ProviderResolutionError::no_account(
                            provider_id,
                            Some(account_id.to_string()),
                        )
                    } else {
                        ProviderResolutionError::storage(error.to_string())
                    }
                })?
            }
            None => AccountRepository::get_default_for_provider(connection, provider_id)
                .map_err(|error| ProviderResolutionError::storage(error.to_string()))?
                .ok_or_else(|| ProviderResolutionError::no_account(provider_id, None))?,
        };

        Self::credential_from_account(connection, provider_id, account)
    }

    fn credential_from_account(
        connection: &Connection,
        provider_id: &str,
        account: AccountDto,
    ) -> Result<CredentialHandle, ProviderResolutionError> {
        if account.provider_id != provider_id {
            return Err(ProviderResolutionError::no_account(
                provider_id,
                Some(account.id),
            ));
        }

        if account.status == "expired" {
            return Err(ProviderResolutionError::expired_account(
                provider_id,
                account.id,
            ));
        }

        if account.status != "active" {
            return Err(ProviderResolutionError::inactive_account(
                provider_id,
                account.id,
                account.status,
            ));
        }

        if account.auth_type == "oauth" {
            if let Some(expires_at) = account.token_expires_at.as_deref() {
                if is_expired(expires_at) {
                    return Err(ProviderResolutionError::expired_account(
                        provider_id,
                        account.id,
                    ));
                }
            }
        }

        let credential_ref = AccountRepository::credential_ref(connection, &account.id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;

        Ok(CredentialHandle::new(
            account.provider_id,
            account.id,
            account.auth_type,
            credential_ref,
            account.token_expires_at,
        ))
    }
}

fn is_expired(expires_at: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|expires_at| expires_at.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(false)
}

fn conversation_for_pane(
    connection: &Connection,
    pane_id: &str,
) -> Result<Conversation, StorageError> {
    let mut conversation = Conversation::new(pane_id, Model::OpenAIGpt);
    for message in MessageRepository::list_for_pane(connection, pane_id)? {
        if message.role == "assistant" && message.status == "pending" && message.content.is_empty()
        {
            continue;
        }
        if let Some(role) = message_role(&message.role) {
            conversation = conversation.with_message(Message::new(role, message.content));
        }
    }
    Ok(conversation)
}

fn message_role(role: &str) -> Option<MessageRole> {
    match role {
        "system" => Some(MessageRole::System),
        "user" => Some(MessageRole::User),
        "assistant" => Some(MessageRole::Assistant),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::{ChatExecutionService, ProviderResolutionService};
    use crate::auth::CredentialService;
    use crate::models::Model;
    use crate::providers::OpenAIProvider;
    use crate::storage::error::StorageResult;
    use crate::storage::models::CreatePaneRequest;
    use crate::storage::repositories::accounts::AccountRepository;
    use crate::storage::repositories::messages::MessageRepository;
    use crate::storage::repositories::panes::PaneRepository;
    use crate::storage::repositories::providers::ProviderRepository;

    #[test]
    fn resolves_openai_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "openai-account",
            "openai",
            "api_key",
            "active",
            true,
        )?;
        let pane = create_bound_pane(&conn, "openai", Some("openai-account"))?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("openai provider should resolve with account");

        assert_eq!(
            resolved.provider.list_models().unwrap(),
            vec![Model::OpenAIGpt]
        );
        assert_eq!(resolved.credential.account_id, "openai-account");
        assert_eq!(resolved.credential.auth_type, "api_key");
        assert_eq!(
            resolved.credential.credential_ref,
            "credential-ref-openai-account"
        );
        Ok(())
    }

    #[test]
    fn resolves_anthropic_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "anthropic-account",
            "anthropic",
            "api_key",
            "active",
            true,
        )?;
        let pane = create_bound_pane(&conn, "anthropic", Some("anthropic-account"))?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("anthropic provider should resolve with account");

        assert_eq!(
            resolved.provider.list_models().unwrap(),
            vec![Model::AnthropicClaude]
        );
        assert_eq!(resolved.credential.account_id, "anthropic-account");
        Ok(())
    }

    #[test]
    fn resolves_google_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "google-account",
            "google",
            "oauth",
            "active",
            true,
        )?;
        let pane = create_bound_pane(&conn, "google", Some("google-account"))?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("google provider should resolve with credential handle");

        assert_eq!(
            resolved.provider.list_models().unwrap(),
            vec![Model::GoogleGemini]
        );
        assert_eq!(resolved.credential.account_id, "google-account");
        assert_eq!(resolved.credential.auth_type, "oauth");
        assert!(resolved.credential.is_oauth());
        Ok(())
    }

    #[test]
    fn resolves_google_default_oauth_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "default-google",
            "google",
            "oauth",
            "active",
            true,
        )?;
        set_token_expires_at(&conn, "default-google", "2099-01-01T00:00:00Z")?;
        let pane = create_bound_pane(&conn, "google", None)?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("google default OAuth account should resolve");

        assert_eq!(
            resolved.provider.list_models().unwrap(),
            vec![Model::GoogleGemini]
        );
        assert_eq!(resolved.credential.account_id, "default-google");
        assert_eq!(resolved.credential.auth_type, "oauth");
        assert_eq!(
            resolved.credential.token_expires_at.as_deref(),
            Some("2099-01-01T00:00:00Z")
        );
        Ok(())
    }

    #[test]
    fn resolves_default_account_when_pane_has_no_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "default-openai",
            "openai",
            "api_key",
            "active",
            true,
        )?;
        let pane = create_bound_pane(&conn, "openai", None)?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("default account should resolve");

        assert_eq!(
            resolved.provider.list_models().unwrap(),
            vec![Model::OpenAIGpt]
        );
        assert_eq!(resolved.credential.account_id, "default-openai");
        Ok(())
    }

    #[test]
    fn execution_resolver_binds_openai_api_key_from_credential_service() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "openai-exec",
            "openai",
            "api_key",
            "active",
            true,
        )?;
        let credentials = CredentialService::in_memory();
        credentials.store_api_key(
            "credential-ref-openai-exec",
            "OpenAI Exec",
            "openai",
            "sk-exec-test",
        )?;
        let pane = create_bound_pane(&conn, "openai", Some("openai-exec"))?;

        let resolved =
            ProviderResolutionService::resolve_for_pane_execution(&conn, &pane.id, &credentials)
                .expect("OpenAI execution provider should resolve with API key");

        assert_eq!(resolved.credential.account_id, "openai-exec");
        assert_eq!(
            resolved.provider.list_models().unwrap(),
            vec![Model::OpenAIGpt]
        );
        Ok(())
    }

    #[test]
    fn execution_resolver_rejects_missing_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        let pane = create_bound_pane(&conn, "openai", None)?;
        let credentials = CredentialService::in_memory();

        let error = match ProviderResolutionService::resolve_for_pane_execution(
            &conn,
            &pane.id,
            &credentials,
        ) {
            Ok(_) => panic!("missing account should fail before provider execution"),
            Err(error) => error,
        };

        assert_eq!(error.code, "no_account");
        Ok(())
    }

    #[test]
    fn execution_resolver_rejects_non_openai_provider() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "anthropic-exec",
            "anthropic",
            "api_key",
            "active",
            true,
        )?;
        let credentials = CredentialService::in_memory();
        credentials.store_api_key(
            "credential-ref-anthropic-exec",
            "Anthropic Exec",
            "anthropic",
            "sk-ant-test",
        )?;
        let pane = create_bound_pane(&conn, "anthropic", Some("anthropic-exec"))?;

        let error = match ProviderResolutionService::resolve_for_pane_execution(
            &conn,
            &pane.id,
            &credentials,
        ) {
            Ok(_) => panic!("Phase 4A executes OpenAI only"),
            Err(error) => error,
        };

        assert_eq!(error.code, "unsupported_provider");
        Ok(())
    }

    #[test]
    fn openai_streaming_response_is_persisted_in_pane_conversation() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "openai-stream",
            "openai",
            "api_key",
            "active",
            true,
        )?;
        let pane = create_bound_pane(&conn, "openai", Some("openai-stream"))?;
        let base_url = spawn_streaming_server(concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" there\"},\"finish_reason\":null}]}\n\n",
            "data: [DONE]\n\n",
        ));
        let provider = OpenAIProvider::with_base_url_for_test("sk-test", base_url);

        let assistant = ChatExecutionService::stream_message_with_provider(
            &conn,
            &pane.id,
            "Hello".to_string(),
            &provider,
        )
        .expect("streaming response should persist");

        assert_eq!(assistant.content, "Hi there");
        assert_eq!(assistant.status, "complete");

        let messages = MessageRepository::list_for_pane(&conn, &pane.id)?;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "Hi there");
        assert_eq!(messages[1].status, "complete");
        Ok(())
    }

    #[test]
    fn inactive_account_is_rejected() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "revoked-openai",
            "openai",
            "api_key",
            "revoked",
            true,
        )?;
        let pane = create_bound_pane(&conn, "openai", Some("revoked-openai"))?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("inactive account should not resolve"),
            Err(error) => error,
        };

        assert_eq!(error.code, "inactive_account");
        assert_eq!(error.account_id.as_deref(), Some("revoked-openai"));
        Ok(())
    }

    #[test]
    fn expired_account_status_is_rejected() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "expired-google",
            "google",
            "oauth",
            "expired",
            true,
        )?;
        let pane = create_bound_pane(&conn, "google", Some("expired-google"))?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("expired account should not resolve"),
            Err(error) => error,
        };

        assert_eq!(error.code, "expired_account");
        assert_eq!(error.account_id.as_deref(), Some("expired-google"));
        Ok(())
    }

    #[test]
    fn expired_oauth_token_is_rejected() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(
            &conn,
            "past-google",
            "google",
            "oauth",
            "active",
            true,
        )?;
        set_token_expires_at(&conn, "past-google", "2000-01-01T00:00:00Z")?;
        let pane = create_bound_pane(&conn, "google", Some("past-google"))?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("expired OAuth token should not resolve"),
            Err(error) => error,
        };

        assert_eq!(error.code, "expired_account");
        assert_eq!(error.account_id.as_deref(), Some("past-google"));
        Ok(())
    }

    #[test]
    fn missing_account_is_rejected() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        let pane = create_pane_with_missing_account(&conn, "openai", "missing-openai")?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("missing account should not resolve"),
            Err(error) => error,
        };

        assert_eq!(error.code, "no_account");
        assert_eq!(error.account_id.as_deref(), Some("missing-openai"));
        Ok(())
    }

    #[test]
    fn missing_default_account_is_rejected() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        let pane = create_bound_pane(&conn, "openai", None)?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("provider with no active default account should not resolve"),
            Err(error) => error,
        };

        assert_eq!(error.code, "no_account");
        assert_eq!(error.provider_id.as_deref(), Some("openai"));
        assert!(error.account_id.is_none());
        Ok(())
    }

    #[test]
    fn unsupported_pane_provider_returns_structured_error() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        ProviderRepository::insert_test_provider(&conn, "openrouter", "openrouter")?;
        AccountRepository::insert_test_account(
            &conn,
            "openrouter-account",
            "openrouter",
            "api_key",
            "active",
            true,
        )?;
        let pane = create_bound_pane(&conn, "openrouter", Some("openrouter-account"))?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("openrouter is intentionally unsupported in Phase 3A"),
            Err(error) => error,
        };

        assert_eq!(error.code, "unsupported_provider");
        assert_eq!(error.provider_type.as_deref(), Some("openrouter"));
        Ok(())
    }

    fn create_bound_pane(
        conn: &rusqlite::Connection,
        provider_id: &str,
        account_id: Option<&str>,
    ) -> StorageResult<crate::storage::models::PaneDto> {
        let pane = PaneRepository::create(
            conn,
            CreatePaneRequest {
                workspace_id: None,
                title: Some("Provider Pane".to_string()),
                sort_order: None,
            },
        )?;
        conn.execute(
            "UPDATE panes SET provider_id = ?1, account_id = ?2 WHERE id = ?3",
            (provider_id, account_id, &pane.id),
        )?;
        PaneRepository::get_open_by_id(conn, &pane.id)
    }

    fn create_pane_with_missing_account(
        conn: &rusqlite::Connection,
        provider_id: &str,
        missing_account_id: &str,
    ) -> StorageResult<crate::storage::models::PaneDto> {
        let pane = PaneRepository::create(
            conn,
            CreatePaneRequest {
                workspace_id: None,
                title: Some("Provider Pane".to_string()),
                sort_order: None,
            },
        )?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
        conn.execute(
            "UPDATE panes SET provider_id = ?1, account_id = ?2 WHERE id = ?3",
            (provider_id, missing_account_id, &pane.id),
        )?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        PaneRepository::get_open_by_id(conn, &pane.id)
    }

    fn spawn_streaming_server(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind streaming server");
        let address = listener.local_addr().expect("streaming server address");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept streaming request");
            let mut buffer = [0_u8; 1024];
            loop {
                let read = stream.read(&mut buffer).expect("read request");
                if read == 0
                    || buffer[..read]
                        .windows(4)
                        .any(|window| window == b"\r\n\r\n")
                {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write streaming response");
        });
        format!("http://{address}")
    }

    fn set_token_expires_at(
        conn: &rusqlite::Connection,
        account_id: &str,
        token_expires_at: &str,
    ) -> StorageResult<()> {
        conn.execute(
            "UPDATE accounts SET token_expires_at = ?1 WHERE id = ?2",
            (token_expires_at, account_id),
        )?;
        Ok(())
    }
}
