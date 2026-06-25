use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tokio::sync::mpsc;

use crate::auth::{CredentialService, OAuthService};
use crate::chat::ProviderResolutionService;
use crate::execution::{global_engine_registry, ExecutionContext, ExecutionEvent, ExecutionManager, ExecutionRequest};
use crate::filesystem_tools::perf::{trace_perf_metric, PerfSpan};
use crate::project_scope_cache::ProjectScopeCache;
use crate::providers::{ProviderError, ProviderResolutionError};
use crate::runtime_diagnostics::trace_runtime_phase;
use crate::storage::commands::{
    emit_stream_enrichment_started, emit_stream_error, enrich_conversation_with_filesystem,
    prepare_stream_execution_db_only,
};
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::stream_persistence::StreamPersistenceService;
use crate::stream_write_buffer::StreamWriteBuffer;

#[derive(Clone, Debug)]
pub struct StreamChatJob {
    pub pane_id: String,
    pub provider_id: String,
    pub builder_id: Option<String>,
    pub account_id: String,
    pub model_id: String,
    pub assistant_message_id: String,
    pub reasoning_level: Option<String>,
}

pub async fn run_background_stream_chat<R: Runtime>(
    app: AppHandle<R>,
    database: Arc<Database>,
    credentials: Arc<CredentialService>,
    oauth: Arc<OAuthService>,
    stream_persistence: Arc<StreamPersistenceService>,
    scope_cache: Arc<ProjectScopeCache>,
    job: StreamChatJob,
) {
    if let Err(error) = run_background_stream_chat_inner(
        &app,
        &database,
        &credentials,
        Some(Arc::clone(&credentials)),
        &oauth,
        &stream_persistence,
        &scope_cache,
        &job,
    )
    .await
    {
        let message = error.to_string();
        let _ = stream_persistence.enqueue_error(
            &job.pane_id,
            &job.assistant_message_id,
            "provider_execution_failed",
            message.clone(),
        );
        let _ = stream_persistence.drain_message(&job.assistant_message_id).await;
        emit_stream_error(
            &app,
            &job.pane_id,
            &job.assistant_message_id,
            "provider_execution_failed",
            &message,
        );
    }
}

pub async fn run_background_stream_chat_inner<R: Runtime>(
    app: &AppHandle<R>,
    database: &Database,
    credentials: &CredentialService,
    credential_service: Option<Arc<CredentialService>>,
    oauth: &OAuthService,
    stream_persistence: &Arc<StreamPersistenceService>,
    scope_cache: &ProjectScopeCache,
    job: &StreamChatJob,
) -> Result<(), StorageError> {
    let total_span = PerfSpan::start("TOTAL_REQUEST_DURATION_MS");
    let ttft_span = PerfSpan::start("TTFT_MS");

    trace_runtime_phase("stream_chat_prepare", "start");
    let prepared = database.with_connection_labeled("stream_chat_prepare", |connection| {
        prepare_stream_execution_db_only(
            connection,
            Some(scope_cache),
            &job.pane_id,
            &job.provider_id,
            &job.account_id,
            &job.model_id,
            job.reasoning_level.as_deref(),
        )
    })?;
    trace_runtime_phase("stream_chat_prepare", "complete");

    if prepared.execution_context.credential.auth_type == "oauth" {
        ProviderResolutionService::refresh_oauth_access_token_if_needed(
            database,
            credentials,
            oauth,
            &prepared.execution_context,
        )
        .map_err(map_provider_resolution_error)?;
    }

    emit_stream_enrichment_started(app, &job.pane_id, &job.assistant_message_id);
    trace_perf_metric("TTFT_MS", ttft_span.elapsed_ms());

    trace_runtime_phase("filesystem_enrichment", "spawn_start");
    let conversation = if let Some(plan) = prepared.enrichment_plan {
        let fs_span = PerfSpan::start("FILESYSTEM_SCAN_DURATION_MS");
        let enriched = tauri::async_runtime::spawn_blocking(move || {
            enrich_conversation_with_filesystem(plan)
        })
        .await
        .map_err(|_| {
            StorageError::InvalidInput("filesystem enrichment task cancelled".to_string())
        })??;
        fs_span.finish();
        trace_runtime_phase("filesystem_enrichment", "spawn_complete");
        enriched
    } else {
        trace_runtime_phase("filesystem_enrichment", "skipped");
        prepared.conversation
    };

    let exec_request = ExecutionRequest::chat(conversation, job.reasoning_level.clone());
    let mut routing_context = ExecutionContext::from_pane_project(
        job.assistant_message_id.clone(),
        Some(job.pane_id.clone()),
        None,
        None,
        None,
    );
    routing_context.credential = Some(prepared.execution_context.credential.clone());
    routing_context.credential_service = credential_service;

    let route_id = job.builder_id.as_deref().unwrap_or(&job.provider_id);
    let resolution = ExecutionManager::resolve_stream_route(
        route_id,
        &job.model_id,
        job.reasoning_level.as_deref(),
        &routing_context,
        &exec_request,
    );

    let engine_key = resolution.engine_id.clone();

    // Note: We keep using conversation as-is (manager decision logged). Model/effort could be injected but to avoid changing execution behavior we note them.
    if !resolution.reason.is_empty() {
        // Surface reason for diagnostics (no UI change required)
        crate::runtime_diagnostics::trace_runtime_phase("execution_manager_decision", &resolution.reason);
    }

    let engine = global_engine_registry()
        .get(&engine_key)
        .ok_or_else(|| {
            StorageError::InvalidInput(format!(
                "no execution engine registered for '{}'",
                engine_key
            ))
        })?;

    let engine_span = PerfSpan::start("ENGINE_REQUEST_DURATION_MS");
    let engine_request_start = std::time::Instant::now();
    trace_runtime_phase("engine_stream", "start");

    // Buffer only used for finish (deltas now driven by events)
    let finish_write_buffer = StreamWriteBuffer::new(
        Arc::clone(stream_persistence),
        job.pane_id.clone(),
        job.assistant_message_id.clone(),
    );

    // Event-driven worker (generalized). TextDelta events are turned into the
    // exact same push + metric behavior the old StreamChunk path had.
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ExecutionEvent>();

    let app_clone = app.clone();
    let worker_persistence = Arc::clone(stream_persistence);
    let worker_pane = job.pane_id.clone();
    let worker_msg = job.assistant_message_id.clone();
    let worker = tauri::async_runtime::spawn(async move {
        let write_buffer = StreamWriteBuffer::new(worker_persistence, worker_pane, worker_msg);
        let mut started = false;
        while let Some(event) = event_rx.recv().await {
            match event {
                ExecutionEvent::TextDelta { content } => {
                    if !started {
                        trace_perf_metric("ENGINE_REQUEST_DURATION_MS", engine_request_start.elapsed().as_millis());
                        started = true;
                    }
                    if let Err(error) = write_buffer.push(&app_clone, &content) {
                        return Err(ProviderError::InvalidResponse {
                            message: error.to_string(),
                        });
                    }
                }
                ExecutionEvent::RunCompleted { .. } | ExecutionEvent::RunStarted { .. } => {
                    // no-op for delta path; finish handled after
                }
                ExecutionEvent::Error { message, .. } => {
                    return Err(ProviderError::InvalidResponse { message });
                }
                _ => {}
            }
        }
        Ok(())
    });

    // Call the generalized engine API
    let stream_result = engine
        .execute(
            routing_context,
            exec_request,
            Box::new(move |event| {
                let _ = event_tx.send(event);
            }),
        )
        .await;

    let _ = worker.await;

    if let Err(error) = stream_result {
        let message = format!("{error:?}");
        stream_persistence.enqueue_error(
            &job.pane_id,
            &job.assistant_message_id,
            "provider_error",
            message,
        )?;
        stream_persistence.drain_message(&job.assistant_message_id).await?;
        return Err(StorageError::InvalidInput(format!("provider stream error: {error:?}")));
    }

    finish_write_buffer.finish_with_complete(app).await?;

    trace_runtime_phase("engine_stream", "complete");

    // Post-stream metric/tracing kept for diagnostic parity. Event worker handled first-chunk timing.
    trace_perf_metric("ENGINE_STREAM_TOTAL_MS", engine_span.elapsed_ms());
    let _ = engine_span.finish();
    total_span.finish();
    Ok(())
}

fn map_provider_resolution_error(error: ProviderResolutionError) -> StorageError {
    StorageError::InvalidInput(format!("provider resolution error: {error:?}"))
}

pub fn stream_chat_with_services<R: Runtime>(
    app: &AppHandle<R>,
    database: &Database,
    credentials: &CredentialService,
    oauth: &OAuthService,
    stream_persistence: &Arc<StreamPersistenceService>,
    scope_cache: &ProjectScopeCache,
    pane_id: &str,
    provider_id: &str,
    account_id: &str,
    model_id: &str,
    assistant_message_id: &str,
    reasoning_level: Option<&str>,
) -> Result<(), StorageError> {
    let job = StreamChatJob {
        pane_id: pane_id.to_string(),
        provider_id: provider_id.to_string(),
        builder_id: None,
        account_id: account_id.to_string(),
        model_id: model_id.to_string(),
        assistant_message_id: assistant_message_id.to_string(),
        reasoning_level: reasoning_level.map(str::to_string),
    };

    tauri::async_runtime::block_on(run_background_stream_chat_inner(
        app,
        database,
        credentials,
        None,
        oauth,
        stream_persistence,
        scope_cache,
        &job,
    ))
}
