use std::sync::Arc;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::auth::{CredentialService, OAuthService};
use crate::builders::global_builder_registry;
use crate::chat::{PaneExecutionContext, ProviderResolutionService};
use crate::execution::{global_engine_registry, ExecutionClass};
use crate::filesystem_intent::{route_filesystem_tools, FilesystemToolCall};
use crate::filesystem_tools::perf::{trace_perf_metric, PerfSpan};
use crate::filesystem_tools::scan_context::ScanContext;
use crate::filesystem_tools::service::{
    FilesystemService, MAX_INJECTED_FIND_PATHS, MAX_INJECTED_READ_FILE_CHARS,
    MAX_INJECTED_SEARCH_FILES, MAX_PROMPT_INJECTION_BYTES,
};
use crate::models::{Conversation, Message, MessageRole, Model};
use crate::project_scope_cache::ProjectScopeCache;
use crate::projects::repository::ProjectRepository;
use crate::providers::StreamChunk;
use crate::runtime_diagnostics::{emit_main_thread_block_total, trace_runtime_phase, RuntimeSpan};
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::{
    AccountDto, AccountStatusDto, AppendMessageRequest, CreatePaneRequest, MessageCompleteRequest,
    MessageCreateRequest, MessageCreateResult, MessageDto, MessageErrorRequest,
    MessageStreamUpdateRequest, PaneDto, ProviderDto, WorkspaceDto,
};
use crate::storage::repositories::accounts::AccountRepository;
use crate::storage::repositories::messages::MessageRepository;
use crate::storage::repositories::panes::PaneRepository;
use crate::storage::repositories::providers::ProviderRepository;
use crate::storage::repositories::workspaces::WorkspaceRepository;
use crate::stream_execution::{run_background_stream_chat, StreamChatJob};
use crate::stream_persistence::StreamPersistenceService;

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
pub fn engine_list() -> Vec<serde_json::Value> {
    let reg = global_engine_registry();
    reg.list_ids()
        .into_iter()
        .filter_map(|id| {
            reg.get(&id).map(|engine| {
                let caps = engine.capabilities();
                serde_json::json!({
                    "id": id,
                    "displayName": engine.display_name(),
                    "models": engine.list_models(),
                    "supportedEfforts": engine.supported_effort_levels(),
                    "health": engine.health(),
                    "capabilities": {
                        "locality": format!("{:?}", caps.locality),
                        "features": {
                            "chat": caps.features.chat,
                            "streaming": caps.features.streaming,
                            "reasoning": caps.features.reasoning,
                            "toolUse": caps.features.tool_use,
                            "images": caps.features.images,
                            "embeddings": caps.features.embeddings,
                            "structuredOutput": caps.features.structured_output,
                            "multimodal": caps.features.multimodal,
                            "filesystem": caps.features.filesystem,
                            "shell": caps.features.shell,
                            "subagents": caps.features.subagents,
                            "worktrees": caps.features.worktrees,
                            "cancellation": caps.features.cancellation,
                        }
                    }
                })
            })
        })
        .collect()
}

#[tauri::command]
pub fn builder_list() -> Vec<serde_json::Value> {
    let reg = global_builder_registry();
    reg.list()
        .into_iter()
        .map(|b| {
            serde_json::json!({
                "name": b.name,
                "displayName": b.display_name,
                "execution": {
                    "preferredClass": b.execution.preferred_class,
                    "preferredEngine": b.execution.preferred_engine,
                    "fallbackEngines": b.execution.fallback_engines,
                    "effort": b.execution.effort,
                    "defaultModel": b.execution.default_model,
                    "reviewRequirements": b.execution.review_requirements,
                    "memoryDefaults": b.execution.memory_defaults,
                }
            })
        })
        .collect()
}

#[tauri::command]
pub fn capability_list(
    allow_shell: Option<bool>,
    allow_read: Option<bool>,
    allow_write: Option<bool>,
    allow_delete: Option<bool>,
    allow_git: Option<bool>,
    allow_packages: Option<bool>,
    allow_processes: Option<bool>,
    allow_network: Option<bool>,
    include_unavailable: Option<bool>,
) -> Vec<serde_json::Value> {
    let policy = crate::execution::context::ExecutionPolicy {
        allow_shell: allow_shell.unwrap_or(true),
        allow_network: allow_network.unwrap_or(true),
        allow_read: allow_read.unwrap_or(true),
        allow_write: allow_write.unwrap_or(true),
        allow_delete: allow_delete.unwrap_or(true),
        allow_git: allow_git.unwrap_or(true),
        allow_packages: allow_packages.unwrap_or(true),
        allow_processes: allow_processes.unwrap_or(true),
        max_tokens: None,
        timeout_ms: None,
    };
    let show_all = include_unavailable.unwrap_or(false);

    let registry_arc = crate::execution::tools::registry::global_tool_registry();
    let reg = match registry_arc.read() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let all_tools: Vec<_> = if show_all {
        reg.list()
    } else {
        crate::execution::capability_resolver::resolve_allowed_tools(&policy, &reg)
    };

    all_tools.iter().map(|tool| {
        let desc = tool.describe();
        let schema = crate::execution::capability_resolver::tool_input_schema(tool.id().as_str());
        let is_allowed = tool.permissions().iter().all(|p| {
            crate::execution::capability_resolver::tool_permission_allowed(p, &policy)
        });
        let blocked_reason = if !is_allowed {
            let blocked: Vec<String> = tool.permissions().iter()
                .filter(|p| !crate::execution::capability_resolver::tool_permission_allowed(p, &policy))
                .map(|p| p.as_str().to_string())
                .collect();
            Some(blocked.join(", "))
        } else {
            None
        };

        serde_json::json!({
            "id": desc.id,
            "displayName": desc.display_name,
            "description": desc.description,
            "category": desc.category,
            "permissions": desc.permissions,
            "executionClasses": desc.supported_execution_classes,
            "inputSchema": schema,
            "available": is_allowed,
            "blockedReason": blocked_reason,
            "examples": crate::execution::capability_resolver::tool_usage_examples(tool.id().as_str()),
        })
    }).collect()
}

#[tauri::command]
pub fn resolve_execution(builder: Option<String>, class: Option<String>) -> serde_json::Value {
    let _mgr = crate::execution::manager::ExecutionManager::new();
    let ctx = crate::execution::ExecutionContext::local("resolve".to_string());
    let req = crate::execution::ExecutionRequest::chat(
        crate::models::Conversation::new(
            "resolve",
            crate::models::Model::Custom("resolve".to_string()),
        ),
        None,
    );
    let cls = class
        .map(|c| ExecutionClass::from_str(&c))
        .unwrap_or(ExecutionClass::Implementation);
    let res = crate::execution::manager::ExecutionManager::resolve(
        builder.as_deref(),
        Some(cls),
        &ctx,
        &req,
    );
    serde_json::json!({
        "engineId": res.engine_id,
        "model": res.model,
        "effort": res.effort,
        "reason": res.reason,
        "class": res.class.as_str(),
    })
}

#[tauri::command]
pub fn workspace_create(
    database: State<'_, Arc<Database>>,
    name: String,
) -> Result<WorkspaceDto, String> {
    database
        .with_connection(|connection| WorkspaceRepository::create(connection, &name))
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn workspace_list(database: State<'_, Arc<Database>>) -> Result<Vec<WorkspaceDto>, String> {
    database
        .with_connection(WorkspaceRepository::list_active)
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn workspace_switch(
    database: State<'_, Arc<Database>>,
    workspace_id: String,
) -> Result<WorkspaceDto, String> {
    database
        .with_connection(|connection| WorkspaceRepository::switch_active(connection, &workspace_id))
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn workspace_get_active(database: State<'_, Arc<Database>>) -> Result<WorkspaceDto, String> {
    database
        .with_connection(WorkspaceRepository::get_active)
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_list(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
) -> Result<Vec<PaneDto>, String> {
    database
        .with_connection_labeled("pane_list", |connection| {
            let _ = workspace_id;
            PaneRepository::list_shell_open(connection)
        })
        .map_err(format_storage_error)
}

#[tauri::command]
pub fn pane_create(
    database: State<'_, Arc<Database>>,
    workspace_id: Option<String>,
    project_id: Option<String>,
    title: Option<String>,
    sort_order: Option<i32>,
) -> Result<PaneDto, String> {
    database
        .with_connection(|connection| {
            PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id,
                    project_id,
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
pub fn pane_set_project(
    database: State<'_, Arc<Database>>,
    scope_cache: State<'_, Arc<crate::project_scope_cache::ProjectScopeCache>>,
    pane_id: String,
    project_id: String,
) -> Result<PaneDto, String> {
    let previous_project_id = database
        .with_connection(|connection| {
            Ok(PaneRepository::get_by_id(connection, &pane_id)
                .ok()
                .and_then(|pane| pane.project_id))
        })
        .map_err(format_storage_error)?;

    let pane = database
        .with_connection(|connection| {
            PaneRepository::set_project(connection, &pane_id, &project_id)
        })
        .map_err(format_storage_error)?;

    if let Some(previous_project_id) = previous_project_id {
        scope_cache.invalidate_project(&previous_project_id);
    }
    scope_cache.invalidate_project(&project_id);
    Ok(pane)
}

#[tauri::command]
pub fn message_list(
    database: State<'_, Arc<Database>>,
    pane_id: String,
) -> Result<Vec<MessageDto>, String> {
    database
        .with_connection_labeled("message_list", |connection| {
            MessageRepository::list_for_pane(connection, &pane_id)
        })
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
pub async fn stream_chat(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    credentials: State<'_, Arc<CredentialService>>,
    oauth: State<'_, Arc<OAuthService>>,
    stream_persistence: State<'_, Arc<StreamPersistenceService>>,
    scope_cache: State<'_, Arc<ProjectScopeCache>>,
    pane_id: String,
    provider_id: String,
    builder_id: Option<String>,
    account_id: String,
    model_id: String,
    assistant_message_id: String,
    reasoning_level: Option<String>,
) -> Result<(), String> {
    let route_is_engine = global_engine_registry().get(&provider_id).is_some();
    let route_is_builder = builder_id
        .as_deref()
        .and_then(|builder| global_builder_registry().get(builder))
        .is_some();

    if !route_is_engine && !route_is_builder {
        let message = "Selected execution route is not available.".to_string();
        emit_stream_error(
            &app,
            &pane_id,
            &assistant_message_id,
            "unsupported_provider",
            &message,
        );
        return Err(message);
    }

    let command_span = RuntimeSpan::start("TAURI_COMMAND_DURATION_MS");
    trace_runtime_phase("stream_chat_command", "start");

    let job = StreamChatJob {
        pane_id,
        provider_id,
        builder_id,
        account_id,
        model_id,
        assistant_message_id,
        reasoning_level,
    };

    tauri::async_runtime::spawn(run_background_stream_chat(
        app.clone(),
        Arc::clone(database.inner()),
        Arc::clone(credentials.inner()),
        Arc::clone(oauth.inner()),
        Arc::clone(stream_persistence.inner()),
        Arc::clone(scope_cache.inner()),
        job,
    ));

    emit_main_thread_block_total();
    trace_runtime_phase("stream_chat_command", "complete");
    command_span.finish();
    Ok(())
}

#[tauri::command]
pub fn runtime_probe_ping() -> Result<u64, String> {
    trace_runtime_phase("runtime_probe_ping", "ok");
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as u64)
}

pub(crate) struct PreparedStreamExecution {
    pub execution_context: PaneExecutionContext,
    pub conversation: Conversation,
    pub enrichment_plan: Option<FilesystemEnrichmentPlan>,
}

pub(crate) struct FilesystemEnrichmentPlan {
    pub base_conversation: Conversation,
    pub scope: crate::filesystem_tools::ApprovedScope,
    pub prompt: String,
    pub tool_calls: Vec<FilesystemToolCall>,
}

pub(crate) fn prepare_stream_execution_db_only(
    connection: &rusqlite::Connection,
    scope_cache: Option<&crate::project_scope_cache::ProjectScopeCache>,
    pane_id: &str,
    provider_id: &str,
    account_id: &str,
    model_id: &str,
    reasoning_level: Option<&str>,
) -> Result<PreparedStreamExecution, StorageError> {
    let pane = PaneRepository::get_open_for_execution(connection, pane_id)?;
    let now = chrono::Utc::now().to_rfc3339();
    let metadata_json =
        pane_metadata_with_reasoning(pane.metadata_json.as_deref(), reasoning_level)?;
    let updated = connection.execute(
        "UPDATE panes
         SET provider_id = ?1,
             account_id = ?2,
             model_id = ?3,
             metadata_json = ?4,
             updated_at = ?5
         WHERE id = ?6 AND closed_at IS NULL",
        (
            provider_id,
            account_id,
            model_id,
            &metadata_json,
            &now,
            pane_id,
        ),
    )?;

    if updated == 0 {
        return Err(StorageError::NotFound(format!(
            "open pane {pane_id} not found"
        )));
    }

    let mut pane_for_resolution = pane.clone();
    pane_for_resolution.provider_id = Some(provider_id.to_string());
    pane_for_resolution.account_id = Some(account_id.to_string());
    pane_for_resolution.model_id = Some(model_id.to_string());
    pane_for_resolution.metadata_json = Some(metadata_json);

    let execution_context = match ProviderResolutionService::load_execution_context_from_pane(
        connection,
        &pane_for_resolution,
    ) {
        Ok(context) => context,
        Err(_error)
            if account_id.is_empty() && global_engine_registry().get(provider_id).is_some() =>
        {
            PaneExecutionContext {
                provider: ProviderDto {
                    id: provider_id.to_string(),
                    provider_type: provider_id.to_string(),
                    display_name: provider_id.to_string(),
                    enabled: true,
                    auth_mode: "none".to_string(),
                    supports_chat: true,
                    supports_streaming: true,
                    supports_tool_use: true,
                    supports_vision: false,
                    context_window: None,
                    locality: "local".to_string(),
                },
                credential: crate::auth::CredentialHandle {
                    provider_id: provider_id.to_string(),
                    account_id: String::new(),
                    auth_type: "none".to_string(),
                    credential_ref: String::new(),
                    token_expires_at: None,
                },
                oauth_external_account_id: None,
            }
        }
        Err(error) => {
            return Err(StorageError::InvalidInput(format!(
                "provider resolution error: {error:?}"
            )));
        }
    };
    let mut conversation = conversation_for_stream(connection, pane_id, model_id)?;
    let project_id = pane.project_id.as_deref().ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "pane {pane_id} is missing project_id for filesystem scope"
        ))
    })?;

    let cached_scope = if let Some(cache) = scope_cache {
        cache.resolve(connection, project_id)?
    } else {
        let project = ProjectRepository::get_by_id(connection, project_id)?;
        let scope = crate::filesystem_tools::ApprovedScope::new(project.approved_root.clone())
            .map_err(|error| StorageError::InvalidInput(error.to_string()))?;
        crate::project_scope_cache::CachedProjectScope {
            project_id: project_id.to_string(),
            project_name: project.name,
            approved_root: project.approved_root,
            scope,
        }
    };

    conversation = conversation.with_message(Message::new(
        MessageRole::System,
        format!(
            "Project: {}\nApproved root: {}\nFilesystem enrichment is running in the background before the model response begins.",
            cached_scope.project_name, cached_scope.approved_root
        ),
    ));

    let enrichment_plan =
        prepare_filesystem_enrichment_with_scope(connection, &cached_scope.scope, &conversation)?;

    Ok(PreparedStreamExecution {
        execution_context,
        conversation,
        enrichment_plan,
    })
}

pub(crate) fn run_filesystem_enrichment_async(
    plan: FilesystemEnrichmentPlan,
) -> Result<Conversation, StorageError> {
    std::thread::spawn(move || enrich_conversation_with_filesystem(plan))
        .join()
        .map_err(|_| {
            StorageError::InvalidInput("filesystem enrichment thread panicked".to_string())
        })?
}

pub(crate) fn enrich_conversation_with_filesystem(
    plan: FilesystemEnrichmentPlan,
) -> Result<Conversation, StorageError> {
    trace_filesystem_tool_loop("FILESYSTEM_ROUTER_MATCHED=true");
    trace_filesystem_tool_loop(&format!("APPROVED_ROOT={}", plan.scope.root_display()));
    trace_filesystem_tool_loop("LOAD_FAILURE_REASON=");

    let scan_span = PerfSpan::start("FILESYSTEM_SCAN_DURATION_MS");
    let results = execute_filesystem_tool_calls(&plan.scope, &plan.prompt, &plan.tool_calls)?;
    scan_span.finish();

    if results.is_empty() {
        return Ok(plan.base_conversation);
    }

    let prompt_span = PerfSpan::start("PROMPT_BUILD_DURATION_MS");
    let injected_prompt = match format_filesystem_tool_results(&results) {
        Ok(prompt) => {
            trace_perf_metric("PROMPT_INJECTION_SIZE", prompt.len());
            trace_filesystem_tool_loop(&format!("RESULT_SIZE={}", prompt.len()));
            trace_filesystem_tool_loop(&format!("PROMPT_INJECTION_SIZE={}", prompt.len()));
            trace_filesystem_tool_loop("PROMPT_INJECTION=success");
            prompt
        }
        Err(error) => {
            trace_filesystem_tool_loop("PROMPT_INJECTION=failure");
            return Err(error);
        }
    };
    prompt_span.finish();

    Ok(plan
        .base_conversation
        .with_message(Message::new(MessageRole::System, injected_prompt)))
}

pub(crate) fn prepare_filesystem_enrichment(
    connection: &rusqlite::Connection,
    project_id: &str,
    conversation: &Conversation,
) -> Result<Option<FilesystemEnrichmentPlan>, StorageError> {
    trace_project_root_lookup(connection, project_id);
    let scope = match ProjectRepository::load_scope(connection, project_id) {
        Ok(scope) => scope,
        Err(error) => {
            trace_filesystem_tool_loop("APPROVED_ROOT=<load_failed>");
            trace_filesystem_tool_loop(&format!("LOAD_FAILURE_REASON={error}"));
            return Err(StorageError::InvalidInput(error.to_string()));
        }
    };
    prepare_filesystem_enrichment_with_scope(connection, &scope, conversation)
}

pub(crate) fn prepare_filesystem_enrichment_with_scope(
    _connection: &rusqlite::Connection,
    scope: &crate::filesystem_tools::ApprovedScope,
    conversation: &Conversation,
) -> Result<Option<FilesystemEnrichmentPlan>, StorageError> {
    let Some(prompt) = latest_user_prompt(conversation) else {
        return Ok(None);
    };
    let routed = route_filesystem_tools(prompt);
    if routed.tools.is_empty() {
        return Ok(None);
    }

    Ok(Some(FilesystemEnrichmentPlan {
        base_conversation: conversation.clone(),
        scope: scope.clone(),
        prompt: prompt.to_string(),
        tool_calls: routed.tools,
    }))
}

pub(crate) fn apply_stream_chunk<R: Runtime>(
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
        MessageRepository::append_stream_delta(
            connection,
            assistant_message_id,
            &chunk.content_delta,
        )?;
        emit_stream_chunk(app, pane_id, assistant_message_id, &chunk.content_delta);
    }

    Ok(())
}

pub(crate) fn flush_stream_delta(
    connection: &rusqlite::Connection,
    assistant_message_id: &str,
    delta: &str,
) -> Result<(), StorageError> {
    if delta.is_empty() {
        return Ok(());
    }
    MessageRepository::append_stream_delta(connection, assistant_message_id, delta)
}

pub(crate) fn conversation_for_stream(
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
        let role = crate::models::message_role_from_db(&message.role);
        if let Some(role) = role {
            conversation = conversation.with_message(Message::new(role, message.content));
        }
    }
    Ok(conversation)
}

fn conversation_with_filesystem_tool_results(
    connection: &rusqlite::Connection,
    project_id: &str,
    conversation: Conversation,
) -> Result<Conversation, StorageError> {
    let enrichment_plan = prepare_filesystem_enrichment(connection, project_id, &conversation)?;
    let Some(plan) = enrichment_plan else {
        return Ok(conversation);
    };
    enrich_conversation_with_filesystem(plan)
}

fn latest_user_prompt(conversation: &Conversation) -> Option<&str> {
    conversation
        .messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::User)
        .map(|message| message.content.as_str())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FilesystemToolResult {
    tool: &'static str,
    input: serde_json::Value,
    output: serde_json::Value,
}

fn execute_filesystem_tool_calls(
    scope: &crate::filesystem_tools::ApprovedScope,
    prompt: &str,
    calls: &[FilesystemToolCall],
) -> Result<Vec<FilesystemToolResult>, StorageError> {
    let mut results = Vec::new();
    let mut shared_scan_context = ScanContext::for_tool(prompt, ".");
    let mut read_cache: std::collections::HashMap<
        String,
        crate::filesystem_tools::models::ReadFileResult,
    > = std::collections::HashMap::new();
    for call in calls {
        trace_filesystem_tool_loop(&format!("TOOL={}", call.trace_tool_name()));
        match call {
            FilesystemToolCall::ListDirectory { path } => {
                let output = match FilesystemService::list_directory(scope, path) {
                    Ok(output) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=true");
                        output
                    }
                    Err(error) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                        return Err(StorageError::InvalidInput(error.to_string()));
                    }
                };
                results.push(FilesystemToolResult {
                    tool: "list_directory",
                    input: serde_json::json!({ "path": path }),
                    output: serde_json::to_value(output)?,
                });
            }
            FilesystemToolCall::ReadFile { path } => {
                let output = if let Some(cached) = read_cache.get(path) {
                    Ok(cached.clone())
                } else {
                    FilesystemService::read_file(scope, path).map(|output| {
                        read_cache.insert(path.clone(), output.clone());
                        output
                    })
                };
                match output {
                    Ok(output) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=true");
                        results.push(FilesystemToolResult {
                            tool: "read_file",
                            input: serde_json::json!({ "path": path }),
                            output: serde_json::to_value(output)?,
                        })
                    }
                    Err(crate::filesystem_tools::FilesystemError::NotAFile(_)) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                    }
                    Err(crate::filesystem_tools::FilesystemError::NotFound(_)) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                    }
                    Err(crate::filesystem_tools::FilesystemError::Io(_)) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                    }
                    Err(error) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                        return Err(StorageError::InvalidInput(error.to_string()));
                    }
                }
            }
            FilesystemToolCall::FindFiles { path, pattern } => {
                let output = match FilesystemService::find_files_with_context(
                    scope,
                    path,
                    pattern,
                    &mut shared_scan_context,
                ) {
                    Ok(output) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=true");
                        output
                    }
                    Err(error) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                        return Err(StorageError::InvalidInput(error.to_string()));
                    }
                };
                results.push(FilesystemToolResult {
                    tool: "find_files",
                    input: serde_json::json!({ "path": path, "pattern": pattern }),
                    output: serde_json::to_value(output)?,
                });
            }
            FilesystemToolCall::SearchFiles { path, query } => {
                let output = match FilesystemService::search_files_with_context(
                    scope,
                    path,
                    query,
                    &mut shared_scan_context,
                ) {
                    Ok(output) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=true");
                        output
                    }
                    Err(error) => {
                        trace_filesystem_tool_loop("TOOL_SUCCESS=false");
                        return Err(StorageError::InvalidInput(error.to_string()));
                    }
                };
                results.push(FilesystemToolResult {
                    tool: "search_files",
                    input: serde_json::json!({ "path": path, "query": query }),
                    output: serde_json::to_value(output)?,
                });
            }
        }
    }
    Ok(results)
}

fn trace_filesystem_tool_loop(message: &str) {
    if std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() == Ok("1") {
        println!("{message}");
    }
}

fn trace_project_root_lookup(connection: &rusqlite::Connection, project_id: &str) {
    if std::env::var("BUILDERBOARD_TRACE_OPENAI_EXECUTION").as_deref() != Ok("1") {
        return;
    }

    println!("PROJECT_ID={project_id}");
    match ProjectRepository::get_approved_root(connection, project_id) {
        Ok(approved_root) => {
            println!("APPROVED_ROOT_ROW_FOUND=true");
            println!("APPROVED_ROOT_VALUE={approved_root}");
        }
        Err(error) => {
            println!("APPROVED_ROOT_ROW_FOUND=false");
            println!("APPROVED_ROOT_VALUE=");
            println!("LOAD_FAILURE_REASON=project lookup failed: {error}");
        }
    }
}

fn format_filesystem_tool_results(
    results: &[FilesystemToolResult],
) -> Result<String, StorageError> {
    let compact_results = compact_results_for_injection(results);
    let serialization_span = PerfSpan::start("PROMPT_SERIALIZATION_DURATION_MS");
    let payload = serde_json::to_string(&compact_results)?;
    serialization_span.finish();
    let header = "Filesystem tool results follow. Use these read-only approved-root results to answer the user. Do not claim you cannot access the project.\n";
    if header.len() + payload.len() <= MAX_PROMPT_INJECTION_BYTES {
        return Ok(format!("{header}{payload}"));
    }

    let truncated_payload = serde_json::to_string(&compact_results_for_injection_with_budget(
        results,
        MAX_PROMPT_INJECTION_BYTES.saturating_sub(header.len()),
    ))?;
    Ok(format!("{header}{truncated_payload}"))
}

fn compact_results_for_injection(results: &[FilesystemToolResult]) -> Vec<FilesystemToolResult> {
    compact_results_for_injection_with_budget(results, MAX_PROMPT_INJECTION_BYTES)
}

fn compact_results_for_injection_with_budget(
    results: &[FilesystemToolResult],
    byte_budget: usize,
) -> Vec<FilesystemToolResult> {
    let mut compact = Vec::new();
    let mut remaining = byte_budget;

    for result in results {
        let mut next = result.clone();
        match result.tool {
            "read_file" => truncate_read_file_output(&mut next.output),
            "find_files" => truncate_find_files_output(&mut next.output),
            "search_files" => truncate_search_files_output(&mut next.output),
            _ => {}
        }

        let encoded = serde_json::to_string(&next).unwrap_or_default();
        if encoded.len() > remaining && !compact.is_empty() {
            break;
        }
        if encoded.len() > remaining {
            next.output = serde_json::json!({
                "truncated": true,
                "reason": "prompt byte budget exceeded"
            });
        }
        remaining = remaining.saturating_sub(encoded.len());
        compact.push(next);
    }

    compact
}

fn truncate_read_file_output(output: &mut serde_json::Value) {
    let Some(content) = output.get_mut("content").and_then(|value| value.as_str()) else {
        return;
    };
    if content.chars().count() > MAX_INJECTED_READ_FILE_CHARS {
        let truncated: String = content.chars().take(MAX_INJECTED_READ_FILE_CHARS).collect();
        *output = serde_json::json!({
            "path": output.get("path").cloned().unwrap_or(serde_json::Value::Null),
            "content": format!("{truncated}\n...[truncated for prompt size]"),
            "sizeBytes": output.get("sizeBytes").cloned().unwrap_or(serde_json::Value::Null),
            "truncated": true
        });
    }
}

fn truncate_find_files_output(output: &mut serde_json::Value) {
    let Some(matches) = output.get("matches").and_then(|value| value.as_array()) else {
        return;
    };
    if matches.len() <= MAX_INJECTED_FIND_PATHS {
        return;
    }
    let kept: Vec<_> = matches
        .iter()
        .take(MAX_INJECTED_FIND_PATHS)
        .cloned()
        .collect();
    *output = serde_json::json!({
        "path": output.get("path").cloned().unwrap_or(serde_json::Value::Null),
        "pattern": output.get("pattern").cloned().unwrap_or(serde_json::Value::Null),
        "matches": kept,
        "totalMatches": matches.len(),
        "truncated": true
    });
}

fn truncate_search_files_output(output: &mut serde_json::Value) {
    let Some(matches) = output.get("matches").and_then(|value| value.as_array()) else {
        return;
    };
    if matches.len() <= MAX_INJECTED_SEARCH_FILES {
        return;
    }
    let kept: Vec<_> = matches
        .iter()
        .take(MAX_INJECTED_SEARCH_FILES)
        .cloned()
        .collect();
    *output = serde_json::json!({
        "path": output.get("path").cloned().unwrap_or(serde_json::Value::Null),
        "query": output.get("query").cloned().unwrap_or(serde_json::Value::Null),
        "matches": kept,
        "totalMatches": matches.len(),
        "truncated": true
    });
}

fn model_from_id(model_id: &str) -> Model {
    match model_id {
        "OpenAIGpt" | "gpt-4o-mini" => Model::OpenAIGpt,
        "GPT-5.5" => Model::Custom("gpt-5.5".to_string()),
        "GPT-5.4 mini" => Model::Custom("gpt-5.4-mini".to_string()),
        "GPT-5.3 Codex Spark" => Model::Custom("gpt-5.3-codex-spark".to_string()),
        "gpt-5.5" | "gpt-5.4-mini" | "gpt-5.3-codex-spark" => Model::Custom(model_id.to_string()),
        other => Model::Custom(other.to_string()),
    }
}

fn pane_metadata_with_reasoning(
    current_metadata_json: Option<&str>,
    reasoning_level: Option<&str>,
) -> Result<String, StorageError> {
    let mut metadata = current_metadata_json
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    if let Some(reasoning_level) = reasoning_level {
        validate_reasoning_level(reasoning_level)?;
        metadata.insert(
            "reasoningLevel".to_string(),
            serde_json::Value::String(reasoning_level.to_string()),
        );
    }

    Ok(serde_json::Value::Object(metadata).to_string())
}

fn validate_reasoning_level(reasoning_level: &str) -> Result<(), StorageError> {
    match reasoning_level {
        "low" | "medium" | "high" | "xhigh" => Ok(()),
        other => Err(StorageError::InvalidInput(format!(
            "unsupported OpenAI reasoning level: {other}"
        ))),
    }
}

pub(crate) fn emit_stream_enrichment_started<R: Runtime>(
    app: &AppHandle<R>,
    pane_id: &str,
    message_id: &str,
) {
    let _ = app.emit(
        "message_stream_enrichment_started",
        serde_json::json!({
            "paneId": pane_id,
            "messageId": message_id,
        }),
    );
}

pub(crate) fn emit_stream_chunk<R: Runtime>(
    app: &AppHandle<R>,
    pane_id: &str,
    message_id: &str,
    delta: &str,
) {
    let _ = app.emit(
        "message_stream_chunk",
        serde_json::json!({
            "paneId": pane_id,
            "messageId": message_id,
            "delta": delta,
        }),
    );
}

pub(crate) fn emit_stream_complete<R: Runtime>(
    app: &AppHandle<R>,
    pane_id: &str,
    message_id: &str,
) {
    let _ = app.emit(
        "message_stream_complete",
        serde_json::json!({
            "paneId": pane_id,
            "messageId": message_id,
        }),
    );
}

pub(crate) fn emit_stream_error<R: Runtime>(
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
        account_list_from_database, conversation_with_filesystem_tool_results, model_from_id,
        pane_metadata_with_reasoning, prepare_filesystem_enrichment,
        prepare_stream_execution_db_only, provider_list_from_database,
        run_filesystem_enrichment_async,
    };
    use crate::auth::CredentialService;
    use crate::filesystem_intent::{route_filesystem_tools, FilesystemToolCall};
    use crate::filesystem_tools::approved_root::set_approved_root;
    use crate::models::Model;
    use crate::models::{Conversation, Message, MessageRole};
    use crate::storage::db::{test_database_path, Database};
    use crate::storage::error::StorageResult;
    use crate::storage::models::{CreatePaneRequest, MessageCreateRequest};
    use crate::storage::repositories::accounts::AccountRepository;
    use crate::storage::repositories::messages::MessageRepository;
    use crate::storage::repositories::panes::PaneRepository;

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

    #[test]
    fn openai_model_ids_and_reasoning_metadata_are_preserved() -> StorageResult<()> {
        assert_eq!(
            model_from_id("gpt-5.5"),
            Model::Custom("gpt-5.5".to_string())
        );
        assert_eq!(
            model_from_id("GPT-5.4 mini"),
            Model::Custom("gpt-5.4-mini".to_string())
        );
        assert_eq!(
            model_from_id("gpt-5.3-codex-spark"),
            Model::Custom("gpt-5.3-codex-spark".to_string())
        );

        let metadata = pane_metadata_with_reasoning(Some(r#"{"theme":"dark"}"#), Some("high"))?;
        let value: serde_json::Value = serde_json::from_str(&metadata)?;
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["reasoningLevel"], "high");

        assert!(pane_metadata_with_reasoning(None, Some("extreme")).is_err());
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

    fn setup_filesystem_root(name: &str) -> StorageResult<(Database, String)> {
        let (database, _credentials) = setup_services(name)?;
        let root = std::env::temp_dir()
            .join("builderboard-tests")
            .join(format!("{name}-root"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src"))?;
        fs::write(
            root.join("package.json"),
            r#"{"name":"pepfox","dependencies":{"oauth-lib":"1.0.0"}}"#,
        )?;
        fs::write(root.join("README.md"), "# PepFox\nA test project.")?;
        fs::write(
            root.join("src").join("auth.ts"),
            "export const OAuth = true;",
        )?;
        fs::write(
            root.join("src").join("index.ts"),
            "export const app = 'PepFox';",
        )?;
        let project_id = database.with_connection(|connection| {
            crate::projects::repository::ProjectRepository::create_from_folder(
                connection,
                root.to_str().unwrap(),
                true,
            )
            .map(|project| project.id)
        })?;
        Ok((database, project_id))
    }

    fn tool_context_for_prompt(prompt: &str) -> StorageResult<String> {
        let (database, project_id) =
            setup_filesystem_root(&format!("tool-loop-{}.db", uuid::Uuid::new_v4()))?;
        let conversation = Conversation::new("pane-1", Model::OpenAIGpt)
            .with_message(Message::new(MessageRole::User, prompt));
        let conversation = database.with_connection(|connection| {
            conversation_with_filesystem_tool_results(connection, &project_id, conversation)
        })?;
        Ok(conversation
            .messages
            .last()
            .expect("tool context message")
            .content
            .clone())
    }

    #[test]
    fn filesystem_tool_router_matches_validation_scenarios() {
        assert!(matches!(
            route_filesystem_tools("Review the project structure.")
                .tools
                .first(),
            Some(FilesystemToolCall::ListDirectory { path }) if path == "."
        ));
        assert!(
            route_filesystem_tools("Review /Users/sterlingdigital/PepFox")
                .tools
                .iter()
                .any(|call| {
                    matches!(call, FilesystemToolCall::ListDirectory { path } if path == ".")
                })
        );
        assert!(matches!(
            route_filesystem_tools("Take a look at PepFox")
                .tools
                .first(),
            Some(FilesystemToolCall::ListDirectory { path }) if path == "."
        ));
        assert!(route_filesystem_tools("Review package.json.")
            .tools
            .iter()
            .any(|call| {
                matches!(call, FilesystemToolCall::ReadFile { path } if path == "package.json")
            }));
        assert!(route_filesystem_tools("Find OAuth code.")
            .tools
            .iter()
            .any(|call| {
                matches!(call, FilesystemToolCall::SearchFiles { path, query } if path == "." && query == "OAuth")
            }));
        assert!(route_filesystem_tools("Find all TypeScript files.")
            .tools
            .iter()
            .any(|call| {
                matches!(call, FilesystemToolCall::FindFiles { path, pattern } if path == "." && pattern == "*.ts")
            }));
        assert!(
            !route_filesystem_tools("Perform a production readiness review")
                .tools
                .is_empty()
        );
        assert!(route_filesystem_tools("Find security concerns")
            .tools
            .iter()
            .any(|call| matches!(call, FilesystemToolCall::SearchFiles { .. })));
        assert!(route_filesystem_tools("Identify technical debt")
            .tools
            .iter()
            .any(|call| matches!(call, FilesystemToolCall::FindFiles { .. })));
    }

    #[test]
    fn filesystem_tool_loop_injects_list_directory_results() -> StorageResult<()> {
        let context = tool_context_for_prompt("Review the project structure.")?;
        assert!(context.contains("Filesystem tool results follow"));
        assert!(context.contains("list_directory"));
        assert!(context.contains("package.json"));
        Ok(())
    }

    #[test]
    fn filesystem_tool_loop_injects_read_file_results() -> StorageResult<()> {
        let context = tool_context_for_prompt("Review package.json.")?;
        assert!(context.contains("read_file"));
        assert!(context.contains("pepfox"));
        Ok(())
    }

    #[test]
    fn filesystem_tool_loop_injects_search_results() -> StorageResult<()> {
        let context = tool_context_for_prompt("Find OAuth code.")?;
        assert!(context.contains("search_files"));
        assert!(context.contains("src/auth.ts"));
        assert!(context.contains("OAuth"));
        Ok(())
    }

    #[test]
    fn filesystem_tool_loop_injects_find_files_results() -> StorageResult<()> {
        let context = tool_context_for_prompt("Find all TypeScript files.")?;
        assert!(
            context.contains(r#""tool":"find_files"#)
                || context.contains(r#""tool": "find_files""#)
        );
        assert!(context.contains("src/auth.ts"));
        assert!(context.contains("src/index.ts"));
        Ok(())
    }

    #[test]
    fn review_prompt_avoids_find_files_and_caps_payload_size() -> StorageResult<()> {
        let context = tool_context_for_prompt("Review the project architecture.")?;
        assert!(!context.contains(r#""tool":"find_files"#));
        assert!(
            context.contains(r#""tool":"list_directory"#)
                || context.contains(r#""tool": "list_directory""#)
        );
        assert!(
            context.len() <= 24_576,
            "review payload should stay within injection budget"
        );
        Ok(())
    }

    #[test]
    fn review_prompt_includes_src_listing_sample() -> StorageResult<()> {
        let context = tool_context_for_prompt("Review the project structure.")?;
        assert!(context.contains("src"));
        Ok(())
    }

    #[test]
    fn async_filesystem_enrichment_preserves_tool_results() -> StorageResult<()> {
        let (database, project_id) =
            setup_filesystem_root(&format!("async-enrich-{}.db", uuid::Uuid::new_v4()))?;
        let conversation = Conversation::new("pane-1", Model::OpenAIGpt)
            .with_message(Message::new(MessageRole::User, "Find OAuth code."));

        let plan = database.with_connection(|connection| {
            prepare_filesystem_enrichment(connection, &project_id, &conversation)
        })?;
        let enriched = run_filesystem_enrichment_async(plan.expect("enrichment plan"))?;

        let context = enriched
            .messages
            .last()
            .expect("enriched context")
            .content
            .clone();
        assert!(context.contains("search_files"));
        assert!(context.contains("OAuth"));
        Ok(())
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

    #[test]
    fn stream_prepare_builds_enrichment_plan_in_one_pass() -> StorageResult<()> {
        let (database, project_id) =
            setup_filesystem_root(&format!("stream-prepare-{}.db", uuid::Uuid::new_v4()))?;
        let account = database.with_connection(|connection| {
            AccountRepository::insert_test_account(
                connection,
                "prepare-openai",
                "openai",
                "api_key",
                "active",
                true,
            )?;
            Ok("prepare-openai".to_string())
        })?;
        let pane_id = database.with_connection(|connection| {
            let pane = PaneRepository::create(
                connection,
                CreatePaneRequest {
                    workspace_id: None,
                    project_id: Some(project_id),
                    title: Some("Prepare pane".to_string()),
                    sort_order: None,
                },
            )?;
            MessageRepository::create_conversation_turn(
                connection,
                MessageCreateRequest {
                    pane_id: pane.id.clone(),
                    content: "Run a security review of this project".to_string(),
                    content_type: Some("text".to_string()),
                    metadata_json: None,
                },
            )?;
            Ok(pane.id)
        })?;

        let prepared = database.with_connection(|connection| {
            prepare_stream_execution_db_only(
                connection,
                None,
                &pane_id,
                "openai",
                &account,
                "OpenAIGpt",
                Some("medium"),
            )
        })?;

        assert!(prepared.enrichment_plan.is_some());
        assert_eq!(prepared.execution_context.credential.account_id, account);
        Ok(())
    }
}
