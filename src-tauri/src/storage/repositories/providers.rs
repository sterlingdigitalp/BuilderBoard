use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::ProviderDto;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OAuthProviderConfig {
    pub authorization_url: String,
    pub token_url: String,
    pub revocation_url: String,
    pub scopes: Vec<String>,
    pub userinfo_url: String,
}

pub struct ProviderRepository;

impl ProviderRepository {
    pub fn list_enabled(connection: &Connection) -> StorageResult<Vec<ProviderDto>> {
        let mut statement = connection.prepare(
            "SELECT id, provider_type, display_name, enabled, auth_mode,
                    supports_chat, supports_streaming, supports_tool_use, supports_vision,
                    context_window, locality
             FROM providers
             WHERE enabled = 1
             ORDER BY display_name",
        )?;

        let providers = statement
            .query_map([], map_provider_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    pub fn count(connection: &Connection) -> StorageResult<i64> {
        let count: i64 = connection.query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn get_oauth_config(connection: &Connection, provider_id: &str) -> StorageResult<OAuthProviderConfig> {
        let raw: Option<String> = connection
            .query_row(
                "SELECT oauth_config_json FROM providers WHERE id = ?1 AND enabled = 1",
                [provider_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        let Some(raw) = raw else {
            return Err(StorageError::InvalidInput(format!(
                "provider {provider_id} has no oauth configuration"
            )));
        };

        serde_json::from_str(&raw).map_err(|err| {
            StorageError::InvalidInput(format!("invalid oauth_config_json for {provider_id}: {err}"))
        })
    }

    pub fn get_enabled_by_id(connection: &Connection, provider_id: &str) -> StorageResult<ProviderDto> {
        connection
            .query_row(
                "SELECT id, provider_type, display_name, enabled, auth_mode,
                        supports_chat, supports_streaming, supports_tool_use, supports_vision,
                        context_window, locality
                 FROM providers
                 WHERE id = ?1 AND enabled = 1",
                [provider_id],
                map_provider_row,
            )
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("enabled provider {provider_id} not found")))
    }

    #[cfg(test)]
    pub fn insert_test_provider(
        connection: &Connection,
        provider_id: &str,
        provider_type: &str,
    ) -> StorageResult<()> {
        connection.execute(
            "INSERT INTO providers (
                id, provider_type, display_name, enabled, auth_mode,
                supports_chat, supports_streaming, supports_tool_use, supports_vision,
                context_window, locality, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 1, 'api_key', 1, 1, 0, 0, NULL, 'remote',
                '2026-06-23T00:00:00Z', '2026-06-23T00:00:00Z')",
            (provider_id, provider_type, provider_id),
        )?;
        Ok(())
    }
}

fn map_provider_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderDto> {
    Ok(ProviderDto {
        id: row.get(0)?,
        provider_type: row.get(1)?,
        display_name: row.get(2)?,
        enabled: row.get::<_, i64>(3)? == 1,
        auth_mode: row.get(4)?,
        supports_chat: row.get::<_, i64>(5)? == 1,
        supports_streaming: row.get::<_, i64>(6)? == 1,
        supports_tool_use: row.get::<_, i64>(7)? == 1,
        supports_vision: row.get::<_, i64>(8)? == 1,
        context_window: row.get(9)?,
        locality: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_three_providers() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATION_0001_FOR_TEST)?;
        let providers = ProviderRepository::list_enabled(&conn)?;
        assert_eq!(providers.len(), 3);
        let ids: Vec<_> = providers.iter().map(|provider| provider.id.as_str()).collect();
        assert!(ids.contains(&"anthropic"));
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"google"));
        Ok(())
    }

    #[test]
    fn gets_enabled_provider_by_id() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;

        let provider = ProviderRepository::get_enabled_by_id(&conn, "openai")?;

        assert_eq!(provider.provider_type, "openai");
        Ok(())
    }

    #[test]
    fn google_oauth_config_is_seeded() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATIONS_FOR_TEST)?;

        let config = ProviderRepository::get_oauth_config(&conn, "google")?;
        assert_eq!(
            config.authorization_url,
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(config.scopes, vec!["openid".to_string(), "email".to_string()]);
        assert!(!config
            .scopes
            .iter()
            .any(|scope| scope.contains("generative-language")));
        Ok(())
    }
}
