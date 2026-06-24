use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tokio::sync::mpsc;

use crate::auth::{CredentialService, OAuthService};
use crate::chat::ProviderResolutionService;
use crate::execution::{global_engine_registry, ExecutionContext, ExecutionEvent, ExecutionRequest};
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

    // Route ALL execution through the (now generalized) ExecutionEngine abstraction.
    // The abstraction is execution-centric. OpenAI remains the only engine for now
    // and produces exactly the same deltas, metrics, and side-effects as before.
    // Phase 8.9E: Intelligent routing via ExecutionManager when a Builder is indicated.
    // Builder intent + class -> scored engine selection with reason.
    // Falls back gracefully. Manual (non-builder) paths unchanged.
    let manager = crate::execution::manager::ExecutionManager::new();
    let exec_ctx = prepared.execution_context.clone(); // reuse for context (limited but sufficient)

    let resolution = if job.provider_id == "builder-a" || job.provider_id == "builder-b" || job.provider_id == "builder-c" {
        // Builder selected as "provider" for demo; derive class from model or default Implementation
        let class = if job.model_id.to_lowercase().contains("review") || job.model_id.to_lowercase().contains("arch") {
            crate::execution::ExecutionClass::Review
        } else if job.model_id.to_lowercase().contains("test") || job.model_id.to_lowercase().contains("debug") {
            crate::execution::ExecutionClass::Testing
        } else {
            crate::execution::ExecutionClass::Implementation
        };
        // Use manager for intelligent choice + reason (logged). Use minimal context for resolution.
        // Minimal resolution call (context passed is approximate for this phase)
        let dummy_ctx = crate::execution::ExecutionContext::local(job.pane_id.clone());
        let res = crate::execution::manager::ExecutionManager::resolve(Some(&job.provider_id), Some(class.clone()), &dummy_ctx, &crate::execution::ExecutionRequest::chat(
            // Use empty conversation for resolution decision only (real one is prepared later)
            crate::models::Conversation::new("resolution", crate::models::Model::Custom("decision".into())),
            job.reasoning_level.clone()
        ));
        eprintln!("[ExecutionManager] Builder={} Class={} -> Engine={} Model={} Reason: {}",
            job.provider_id, class.as_str(), res.engine_id, res.model, res.reason);
        res
    } else if job.model_id.to_lowercase().contains("grok") || job.provider_id.to_lowercase().contains("grok") {
        // Direct grok model
        crate::execution::ExecutionResolution {
            engine_id: "grok".to_string(),
            model: job.model_id.clone(),
            effort: job.reasoning_level.clone().unwrap_or_else(|| "high".to_string()),
            reason: "Direct grok model selection".to_string(),
            class: crate::execution::ExecutionClass::Implementation,
            policy_applied: false,
        }
    } else {
        // Legacy / OpenAI direct
        crate::execution::ExecutionResolution {
            engine_id: job.provider_id.clone(),
            model: job.model_id.clone(),
            effort: job.reasoning_level.clone().unwrap_or_else(|| "medium".to_string()),
            reason: "Direct provider/model selection".to_string(),
            class: crate::execution::ExecutionClass::General,
            policy_applied: false,
        }
    };

    let engine_key = resolution.engine_id.clone();
    let effective_model = resolution.model.clone();
    let effective_effort = resolution.effort.clone();

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

    let openai_span = PerfSpan::start("OPENAI_REQUEST_DURATION_MS");
    let openai_request_start = std::time::Instant::now();
    trace_runtime_phase("openai_stream", "start");

    // Build generalized request and context (future-proof)
    let exec_request = ExecutionRequest::chat(conversation, job.reasoning_level.clone());

    // Build a rich but minimal ExecutionContext from existing data.
    // For the current OpenAI chat path, credential material is still supplied.
    // Local engines will see credential = None.
    let exec_context = ExecutionContext::from_pane_project(
        job.assistant_message_id.clone(),
        Some(job.pane_id.clone()),
        None, // project enrichment happens outside engine (unchanged)
        None,
        None, // cwd can be set by specific engines or future logic
    );

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
                        trace_perf_metric("OPENAI_REQUEST_DURATION_MS", openai_request_start.elapsed().as_millis());
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
            exec_context,
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

    trace_runtime_phase("openai_stream", "complete");

    // Post-stream metric/tracing kept for diagnostic parity. Event worker handled first-chunk timing.
    trace_perf_metric("OPENAI_STREAM_TOTAL_MS", openai_span.elapsed_ms());
    let _ = openai_span.finish();
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
        account_id: account_id.to_string(),
        model_id: model_id.to_string(),
        assistant_message_id: assistant_message_id.to_string(),
        reasoning_level: reasoning_level.map(str::to_string),
    };

    tauri::async_runtime::block_on(run_background_stream_chat_inner(
        app,
        database,
        credentials,
        oauth,
        stream_persistence,
        scope_cache,
        &job,
    ))
}