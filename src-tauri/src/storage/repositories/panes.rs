use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::projects::repository::ProjectRepository;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::models::{CreatePaneRequest, PaneDto, PaneHeaderDisplayDto, SHELL_WORKSPACE_ID};
use crate::storage::repositories::workspaces::WorkspaceRepository;

const PANE_SELECT: &str = "SELECT id, workspace_id, title, role_label, sort_order, width_ratio, height_ratio,
        provider_id, account_id, model_id, status, project_id, layout_json, metadata_json,
        created_at, updated_at";

pub struct PaneRepository;

impl PaneRepository {
    pub fn list_shell_open(connection: &Connection) -> StorageResult<Vec<PaneDto>> {
        Self::list_open(connection, Some(SHELL_WORKSPACE_ID))
    }

    pub fn list_open(
        connection: &Connection,
        workspace_id: Option<&str>,
    ) -> StorageResult<Vec<PaneDto>> {
        let resolved_workspace_id = match workspace_id {
            Some(id) => id.to_string(),
            None => SHELL_WORKSPACE_ID.to_string(),
        };

        let sql = format!(
            "{PANE_SELECT}
             FROM panes
             WHERE workspace_id = ?1 AND closed_at IS NULL
             ORDER BY sort_order, created_at"
        );

        let mut statement = connection.prepare(&sql)?;

        let mut panes = statement
            .query_map([&resolved_workspace_id], map_pane_row)?
            .collect::<Result<Vec<_>, _>>()?;

        for pane in &mut panes {
            enrich_header_display(connection, pane)?;
        }

        Ok(panes)
    }

    pub fn create(connection: &Connection, request: CreatePaneRequest) -> StorageResult<PaneDto> {
        let workspace_id = request
            .workspace_id
            .as_deref()
            .unwrap_or(SHELL_WORKSPACE_ID)
            .to_string();
        if workspace_id != SHELL_WORKSPACE_ID {
            return Err(StorageError::InvalidInput(
                "panes must be created in the shell workspace".to_string(),
            ));
        }
        WorkspaceRepository::get_by_id(connection, &workspace_id)?;

        let project_id = match request.project_id {
            Some(project_id) => {
                ProjectRepository::get_by_id(connection, &project_id)?;
                project_id
            }
            None => ProjectRepository::resolve_focused_project_id(connection)?,
        };

        let next_sort_order = match request.sort_order {
            Some(sort_order) => sort_order,
            None => {
                let max_sort_order: Option<i32> = connection
                    .query_row(
                        "SELECT MAX(sort_order) FROM panes WHERE workspace_id = ?1 AND closed_at IS NULL",
                        [SHELL_WORKSPACE_ID],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                max_sort_order.unwrap_or(-1) + 1
            }
        };

        let now = Utc::now().to_rfc3339();
        let pane_id = Uuid::new_v4().to_string();
        let title = request.title.unwrap_or_else(|| "New Pane".to_string());

        connection.execute(
            "INSERT INTO panes (
                id, workspace_id, project_id, title, sort_order, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'idle', ?6, ?7)",
            (
                &pane_id,
                &workspace_id,
                &project_id,
                &title,
                next_sort_order,
                &now,
                &now,
            ),
        )?;

        Self::get_by_id(connection, &pane_id)
    }

    pub fn set_project(
        connection: &Connection,
        pane_id: &str,
        project_id: &str,
    ) -> StorageResult<PaneDto> {
        ProjectRepository::get_by_id(connection, project_id)?;
        let updated = connection.execute(
            "UPDATE panes
             SET project_id = ?1, updated_at = datetime('now')
             WHERE id = ?2 AND closed_at IS NULL",
            (project_id, pane_id),
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!(
                "open pane {pane_id} not found"
            )));
        }

        Self::get_by_id(connection, pane_id)
    }

    pub fn close(connection: &Connection, pane_id: &str) -> StorageResult<()> {
        let now = Utc::now().to_rfc3339();
        let updated = connection.execute(
            "UPDATE panes
             SET closed_at = ?1, updated_at = ?1, status = 'idle'
             WHERE id = ?2 AND closed_at IS NULL",
            (&now, pane_id),
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(format!(
                "open pane {pane_id} not found"
            )));
        }

        Ok(())
    }

    pub fn get_by_id(connection: &Connection, pane_id: &str) -> StorageResult<PaneDto> {
        let sql = format!("{PANE_SELECT} FROM panes WHERE id = ?1");
        let mut pane = connection
            .query_row(&sql, [pane_id], map_pane_row)
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("pane {pane_id} not found")))?;
        enrich_header_display(connection, &mut pane)?;
        Ok(pane)
    }

    pub fn get_open_by_id(connection: &Connection, pane_id: &str) -> StorageResult<PaneDto> {
        let mut pane = Self::get_open_for_execution(connection, pane_id)?;
        enrich_header_display(connection, &mut pane)?;
        Ok(pane)
    }

    /// Hot-path pane fetch for stream execution. Skips header display enrichment queries.
    pub fn get_open_for_execution(connection: &Connection, pane_id: &str) -> StorageResult<PaneDto> {
        let sql = format!("{PANE_SELECT} FROM panes WHERE id = ?1 AND closed_at IS NULL");
        let pane = connection
            .query_row(&sql, [pane_id], map_pane_row)
            .optional()?
            .ok_or_else(|| StorageError::NotFound(format!("open pane {pane_id} not found")))?;
        if pane.project_id.is_none() {
            return Err(StorageError::InvalidInput(format!(
                "open pane {pane_id} is missing project_id"
            )));
        }
        Ok(pane)
    }
}

fn map_pane_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PaneDto> {
    Ok(PaneDto {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        role_label: row.get(3)?,
        sort_order: row.get(4)?,
        width_ratio: row.get(5)?,
        height_ratio: row.get(6)?,
        provider_id: row.get(7)?,
        account_id: row.get(8)?,
        model_id: row.get(9)?,
        status: row.get(10)?,
        project_id: row.get(11)?,
        layout_json: row.get(12)?,
        metadata_json: row.get(13)?,
        header_display: PaneHeaderDisplayDto::default(),
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn enrich_header_display(connection: &Connection, pane: &mut PaneDto) -> StorageResult<()> {
    pane.header_display = PaneHeaderDisplayDto {
        provider_label: pane
            .provider_id
            .as_deref()
            .map(|provider_id| provider_display_label(connection, provider_id))
            .transpose()?,
        auth_label: pane.account_id.as_deref().and_then(|account_id| {
            account_auth_label(connection, account_id).ok()
        }),
        model_label: pane.model_id.as_deref().map(compact_model_label),
        reasoning_label: reasoning_display_label(pane.metadata_json.as_deref()),
    };
    Ok(())
}

fn provider_display_label(connection: &Connection, provider_id: &str) -> StorageResult<String> {
    connection
        .query_row(
            "SELECT display_name FROM providers WHERE id = ?1",
            [provider_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| StorageError::NotFound(format!("provider {provider_id} not found")))
}

fn account_auth_label(connection: &Connection, account_id: &str) -> StorageResult<String> {
    let auth_type: String = connection
        .query_row(
            "SELECT auth_type FROM accounts WHERE id = ?1",
            [account_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| StorageError::NotFound(format!("account {account_id} not found")))?;
    Ok(compact_auth_label(&auth_type))
}

fn compact_auth_label(auth_type: &str) -> String {
    match auth_type {
        "api_key" => "API Key".to_string(),
        "oauth" => "OAuth".to_string(),
        other => other.to_string(),
    }
}

fn compact_model_label(model_id: &str) -> String {
    match model_id {
        "OpenAIGpt" | "gpt-4o-mini" => "GPT-4o Mini".to_string(),
        "gpt-5.5" | "GPT-5.5" => "GPT-5.5".to_string(),
        "gpt-5.4-mini" | "GPT-5.4 mini" => "GPT-5.4 Mini".to_string(),
        "gpt-5.3-codex-spark" | "GPT-5.3 Codex Spark" => "GPT-5.3 Codex Spark".to_string(),
        other => other.to_string(),
    }
}

fn reasoning_display_label(metadata_json: Option<&str>) -> Option<String> {
    let metadata = metadata_json?;
    let value: serde_json::Value = serde_json::from_str(metadata).ok()?;
    let reasoning = value.get("reasoningLevel")?.as_str()?;
    Some(match reasoning {
        "low" => "Low".to_string(),
        "medium" => "Medium".to_string(),
        "high" => "High".to_string(),
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repositories::accounts::AccountRepository;

    fn setup_connection() -> Connection {
        let connection = crate::storage::test_fixtures::initialize_test_connection();
        crate::storage::test_fixtures::seed_test_project(&connection, "PaneTests")
            .expect("seed project");
        connection
    }

    #[test]
    fn pane_dto_includes_compact_header_display_metadata() -> StorageResult<()> {
        let connection = setup_connection();
        let account = AccountRepository::create_oauth_account(
            &connection,
            "openai",
            "sterlingdigitalp@gmail.com (OAuth) (default)",
            "cred-oauth",
            "chatgpt-account",
            Some("sterlingdigitalp@gmail.com"),
            &Utc::now().to_rfc3339(),
            Some("openid profile email offline_access"),
            true,
        )?;
        let project_id = ProjectRepository::resolve_focused_project_id(&connection)?;
        let pane = PaneRepository::create(
            &connection,
            CreatePaneRequest {
                workspace_id: None,
                project_id: Some(project_id),
                title: Some("Display".to_string()),
                sort_order: None,
            },
        )?;

        connection.execute(
            "UPDATE panes
             SET provider_id = 'openai', account_id = ?1, model_id = 'gpt-5.5', metadata_json = ?2
             WHERE id = ?3",
            (
                &account.id,
                serde_json::json!({ "reasoningLevel": "medium" }).to_string(),
                &pane.id,
            ),
        )?;

        let pane = PaneRepository::get_by_id(&connection, &pane.id)?;

        assert_eq!(
            pane.header_display.provider_label.as_deref(),
            Some("OpenAI")
        );
        assert_eq!(pane.header_display.auth_label.as_deref(), Some("OAuth"));
        assert_eq!(pane.header_display.model_label.as_deref(), Some("GPT-5.5"));
        assert_eq!(
            pane.header_display.reasoning_label.as_deref(),
            Some("Medium")
        );
        Ok(())
    }

    #[test]
    fn pane_dto_uses_api_key_auth_header_label() -> StorageResult<()> {
        let connection = setup_connection();
        let account = AccountRepository::create_api_key_account(
            &connection,
            "openai",
            "OpenAI Work",
            "cred-api",
            true,
        )?;
        let project_id = ProjectRepository::resolve_focused_project_id(&connection)?;
        let pane = PaneRepository::create(
            &connection,
            CreatePaneRequest {
                workspace_id: None,
                project_id: Some(project_id),
                title: Some("Display".to_string()),
                sort_order: None,
            },
        )?;

        connection.execute(
            "UPDATE panes
             SET provider_id = 'openai', account_id = ?1, model_id = 'gpt-5.4-mini'
             WHERE id = ?2",
            (&account.id, &pane.id),
        )?;

        let pane = PaneRepository::get_by_id(&connection, &pane.id)?;

        assert_eq!(
            pane.header_display.provider_label.as_deref(),
            Some("OpenAI")
        );
        assert_eq!(pane.header_display.auth_label.as_deref(), Some("API Key"));
        assert_eq!(
            pane.header_display.model_label.as_deref(),
            Some("GPT-5.4 Mini")
        );
        assert_eq!(pane.header_display.reasoning_label, None);
        Ok(())
    }
}