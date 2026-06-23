use chrono::Utc;
use rusqlite::Connection;

use crate::auth::CredentialHandle;
use crate::providers::{resolve_provider_with_credential, ProviderResolutionError, ResolvedProvider};
use crate::storage::error::StorageError;
use crate::storage::models::AccountDto;
use crate::storage::repositories::accounts::AccountRepository;
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::providers::ProviderRepository;

pub struct ProviderResolutionService;

impl ProviderResolutionService {
    pub fn resolve_for_pane(
        connection: &Connection,
        pane_id: &str,
    ) -> Result<ResolvedProvider, ProviderResolutionError> {
        let pane = PaneRepository::get_open_by_id(connection, pane_id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;
        let provider_id = pane
            .provider_id
            .as_deref()
            .ok_or_else(ProviderResolutionError::provider_not_configured)?;

        let provider = ProviderRepository::get_enabled_by_id(connection, provider_id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;

        let credential = Self::resolve_credential(connection, provider_id, pane.account_id.as_deref())?;

        resolve_provider_with_credential(&provider, credential)
    }

    fn resolve_credential(
        connection: &Connection,
        provider_id: &str,
        account_id: Option<&str>,
    ) -> Result<CredentialHandle, ProviderResolutionError> {
        let account = match account_id {
            Some(account_id) => AccountRepository::get_by_id(connection, account_id).map_err(|error| {
                if matches!(error, StorageError::NotFound(_)) {
                    ProviderResolutionError::no_account(provider_id, Some(account_id.to_string()))
                } else {
                    ProviderResolutionError::storage(error.to_string())
                }
            })?,
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

#[cfg(test)]
mod tests {
    use super::ProviderResolutionService;
    use crate::models::Model;
    use crate::storage::error::StorageResult;
    use crate::storage::models::CreatePaneRequest;
    use crate::storage::repositories::accounts::AccountRepository;
    use crate::storage::repositories::panes::PaneRepository;
    use crate::storage::repositories::providers::ProviderRepository;

    #[test]
    fn resolves_openai_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(&conn, "openai-account", "openai", "api_key", "active", true)?;
        let pane = create_bound_pane(&conn, "openai", Some("openai-account"))?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("openai provider should resolve with account");

        assert_eq!(resolved.provider.list_models().unwrap(), vec![Model::OpenAIGpt]);
        assert_eq!(resolved.credential.account_id, "openai-account");
        assert_eq!(resolved.credential.auth_type, "api_key");
        assert_eq!(resolved.credential.credential_ref, "credential-ref-openai-account");
        Ok(())
    }

    #[test]
    fn resolves_anthropic_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(&conn, "anthropic-account", "anthropic", "api_key", "active", true)?;
        let pane = create_bound_pane(&conn, "anthropic", Some("anthropic-account"))?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("anthropic provider should resolve with account");

        assert_eq!(resolved.provider.list_models().unwrap(), vec![Model::AnthropicClaude]);
        assert_eq!(resolved.credential.account_id, "anthropic-account");
        Ok(())
    }

    #[test]
    fn resolves_google_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(&conn, "google-account", "google", "oauth", "active", true)?;
        let pane = create_bound_pane(&conn, "google", Some("google-account"))?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("google provider should resolve with credential handle");

        assert_eq!(resolved.provider.list_models().unwrap(), vec![Model::GoogleGemini]);
        assert_eq!(resolved.credential.account_id, "google-account");
        assert_eq!(resolved.credential.auth_type, "oauth");
        assert!(resolved.credential.is_oauth());
        Ok(())
    }

    #[test]
    fn resolves_google_default_oauth_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(&conn, "default-google", "google", "oauth", "active", true)?;
        set_token_expires_at(&conn, "default-google", "2099-01-01T00:00:00Z")?;
        let pane = create_bound_pane(&conn, "google", None)?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("google default OAuth account should resolve");

        assert_eq!(resolved.provider.list_models().unwrap(), vec![Model::GoogleGemini]);
        assert_eq!(resolved.credential.account_id, "default-google");
        assert_eq!(resolved.credential.auth_type, "oauth");
        assert_eq!(resolved.credential.token_expires_at.as_deref(), Some("2099-01-01T00:00:00Z"));
        Ok(())
    }

    #[test]
    fn resolves_default_account_when_pane_has_no_account() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(&conn, "default-openai", "openai", "api_key", "active", true)?;
        let pane = create_bound_pane(&conn, "openai", None)?;

        let resolved = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("default account should resolve");

        assert_eq!(resolved.provider.list_models().unwrap(), vec![Model::OpenAIGpt]);
        assert_eq!(resolved.credential.account_id, "default-openai");
        Ok(())
    }

    #[test]
    fn inactive_account_is_rejected() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;
        AccountRepository::insert_test_account(&conn, "revoked-openai", "openai", "api_key", "revoked", true)?;
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
        AccountRepository::insert_test_account(&conn, "expired-google", "google", "oauth", "expired", true)?;
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
        AccountRepository::insert_test_account(&conn, "past-google", "google", "oauth", "active", true)?;
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
