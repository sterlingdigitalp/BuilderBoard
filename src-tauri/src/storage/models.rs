use serde::{Deserialize, Serialize};

pub const DEFAULT_WORKSPACE_ID: &str = "00000000-0000-4000-8000-000000000001";
pub const SHELL_WORKSPACE_ID: &str = DEFAULT_WORKSPACE_ID;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub is_default: bool,
    pub layout_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDto {
    pub id: String,
    pub provider_type: String,
    pub display_name: String,
    pub enabled: bool,
    pub auth_mode: String,
    pub supports_chat: bool,
    pub supports_streaming: bool,
    pub supports_tool_use: bool,
    pub supports_vision: bool,
    pub context_window: Option<i64>,
    pub locality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneDto {
    pub id: String,
    pub workspace_id: String,
    pub title: Option<String>,
    pub role_label: Option<String>,
    pub sort_order: i32,
    pub width_ratio: Option<f64>,
    pub height_ratio: Option<f64>,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model_id: Option<String>,
    pub status: String,
    pub project_id: Option<String>,
    pub layout_json: Option<String>,
    pub metadata_json: Option<String>,
    pub header_display: PaneHeaderDisplayDto,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneHeaderDisplayDto {
    pub provider_label: Option<String>,
    pub auth_label: Option<String>,
    pub model_label: Option<String>,
    pub reasoning_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDto {
    pub id: String,
    pub workspace_id: String,
    pub pane_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub content: String,
    pub content_type: String,
    pub status: String,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model_id: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaneRequest {
    pub workspace_id: Option<String>,
    pub project_id: Option<String>,
    pub title: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub compact_label: String,
    pub auth_type: String,
    pub external_email: Option<String>,
    pub status: String,
    pub token_expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatusDto {
    pub account_id: String,
    pub provider_id: String,
    pub status: String,
    pub token_expires_at: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStartResult {
    pub auth_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCompleteEvent {
    pub account_id: String,
    pub provider_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthErrorEvent {
    pub provider_id: String,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendMessageRequest {
    pub pane_id: String,
    pub role: String,
    pub content: String,
    pub content_type: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageCreateRequest {
    pub pane_id: String,
    pub content: String,
    pub content_type: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageCreateResult {
    pub user_message: MessageDto,
    pub assistant_message: MessageDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageStreamUpdateRequest {
    pub message_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageCompleteRequest {
    pub message_id: String,
    pub content: Option<String>,
    pub token_count_input: Option<i64>,
    pub token_count_output: Option<i64>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageErrorRequest {
    pub message_id: String,
    pub error_code: String,
    pub error_message: String,
}
