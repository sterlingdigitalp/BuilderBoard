use rusqlite::Connection;

use crate::providers::{resolve_provider_for_registry_entry, LLMProvider, ProviderResolutionError};
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::providers::ProviderRepository;

pub struct ProviderResolutionService;

impl ProviderResolutionService {
    pub fn resolve_for_pane(
        connection: &Connection,
        pane_id: &str,
    ) -> Result<Box<dyn LLMProvider>, ProviderResolutionError> {
        let pane = PaneRepository::get_open_by_id(connection, pane_id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;
        let provider_id = pane
            .provider_id
            .as_deref()
            .ok_or_else(ProviderResolutionError::provider_not_configured)?;

        let provider = ProviderRepository::get_enabled_by_id(connection, provider_id)
            .map_err(|error| ProviderResolutionError::storage(error.to_string()))?;

        resolve_provider_for_registry_entry(&provider)
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderResolutionService;
    use crate::models::Model;
    use crate::storage::error::StorageResult;
    use crate::storage::models::CreatePaneRequest;
    use crate::storage::repositories::panes::PaneRepository;
    use crate::storage::repositories::providers::ProviderRepository;

    #[test]
    fn resolves_provider_for_pane_from_database_registry() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATION_0001_FOR_TEST)?;
        let pane = PaneRepository::create(
            &conn,
            CreatePaneRequest {
                workspace_id: None,
                title: Some("Provider Pane".to_string()),
                sort_order: None,
            },
        )?;
        conn.execute(
            "UPDATE panes SET provider_id = 'anthropic' WHERE id = ?1",
            [&pane.id],
        )?;

        let provider = ProviderResolutionService::resolve_for_pane(&conn, &pane.id)
            .expect("anthropic provider should resolve");

        assert_eq!(provider.list_models().unwrap(), vec![Model::AnthropicClaude]);
        Ok(())
    }

    #[test]
    fn unsupported_pane_provider_returns_structured_error() -> StorageResult<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(crate::storage::migrations::MIGRATION_0001_FOR_TEST)?;
        ProviderRepository::insert_test_provider(&conn, "openrouter", "openrouter")?;
        let pane = PaneRepository::create(
            &conn,
            CreatePaneRequest {
                workspace_id: None,
                title: Some("Unsupported Provider Pane".to_string()),
                sort_order: None,
            },
        )?;
        conn.execute(
            "UPDATE panes SET provider_id = 'openrouter' WHERE id = ?1",
            [&pane.id],
        )?;

        let error = match ProviderResolutionService::resolve_for_pane(&conn, &pane.id) {
            Ok(_) => panic!("openrouter is intentionally unsupported in Phase 2C"),
            Err(error) => error,
        };

        assert_eq!(error.code, "unsupported_provider");
        assert_eq!(error.provider_type.as_deref(), Some("openrouter"));
        Ok(())
    }
}
