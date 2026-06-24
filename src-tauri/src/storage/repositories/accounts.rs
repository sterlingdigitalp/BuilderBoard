use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{AccountDto, AccountStatusDto};
use crate::storage::repositories::providers::ProviderRepository;

pub const API_KEY_SUPPORTED_PROVIDERS: &[&str] = &["openai", "anthropic", "google"];
pub const OAUTH_SUPPORTED_PROVIDERS: &[&str] = &["google", "openai"];

pub struct AccountRepository;

impl AccountRepository {
    pub fn validate_api_key_provider(
        connection: &Connection,
        provider_id: &str,
    ) -> StorageResult<()> {
        if !API_KEY_SUPPORTED_PROVIDERS.contains(&provider_id) {
            return Err(StorageError::InvalidInput(format!(
                "provider {provider_id} does not support API-key accounts in Phase 3A"
            )));
        }

        let providers = ProviderRepository::list_enabled(connection)?;
        if !providers.iter().any(|provider| provider.id == provider_id) {
            return Err(StorageError::NotFound(format!(
                "provider {provider_id} not found"
            )));
        }

        Ok(())
    }

    pub fn validate_oauth_provider(
        connection: &Connection,
        provider_id: &str,
    ) -> StorageResult<()> {
        if !OAUTH_SUPPORTED_PROVIDERS.contains(&provider_id) {
            return Err(StorageError::InvalidInput(format!(
                "provider {provider_id} does not support OAuth in Phase 3B"
            )));
        }

        let provider = ProviderRepository::get_enabled_by_id(connection, provider_id)?;
        if provider.auth_mode != "oauth" && provider_id != "openai" {
            return Err(StorageError::InvalidInput(format!(
                "provider {provider_id} is not configured for oauth"
            )));
        }

        ProviderRepository::get_oauth_config(connection, provider_id)?;

        Ok(())
    }

    pub fn create_oauth_account(
        connection: &Connection,
        provider_id: &str,
        label: &str,
        credential_ref: &str,
        external_account_id: &str,
        external_email: Option<&str>,
        token_expires_at: &str,
        scopes: Option<&str>,
        set_as_default: bool,
    ) -> StorageResult<AccountDto> {
        Self::validate_oauth_provider(connection, provider_id)?;

        if label.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "account label must not be empty".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let account_id = Uuid::new_v4().to_string();
        let should_default =
            set_as_default || !Self::provider_has_active_default(connection, provider_id)?;
        let scopes_json = scopes.map(|value| {
            serde_json::to_string(
                &value
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string())
        });

        connection.execute(
            "INSERT INTO accounts (
                id, provider_id, label, auth_type, credential_ref, external_account_id,
                external_email, token_expires_at, scopes_json, status,
                is_default, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'oauth', ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?10, ?11)",
            (
                &account_id,
                provider_id,
                label,
                credential_ref,
                external_account_id,
                external_email,
                token_expires_at,
                scopes_json,
                i64::from(should_default),
                &now,
                &now,
            ),
        )?;

        if should_default {
            Self::set_default(connection, &account_id)?;
        }

        Self::get_by_id(connection, &account_id)
    }

    pub fn update_oauth_token_metadata(
        connection: &Connection,
        account_id: &str,
        token_expires_at: &str,
        scopes: Option<&str>,
    ) -> StorageResult<()> {
        let now = Utc::now().to_rfc3339();
        let scopes_json = scopes.map(|value| {
            serde_json::to_string(
                &value
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string())
        });

        connection.execute(
            "UPDATE accounts
             SET token_expires_at = ?1, scopes_json = COALESCE(?2, scopes_json), updated_at = ?3, status = 'active'
             WHERE id = ?4",
            (token_expires_at, scopes_json, &now, account_id),
        )?;
        Ok(())
    }

    pub fn mark_expired(connection: &Connection, account_id: &str) -> StorageResult<()> {
        let now = Utc::now().to_rfc3339();
        connection.execute(
            "UPDATE accounts SET status = 'expired', updated_at = ?1 WHERE id = ?2",
            (&now, account_id),
        )?;
        Ok(())
    }

    pub fn create_api_key_account(
        connection: &Connection,
        provider_id: &str,
        label: &str,
        credential_ref: &str,
        set_as_default: bool,
    ) -> StorageResult<AccountDto> {
        Self::validate_api_key_provider(connection, provider_id)?;

        if label.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "account label must not be empty".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let account_id = Uuid::new_v4().to_string();
        let should_default =
            set_as_default || !Self::provider_has_active_default(connection, provider_id)?;

        connection.execute(
            "INSERT INTO accounts (
                id, provider_id, label, auth_type, credential_ref, status,
                is_default, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'api_key', ?4, 'active', ?5, ?6, ?7)",
            (
                &account_id,
                provider_id,
                label,
                credential_ref,
                i64::from(should_default),
                &now,
                &now,
            ),
        )?;

        if should_default {
            Self::set_default(connection, &account_id)?;
        }

        Self::get_by_id(connection, &account_id)
    }

    pub fn list_active(
        connection: &Connection,
        provider_id: Option<&str>,
    ) -> StorageResult<Vec<AccountDto>> {
        let mut accounts = match provider_id {
            Some(provider_id) => {
                let mut statement = connection.prepare(
                    "SELECT id, provider_id, label, auth_type, external_email, status,
                            token_expires_at, last_used_at, is_default, created_at, updated_at
                     FROM accounts
                     WHERE provider_id = ?1 AND status = 'active'
                     ORDER BY is_default DESC, created_at",
                )?;
                let rows = statement.query_map([provider_id], map_account_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut statement = connection.prepare(
                    "SELECT id, provider_id, label, auth_type, external_email, status,
                            token_expires_at, last_used_at, is_default, created_at, updated_at
                     FROM accounts
                     WHERE status = 'active'
                     ORDER BY provider_id, is_default DESC, created_at",
                )?;
                let rows = statement.query_map([], map_account_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };

        accounts.sort_by(|left, right| {
            right
                .is_default
                .cmp(&left.is_default)
                .then_with(|| left.provider_id.cmp(&right.provider_id))
                .then_with(|| left.created_at.cmp(&right.created_at))
        });

        Ok(accounts)
    }

    pub fn get_by_id(connection: &Connection, account_id: &str) -> StorageResult<AccountDto> {
        connection
            .query_row(
                "SELECT id, provider_id, label, auth_type, external_email, status,
                        token_expires_at, last_used_at, is_default, created_at, updated_at
                 FROM accounts
                 WHERE id = ?1",
                [account_id],
                map_account_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("account {account_id} not found")))
    }

    pub fn get_default_for_provider(
        connection: &Connection,
        provider_id: &str,
    ) -> StorageResult<Option<AccountDto>> {
        connection
            .query_row(
                "SELECT id, provider_id, label, auth_type, external_email, status,
                        token_expires_at, last_used_at, is_default, created_at, updated_at
                 FROM accounts
                 WHERE provider_id = ?1 AND status = 'active' AND is_default = 1
                 LIMIT 1",
                [provider_id],
                map_account_row,
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn get_status(
        connection: &Connection,
        account_id: &str,
    ) -> StorageResult<AccountStatusDto> {
        let account = Self::get_by_id(connection, account_id)?;
        Ok(AccountStatusDto {
            account_id: account.id,
            provider_id: account.provider_id,
            status: account.status,
            token_expires_at: account.token_expires_at,
            is_default: account.is_default,
        })
    }

    pub fn set_default(connection: &Connection, account_id: &str) -> StorageResult<AccountDto> {
        let account = Self::get_by_id(connection, account_id)?;
        if account.status != "active" {
            return Err(StorageError::InvalidInput(
                "only active accounts can be set as default".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        connection.execute(
            "UPDATE accounts
             SET is_default = 0, updated_at = ?1
             WHERE provider_id = ?2 AND status = 'active'",
            (&now, &account.provider_id),
        )?;
        connection.execute(
            "UPDATE accounts
             SET is_default = 1, updated_at = ?1
             WHERE id = ?2",
            (&now, account_id),
        )?;

        Self::get_by_id(connection, account_id)
    }

    pub fn credential_ref(connection: &Connection, account_id: &str) -> StorageResult<String> {
        connection
            .query_row(
                "SELECT credential_ref FROM accounts WHERE id = ?1",
                [account_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("account {account_id} not found")))
    }

    pub fn external_account_id(
        connection: &Connection,
        account_id: &str,
    ) -> StorageResult<Option<String>> {
        connection
            .query_row(
                "SELECT external_account_id FROM accounts WHERE id = ?1",
                [account_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("account {account_id} not found")))
    }

    pub fn revoke(connection: &Connection, account_id: &str) -> StorageResult<String> {
        let credential_ref = Self::credential_ref(connection, account_id)?;
        let account = Self::get_by_id(connection, account_id)?;
        if account.status == "revoked" {
            return Err(StorageError::InvalidInput(
                "account is already disconnected".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        connection.execute(
            "UPDATE accounts
             SET status = 'revoked', is_default = 0, updated_at = ?1
             WHERE id = ?2",
            (&now, account_id),
        )?;
        connection.execute(
            "UPDATE panes SET account_id = NULL, updated_at = ?1 WHERE account_id = ?2",
            (&now, account_id),
        )?;

        if account.is_default {
            if let Some(replacement) = Self::first_active_account_for_provider(
                connection,
                &account.provider_id,
                Some(account_id),
            )? {
                Self::set_default(connection, &replacement.id)?;
            }
        }

        Ok(credential_ref)
    }

    pub fn provider_has_active_default(
        connection: &Connection,
        provider_id: &str,
    ) -> StorageResult<bool> {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM accounts
             WHERE provider_id = ?1 AND status = 'active' AND is_default = 1",
            [provider_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn first_active_account_for_provider(
        connection: &Connection,
        provider_id: &str,
        exclude_account_id: Option<&str>,
    ) -> StorageResult<Option<AccountDto>> {
        let account = match exclude_account_id {
            Some(exclude_account_id) => connection
                .query_row(
                    "SELECT id, provider_id, label, auth_type, external_email, status,
                            token_expires_at, last_used_at, is_default, created_at, updated_at
                     FROM accounts
                     WHERE provider_id = ?1 AND status = 'active' AND id != ?2
                     ORDER BY created_at
                     LIMIT 1",
                    (provider_id, exclude_account_id),
                    map_account_row,
                )
                .optional()?,
            None => connection
                .query_row(
                    "SELECT id, provider_id, label, auth_type, external_email, status,
                            token_expires_at, last_used_at, is_default, created_at, updated_at
                     FROM accounts
                     WHERE provider_id = ?1 AND status = 'active'
                     ORDER BY created_at
                     LIMIT 1",
                    [provider_id],
                    map_account_row,
                )
                .optional()?,
        };

        Ok(account)
    }

    #[cfg(test)]
    pub fn insert_test_account(
        connection: &Connection,
        account_id: &str,
        provider_id: &str,
        auth_type: &str,
        status: &str,
        is_default: bool,
    ) -> StorageResult<()> {
        connection.execute(
            "INSERT INTO accounts (
                id, provider_id, label, auth_type, credential_ref, status,
                is_default, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '2026-06-23T00:00:00Z', '2026-06-23T00:00:00Z')",
            (
                account_id,
                provider_id,
                account_id,
                auth_type,
                format!("credential-ref-{account_id}"),
                status,
                i64::from(is_default),
            ),
        )?;
        Ok(())
    }
}

fn map_account_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountDto> {
    let provider_id: String = row.get(1)?;
    let auth_type: String = row.get(3)?;
    Ok(AccountDto {
        id: row.get(0)?,
        provider_id: provider_id.clone(),
        label: row.get(2)?,
        compact_label: compact_account_label(&provider_id, &auth_type),
        auth_type,
        external_email: row.get(4)?,
        status: row.get(5)?,
        token_expires_at: row.get(6)?,
        last_used_at: row.get(7)?,
        is_default: row.get::<_, i64>(8)? == 1,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn compact_account_label(provider_id: &str, auth_type: &str) -> String {
    match (provider_id, auth_type) {
        (_, "api_key") => "API Key".to_string(),
        ("openai", "oauth") => "ChatGPT".to_string(),
        (_, "oauth") => "OAuth".to_string(),
        (_, other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)
            .expect("test schema");
        connection
    }

    #[test]
    fn first_account_becomes_default() -> StorageResult<()> {
        let connection = setup_connection();
        let account = AccountRepository::create_api_key_account(
            &connection,
            "openai",
            "Work",
            "cred-1",
            false,
        )?;
        assert!(account.is_default);
        Ok(())
    }

    #[test]
    fn set_default_switches_provider_default() -> StorageResult<()> {
        let connection = setup_connection();
        let first = AccountRepository::create_api_key_account(
            &connection,
            "openai",
            "First",
            "cred-1",
            true,
        )?;
        let second = AccountRepository::create_api_key_account(
            &connection,
            "openai",
            "Second",
            "cred-2",
            false,
        )?;
        assert!(first.is_default);
        assert!(!second.is_default);

        let updated = AccountRepository::set_default(&connection, &second.id)?;
        let first = AccountRepository::get_by_id(&connection, &first.id)?;
        assert!(!first.is_default);
        assert!(updated.is_default);
        Ok(())
    }

    #[test]
    fn default_account_selects_active_default_for_provider() -> StorageResult<()> {
        let conn = setup_connection();
        AccountRepository::insert_test_account(
            &conn, "openai-1", "openai", "api_key", "active", true,
        )?;

        let account = AccountRepository::get_default_for_provider(&conn, "openai")?
            .expect("default account should exist");

        assert_eq!(account.id, "openai-1");
        assert!(account.is_default);
        Ok(())
    }

    #[test]
    fn default_account_ignores_inactive_accounts() -> StorageResult<()> {
        let conn = setup_connection();
        AccountRepository::insert_test_account(
            &conn,
            "openai-expired",
            "openai",
            "api_key",
            "expired",
            true,
        )?;

        let account = AccountRepository::get_default_for_provider(&conn, "openai")?;

        assert!(account.is_none());
        Ok(())
    }

    #[test]
    fn account_dto_includes_compact_display_label() -> StorageResult<()> {
        let conn = setup_connection();
        let api_key = AccountRepository::create_api_key_account(
            &conn,
            "openai",
            "sterlingdigitalp@gmail.com (API Key) (default)",
            "cred-api",
            true,
        )?;
        let oauth = AccountRepository::create_oauth_account(
            &conn,
            "openai",
            "sterlingdigitalp@gmail.com (OAuth) (default)",
            "cred-oauth",
            "chatgpt-account",
            Some("sterlingdigitalp@gmail.com"),
            &Utc::now().to_rfc3339(),
            Some("openid profile email offline_access"),
            false,
        )?;

        assert_eq!(
            api_key.label,
            "sterlingdigitalp@gmail.com (API Key) (default)"
        );
        assert_eq!(api_key.compact_label, "API Key");
        assert_eq!(oauth.label, "sterlingdigitalp@gmail.com (OAuth) (default)");
        assert_eq!(
            oauth.external_email.as_deref(),
            Some("sterlingdigitalp@gmail.com")
        );
        assert_eq!(oauth.compact_label, "ChatGPT");
        Ok(())
    }
}
