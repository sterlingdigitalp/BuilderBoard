use std::sync::Arc;

use tauri::{AppHandle, Runtime};

use crate::auth::{CredentialService, OAuthService};
use crate::chat::ProviderResolutionService;
use crate::filesystem_tools::perf::{trace_perf_metric, PerfSpan};
use crate::project_scope_cache::ProjectScopeCache;
use crate::providers::{ProviderError, ProviderRequest, ProviderResolutionError};
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

    let openai_provider = ProviderResolutionService::resolve_openai_provider(
        prepared.execution_context,
        credentials,
    )
    .map_err(map_provider_resolution_error)?;

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

    let openai_span = PerfSpan::start("OPENAI_REQUEST_DURATION_MS");
    trace_runtime_phase("openai_stream", "start");

    let request = ProviderRequest::new(conversation)
        .with_reasoning_level(job.reasoning_level.clone());

    let mut openai_started = false;
    let mut write_buffer = StreamWriteBuffer::new(
        Arc::clone(stream_persistence),
        job.pane_id.clone(),
        job.assistant_message_id.clone(),
    );

    let stream_result = openai_provider
        .stream_chunks_async(request, |chunk| {
            if !openai_started {
                trace_perf_metric("OPENAI_REQUEST_DURATION_MS", openai_span.elapsed_ms());
                openai_started = true;
            }

            match chunk {
                Ok(chunk) => {
                    if chunk.is_complete {
                        return Ok(());
                    }
                    write_buffer
                        .push(app, &chunk.content_delta)
                        .map_err(|error| ProviderError::InvalidResponse {
                            message: error.to_string(),
                        })
                }
                Err(error) => Err(error),
            }
        })
        .await;

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

    write_buffer.finish_with_complete(app).await?;

    trace_runtime_phase("openai_stream", "complete");

    if !openai_started {
        openai_span.finish();
    } else {
        trace_perf_metric("OPENAI_STREAM_TOTAL_MS", openai_span.elapsed_ms());
    }
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