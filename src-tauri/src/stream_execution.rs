use std::sync::Arc;

use tauri::{AppHandle, Runtime};

use crate::auth::CredentialService;
use crate::chat::ProviderResolutionService;
use crate::filesystem_tools::perf::{trace_perf_metric, PerfSpan};
use crate::providers::{ProviderError, ProviderRequest, ProviderResolutionError};
use crate::runtime_diagnostics::trace_runtime_phase;
use crate::storage::commands::{
    apply_stream_chunk, emit_stream_complete, emit_stream_enrichment_started, emit_stream_error,
    enrich_conversation_with_filesystem, prepare_stream_execution_db_only,
};
use crate::storage::db::Database;
use crate::storage::error::StorageError;
use crate::storage::models::{MessageCompleteRequest, MessageErrorRequest};
use crate::storage::repositories::messages::MessageRepository;

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
    job: StreamChatJob,
) {
    if let Err(error) = run_background_stream_chat_inner(&app, &database, &credentials, &job).await {
        let message = error.to_string();
        let _ = database.with_connection(|connection| {
            MessageRepository::mark_error(
                connection,
                MessageErrorRequest {
                    message_id: job.assistant_message_id.clone(),
                    error_code: "provider_execution_failed".to_string(),
                    error_message: message.clone(),
                },
            )
        });
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
    job: &StreamChatJob,
) -> Result<(), StorageError> {
    let total_span = PerfSpan::start("TOTAL_REQUEST_DURATION_MS");
    let ttft_span = PerfSpan::start("TTFT_MS");

    trace_runtime_phase("stream_chat_prepare", "start");
    let prepared = database.with_connection_labeled("stream_chat_prepare", |connection| {
        prepare_stream_execution_db_only(
            connection,
            &job.pane_id,
            &job.provider_id,
            &job.account_id,
            &job.model_id,
            job.reasoning_level.as_deref(),
        )
    })?;
    trace_runtime_phase("stream_chat_prepare", "complete");

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

    let pane_id = job.pane_id.clone();
    let assistant_message_id = job.assistant_message_id.clone();
    let mut openai_started = false;

    openai_provider
        .stream_chunks_async(request, |chunk| {
            if !openai_started {
                trace_perf_metric("OPENAI_REQUEST_DURATION_MS", openai_span.elapsed_ms());
                openai_started = true;
            }

            match chunk {
                Ok(chunk) => database
                    .with_connection_labeled("stream_chat_chunk", |connection| {
                        apply_stream_chunk(app, connection, &pane_id, &assistant_message_id, chunk)
                    })
                    .map_err(|error| ProviderError::InvalidResponse {
                        message: error.to_string(),
                    }),
                Err(error) => {
                    let message = format!("{error:?}");
                    let _ = database.with_connection(|connection| {
                        MessageRepository::mark_error(
                            connection,
                            MessageErrorRequest {
                                message_id: assistant_message_id.clone(),
                                error_code: "provider_error".to_string(),
                                error_message: message.clone(),
                            },
                        )
                    });
                    Err(error)
                }
            }
        })
        .await
        .map_err(|error| StorageError::InvalidInput(format!("provider stream error: {error:?}")))?;

    trace_runtime_phase("openai_stream", "complete");

    database.with_connection_labeled("stream_chat_complete", |connection| {
        let latest = MessageRepository::get_by_id(connection, &job.assistant_message_id)?;
        if latest.status != "complete" {
            MessageRepository::mark_complete(
                connection,
                MessageCompleteRequest {
                    message_id: job.assistant_message_id.clone(),
                    content: None,
                    token_count_input: None,
                    token_count_output: None,
                    metadata_json: None,
                },
            )?;
            emit_stream_complete(app, &job.pane_id, &job.assistant_message_id);
        }
        Ok(())
    })?;

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
        &job,
    ))
}