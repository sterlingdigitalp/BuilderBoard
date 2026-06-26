use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tokio::sync::mpsc;

use crate::auth::{CredentialService, OAuthService};
use crate::chat::ProviderResolutionService;
use crate::execution::capability_resolver::{
    build_tool_advertisement, classify_mission, parse_tool_calls, resolve_profile_tools,
    summarize_capabilities,
};
use crate::execution::tools::context::ToolContext;
use crate::execution::{
    global_engine_registry, native_tool_definitions, ExecutionContext, ExecutionEvent,
    ExecutionManager, ExecutionRequest, MissionMetrics,
};
use crate::filesystem_tools::perf::{trace_perf_metric, PerfSpan};
use crate::models::{Message, MessageRole};
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
        let mut mission_metrics = MissionMetrics::start();
        mission_metrics.complete_failed("runtime_error", message.clone());
        let metrics_block = format!("\n\n---\n\n{}", mission_metrics.render_block());
        let _ = stream_persistence.enqueue_append(
            &job.pane_id,
            &job.assistant_message_id,
            metrics_block,
        );
        let _ = stream_persistence.enqueue_error(
            &job.pane_id,
            &job.assistant_message_id,
            "provider_execution_failed",
            message.clone(),
        );
        let _ = stream_persistence
            .drain_message(&job.assistant_message_id)
            .await;
        emit_stream_error(
            &app,
            &job.pane_id,
            &job.assistant_message_id,
            "provider_execution_failed",
            &message,
        );
    }
}

fn exec_ctx_to_tool_ctx(ctx: &ExecutionContext) -> ToolContext {
    ToolContext {
        execution_id: ctx.execution_id.clone(),
        pane_id: ctx.pane_id.clone(),
        project_root: ctx.project.as_ref().map(|p| p.approved_root.clone()),
        filesystem_scope: ctx.filesystem_scope.clone(),
        cwd: ctx
            .cwd
            .clone()
            .or_else(|| ctx.project.as_ref().map(|p| p.approved_root.clone())),
        environment: ctx.environment.clone(),
        cancellation: ctx.cancellation.clone(),
        timeout_ms: ctx.policy.timeout_ms,
        allow_shell: ctx.policy.allow_shell,
        allow_network: ctx.policy.allow_network,
        allow_read: ctx.policy.allow_read,
        allow_write: ctx.policy.allow_write,
        allow_delete: ctx.policy.allow_delete,
        allow_git: ctx.policy.allow_git,
        allow_packages: ctx.policy.allow_packages,
        allow_processes: ctx.policy.allow_processes,
    }
}

fn conversation_trace_value(conversation: &crate::models::Conversation) -> serde_json::Value {
    serde_json::json!({
        "id": conversation.id,
        "model": format!("{:?}", conversation.model),
        "messages": conversation.messages.iter().enumerate().map(|(index, message)| {
            serde_json::json!({
                "index": index,
                "role": format!("{:?}", message.role).to_ascii_lowercase(),
                "content": message.content,
                "tool_calls": [],
                "tool_call_id": null,
            })
        }).collect::<Vec<_>>(),
    })
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
    let mut mission_metrics = MissionMetrics::start();

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
        let enriched =
            tauri::async_runtime::spawn_blocking(move || enrich_conversation_with_filesystem(plan))
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

    let mut routing_context = ExecutionContext::from_pane_project(
        job.assistant_message_id.clone(),
        Some(job.pane_id.clone()),
        None,
        None,
        None,
    );
    routing_context.credential = Some(prepared.execution_context.credential.clone());
    routing_context.credential_service = credential_service;
    routing_context.policy = crate::execution::context::ExecutionPolicy {
        allow_shell: true,
        allow_network: true,
        allow_read: true,
        allow_write: true,
        allow_delete: true,
        allow_git: true,
        allow_packages: true,
        allow_processes: true,
        max_tokens: None,
        timeout_ms: None,
    };

    let exec_request = ExecutionRequest::chat(conversation.clone(), job.reasoning_level.clone());
    let mission = classify_mission(&conversation);

    let route_id = job.builder_id.as_deref().unwrap_or(&job.provider_id);
    let resolution = ExecutionManager::resolve_stream_route(
        route_id,
        &job.model_id,
        job.reasoning_level.as_deref(),
        &routing_context,
        &exec_request,
    );

    let engine_key = resolution.engine_id.clone();

    if !resolution.reason.is_empty() {
        trace_runtime_phase("execution_manager_decision", &resolution.reason);
    }
    crate::native_tool_trace::event(
        "execution_manager",
        serde_json::json!({
            "execution_id": job.assistant_message_id,
            "pane_id": job.pane_id,
            "builder": job.builder_id,
            "provider_id": job.provider_id,
            "account_id": job.account_id,
            "execution_class": format!("{:?}", resolution.class),
            "engine": resolution.engine_id,
            "model": resolution.model,
            "effort": resolution.effort,
            "reason": resolution.reason,
            "mission": mission.mission,
            "mission_class": mission.class.as_str(),
            "capability_profile": mission.profile.as_str(),
            "mission_reason": mission.reason,
        }),
    );

    let engine = global_engine_registry().get(&engine_key).ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "no execution engine registered for '{}'",
            engine_key
        ))
    })?;
    let use_native_tools = engine.supports_native_tools();

    // ===== Phase 9A.3/9A.4: Capability Resolution =====
    trace_runtime_phase("capability_resolution", "start");
    {
        let reg = crate::execution::tools::registry::global_tool_registry();
        let reg_lock = reg
            .read()
            .map_err(|e| StorageError::InvalidInput(format!("ToolRegistry lock error: {}", e)))?;
        let audit = crate::execution::capability_resolver::audit_capabilities(
            &routing_context.policy,
            &reg_lock,
        );
        trace_runtime_phase("capability_audit", &audit.summary());
        trace_runtime_phase(
            "capability_audit",
            &format!("allowed: {:?}", audit.allowed_tool_ids),
        );
        trace_runtime_phase(
            "capability_audit",
            &format!("blocked: {:?}", audit.blocked_tool_ids),
        );
        crate::native_tool_trace::event(
            "capability_resolver",
            serde_json::json!({
                "execution_id": job.assistant_message_id,
                "mission": mission.mission,
                "mission_class": mission.class.as_str(),
                "capability_profile": mission.profile.as_str(),
                "mission_reason": mission.reason,
                "summary": audit.summary(),
                "registered_count": audit.registered_count,
                "allowed_count": audit.allowed_count,
                "blocked_count": audit.blocked_count,
                "allowed_tools": audit.allowed_tool_ids,
                "blocked_tools": audit.blocked_tool_ids,
                "policy": {
                    "allow_shell": audit.policy.allow_shell,
                    "allow_network": audit.policy.allow_network,
                    "allow_read": audit.policy.allow_read,
                    "allow_write": audit.policy.allow_write,
                    "allow_delete": audit.policy.allow_delete,
                    "allow_git": audit.policy.allow_git,
                    "allow_packages": audit.policy.allow_packages,
                    "allow_processes": audit.policy.allow_processes,
                    "max_tokens": audit.policy.max_tokens,
                    "timeout_ms": audit.policy.timeout_ms,
                },
            }),
        );
    }
    let allowed_tools = {
        let reg = crate::execution::tools::registry::global_tool_registry();
        let reg_lock = reg
            .read()
            .map_err(|e| StorageError::InvalidInput(format!("ToolRegistry lock error: {}", e)))?;
        resolve_profile_tools(&routing_context.policy, &reg_lock, &mission.profile)
    };
    let tool_summary = summarize_capabilities(&allowed_tools);
    trace_runtime_phase(
        "capability_resolution",
        &format!("allowed: {}", tool_summary),
    );
    let has_tools = !allowed_tools.is_empty();
    let native_tools = if has_tools && use_native_tools {
        native_tool_definitions(&allowed_tools)
    } else {
        vec![]
    };
    let tool_ad_message = if has_tools && !use_native_tools {
        Some(build_tool_advertisement(&allowed_tools))
    } else {
        None
    };
    trace_runtime_phase(
        "tool_transport",
        if use_native_tools {
            "native"
        } else {
            "markdown"
        },
    );
    crate::native_tool_trace::event(
        "tool_advertisement",
        serde_json::json!({
            "execution_id": job.assistant_message_id,
            "mission": mission.mission,
            "execution_class": mission.class.as_str(),
            "capability_profile": mission.profile.as_str(),
            "mission_reason": mission.reason,
            "transport": if use_native_tools { "native" } else { "markdown" },
            "advertised_tool_count": allowed_tools.len(),
            "advertised_tools": allowed_tools.iter().map(|tool| tool.id().to_string()).collect::<Vec<_>>(),
            "native_function_names": native_tools.iter().map(|tool| serde_json::json!({
                "name": tool.name,
                "tool_id": tool.tool_id,
            })).collect::<Vec<_>>(),
        }),
    );
    mission_metrics.record_planning_complete();
    // ===== End Capability Resolution =====

    // ===== Phase 9A.3: Tool Call Loop =====
    trace_runtime_phase("tool_loop", "start");
    let mut current_conversation = conversation;
    let max_tool_rounds: u32 = 10;
    let mut final_text: Option<String> = None;

    for round in 0..max_tool_rounds {
        let round_number = round + 1;
        // Inject tool definitions on first round
        if let Some(ref ad) = tool_ad_message {
            if round == 0 {
                current_conversation = current_conversation
                    .with_message(Message::new(MessageRole::System, ad.clone()));
            }
        }

        let round_req = if use_native_tools {
            ExecutionRequest::chat_with_native_tools_for_round(
                current_conversation.clone(),
                job.reasoning_level.clone(),
                native_tools.clone(),
                round_number,
            )
        } else {
            ExecutionRequest::chat(current_conversation.clone(), job.reasoning_level.clone())
        };
        crate::native_tool_trace::event(
            "loop_round_start",
            serde_json::json!({
                "execution_id": job.assistant_message_id,
                "round": round_number,
                "conversation_length": current_conversation.messages.len(),
                "transport": if use_native_tools { "native" } else { "markdown" },
            }),
        );

        // Execute engine and collect events without streaming to frontend
        let llm_started = std::time::Instant::now();
        let (collected_text, native_tool_calls, exec_error) = {
            let (tx, mut rx) = mpsc::unbounded_channel::<ExecutionEvent>();
            let mut text_parts: Vec<String> = Vec::new();
            let mut tool_calls: Vec<(String, serde_json::Value)> = Vec::new();

            let round_engine = engine.clone();
            let round_ctx = routing_context.clone();

            let collector = tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        ExecutionEvent::TextDelta { content } => {
                            text_parts.push(content);
                        }
                        ExecutionEvent::ToolCallStarted {
                            call_id,
                            name,
                            arguments,
                        } => {
                            let parsed_arguments = arguments
                                .as_deref()
                                .and_then(|raw| serde_json::from_str(raw).ok())
                                .unwrap_or_else(|| serde_json::json!({}));
                            crate::native_tool_trace::event(
                                "native_tool_parsing",
                                serde_json::json!({
                                    "round": round_number,
                                    "raw_provider_payload": {
                                        "call_id": call_id,
                                        "name": name,
                                        "arguments": arguments,
                                    },
                                    "normalized": {
                                        "tool_name": name,
                                        "arguments": parsed_arguments,
                                    },
                                }),
                            );
                            tool_calls.push((name, parsed_arguments));
                        }
                        ExecutionEvent::Error { message, .. } => {
                            text_parts.push(format!("\n[Error: {}]", message));
                        }
                        _ => {}
                    }
                }
                (text_parts.join(""), tool_calls)
            });

            let result = round_engine
                .execute(
                    round_ctx,
                    round_req,
                    Box::new(move |event| {
                        let _ = tx.send(event);
                    }),
                )
                .await;

            let (text, tool_calls) = collector.await.unwrap_or_default();
            (text, tool_calls, result)
        };
        mission_metrics.record_llm_generation(llm_started.elapsed());

        if let Err(error) = exec_error {
            let message = format!("Engine execution error: {error:?}");
            mission_metrics.complete_failed("engine_execution_error", message.clone());
            trace_runtime_phase("tool_loop_error", &message);
            crate::native_tool_trace::event(
                "loop_round_error",
                serde_json::json!({
                    "execution_id": job.assistant_message_id,
                    "round": round_number,
                    "error": message,
                    "collected_text": collected_text,
                }),
            );
            final_text = Some(format!("{}\n\n{}", collected_text, message));
            break;
        }

        // Parse tool calls from the response
        let tool_calls = if use_native_tools {
            native_tool_calls
        } else {
            parse_tool_calls(&collected_text)
        };
        for (tool_name, _) in &tool_calls {
            mission_metrics.record_tool_call_detected(tool_name, round_number);
        }
        trace_runtime_phase(
            "tool_loop_round",
            &format!(
                "round {}: {} chars, {} tool call(s)",
                round,
                collected_text.len(),
                tool_calls.len()
            ),
        );
        crate::native_tool_trace::event(
            "loop_round_decision",
            serde_json::json!({
                "execution_id": job.assistant_message_id,
                "round": round_number,
                "tool_calls_parsed": tool_calls.len(),
                "conversation_length": current_conversation.messages.len(),
                "termination_reason": if tool_calls.is_empty() { "no_tool_calls" } else { "tool_calls_present" },
                "continue": !tool_calls.is_empty(),
                "last_assistant_response": collected_text,
                "parsed_tools": tool_calls.iter().map(|(name, arguments)| serde_json::json!({
                    "tool": name,
                    "arguments": arguments,
                })).collect::<Vec<_>>(),
            }),
        );

        if tool_calls.is_empty() {
            mission_metrics.complete_success();
            final_text = Some(collected_text);
            crate::native_tool_trace::write_conversation(
                round_number,
                &conversation_trace_value(&current_conversation),
            );
            break;
        }

        // Persist the assistant's response so the LLM sees its own reasoning + tool call in context
        current_conversation = current_conversation
            .with_message(Message::new(MessageRole::Assistant, &collected_text));

        // Execute each tool call and inject results
        let tool_registry = crate::execution::tools::registry::global_tool_registry();
        for (tool_name, arguments) in &tool_calls {
            let tool_display = format!("{} {:?}", tool_name, arguments);
            trace_runtime_phase("tool_invocation", &format!("calling: {}", tool_display));

            let result = {
                let reg = tool_registry
                    .read()
                    .map_err(|e| StorageError::InvalidInput(format!("ToolRegistry lock: {}", e)))?;

                let tool = match reg.get(tool_name) {
                    Some(t) => t,
                    None => {
                        let msg = format!("Unknown tool '{}'", tool_name);
                        mission_metrics.record_tool_failure();
                        trace_runtime_phase("tool_invocation_error", &msg);
                        crate::native_tool_trace::event(
                            "tool_registry_lookup",
                            serde_json::json!({
                                "execution_id": job.assistant_message_id,
                                "round": round_number,
                                "requested_tool": tool_name,
                                "resolved": false,
                                "error": msg,
                            }),
                        );
                        let result_text = format!(
                            "[Tool Result for '{}']\nStatus: ERROR\nError: unknown tool. Available tools are listed above.",
                            tool_name
                        );
                        current_conversation = current_conversation
                            .with_message(Message::new(MessageRole::User, result_text));
                        continue;
                    }
                };
                crate::native_tool_trace::event(
                    "tool_registry_lookup",
                    serde_json::json!({
                        "execution_id": job.assistant_message_id,
                        "round": round_number,
                        "requested_tool": tool_name,
                        "resolved": true,
                        "resolved_tool_id": tool.id().to_string(),
                        "arguments": arguments,
                    }),
                );

                let tool_ctx = exec_ctx_to_tool_ctx(&routing_context);

                if let Err(validation_error) = tool.validate(arguments) {
                    let msg = format!("Validation failed: {}", validation_error);
                    mission_metrics.record_tool_failure();
                    trace_runtime_phase("tool_validation_error", &msg);
                    crate::native_tool_trace::event(
                        "tool_validation",
                        serde_json::json!({
                            "execution_id": job.assistant_message_id,
                            "round": round_number,
                            "tool": tool_name,
                            "valid": false,
                            "error": validation_error,
                            "arguments": arguments,
                        }),
                    );
                    let result_text = format!(
                        "[Tool Result for '{}']\nStatus: VALIDATION_ERROR\nError: {}\nHint: Check the input schema and try again.",
                        tool_name, validation_error
                    );
                    current_conversation = current_conversation
                        .with_message(Message::new(MessageRole::User, result_text));
                    continue;
                }
                crate::native_tool_trace::event(
                    "tool_validation",
                    serde_json::json!({
                        "execution_id": job.assistant_message_id,
                        "round": round_number,
                        "tool": tool_name,
                        "valid": true,
                        "arguments": arguments,
                    }),
                );

                let tool_started = crate::native_tool_trace::now();
                let tool_metrics_started = std::time::Instant::now();
                crate::native_tool_trace::event(
                    "tool_execution_started",
                    serde_json::json!({
                        "execution_id": job.assistant_message_id,
                        "round": round_number,
                        "tool": tool_name,
                        "arguments": arguments,
                    }),
                );
                let exec_result = tool.execute(tool_ctx, arguments.clone(), &|event| match event {
                    ExecutionEvent::PermissionCheck {
                        permission,
                        allowed,
                        ..
                    } => {
                        let status = if allowed { "granted" } else { "denied" };
                        trace_runtime_phase(
                            "permission_check",
                            &format!("{}: {}", status, permission),
                        );
                        crate::native_tool_trace::event(
                            "tool_permission_check",
                            serde_json::json!({
                                "round": round_number,
                                "tool": tool_name,
                                "permission": permission,
                                "allowed": allowed,
                            }),
                        );
                    }
                    ExecutionEvent::ToolFinished { summary, .. } => {
                        if let Some(s) = summary {
                            trace_runtime_phase("tool_event", &format!("finished: {}", s));
                        }
                    }
                    ExecutionEvent::ToolFailed { code, message, .. } => {
                        trace_runtime_phase(
                            "tool_event",
                            &format!("failed [{}]: {}", code, message),
                        );
                    }
                    ExecutionEvent::TimelineEntry { phase, summary, .. } => {
                        trace_runtime_phase("tool_timeline", &format!("{}: {}", phase, summary));
                    }
                    ExecutionEvent::ReviewItemCreated {
                        action, summary, ..
                    } => {
                        trace_runtime_phase("tool_review", &format!("{}: {}", action, summary));
                    }
                    _ => {}
                });
                crate::native_tool_trace::event(
                    "tool_execution_completed",
                    serde_json::json!({
                        "execution_id": job.assistant_message_id,
                        "round": round_number,
                        "tool": tool_name,
                        "duration_ms": crate::native_tool_trace::elapsed_ms(tool_started),
                        "success": exec_result.as_ref().map(|result| result.success).unwrap_or(false),
                        "error": exec_result.as_ref().err().map(|error| error.to_string()),
                        "artifacts_produced": exec_result.as_ref().map(|result| result.artifacts.len()).unwrap_or(0),
                        "review_items": exec_result.as_ref().map(|result| result.review_items.len()).unwrap_or(0),
                        "return_value": exec_result.as_ref().ok().map(|result| serde_json::json!({
                            "success": result.success,
                            "exit_code": result.exit_code,
                            "stdout": result.output.stdout,
                            "stderr": result.output.stderr,
                            "summary": result.output.summary,
                        })),
                    }),
                );
                mission_metrics.record_tool_completion(
                    tool_metrics_started.elapsed(),
                    exec_result
                        .as_ref()
                        .map(|result| result.success)
                        .unwrap_or(false),
                );

                (tool_name.clone(), exec_result)
            };

            let status_label = if result.1.is_ok() && result.1.as_ref().unwrap().success {
                "SUCCESS"
            } else {
                "FAILED"
            };
            match result.1 {
                Ok(tool_result) => {
                    let stdout = if tool_result.output.stdout.is_empty() {
                        String::new()
                    } else {
                        format!("\n{}", tool_result.output.stdout)
                    };
                    let stderr = if tool_result.output.stderr.is_empty() {
                        String::new()
                    } else {
                        format!("\nstderr: {}", tool_result.output.stderr)
                    };
                    let result_text = format!(
                        "[Tool Result for '{}']\nStatus: {}\nExit code: {:?}{}{}",
                        result.0, status_label, tool_result.exit_code, stdout, stderr,
                    );
                    trace_runtime_phase("tool_result", &format!("{} completed", result.0));
                    current_conversation = current_conversation
                        .with_message(Message::new(MessageRole::User, result_text));
                    crate::native_tool_trace::event(
                        "tool_result_injected",
                        serde_json::json!({
                            "execution_id": job.assistant_message_id,
                            "round": round_number,
                            "tool": result.0,
                            "status": status_label,
                            "result_text": current_conversation.messages.last().map(|message| message.content.clone()),
                        }),
                    );
                }
                Err(e) => {
                    let result_text = format!(
                        "[Tool Result for '{}']\nStatus: ERROR\nError: {}",
                        result.0, e
                    );
                    trace_runtime_phase("tool_result", &format!("{} failed: {}", result.0, e));
                    current_conversation = current_conversation
                        .with_message(Message::new(MessageRole::User, result_text));
                    crate::native_tool_trace::event(
                        "tool_result_injected",
                        serde_json::json!({
                            "execution_id": job.assistant_message_id,
                            "round": round_number,
                            "tool": result.0,
                            "status": "ERROR",
                            "error": e,
                            "result_text": current_conversation.messages.last().map(|message| message.content.clone()),
                        }),
                    );
                }
            }
        }
        crate::native_tool_trace::write_conversation(
            round_number,
            &conversation_trace_value(&current_conversation),
        );
    }

    let final_response = if let Some(text) = final_text {
        text
    } else {
        let failure_message =
            "Maximum number of tool call rounds reached. Please refine your request.";
        mission_metrics.complete_failed("max_tool_rounds", failure_message);
        crate::native_tool_trace::event(
            "loop_max_rounds_reached",
            serde_json::json!({
                "execution_id": job.assistant_message_id,
                "max_tool_rounds": max_tool_rounds,
                "why": "every round produced at least one parsed tool call, so the controller never reached a no-tool final assistant response",
                "last_message": current_conversation.messages.last().map(|message| serde_json::json!({
                    "role": format!("{:?}", message.role).to_ascii_lowercase(),
                    "content": message.content,
                })),
            }),
        );
        failure_message.to_string()
    };
    let mission_metrics_summary = mission_metrics.summary();
    crate::native_tool_trace::event(
        "mission_metrics",
        serde_json::json!({
            "execution_id": job.assistant_message_id,
            "pane_id": job.pane_id,
            "metrics": mission_metrics_summary,
        }),
    );
    let final_response = format!(
        "{}\n\n---\n\n{}",
        final_response.trim_end(),
        mission_metrics.render_block()
    );
    trace_runtime_phase("tool_loop", "complete");
    // ===== End Tool Call Loop =====

    // ===== Stream final response to frontend =====
    let engine_span = PerfSpan::start("ENGINE_REQUEST_DURATION_MS");
    let engine_request_start = std::time::Instant::now();
    trace_runtime_phase("engine_stream", "start");

    let finish_write_buffer = StreamWriteBuffer::new(
        Arc::clone(stream_persistence),
        job.pane_id.clone(),
        job.assistant_message_id.clone(),
    );

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
                        trace_perf_metric(
                            "ENGINE_REQUEST_DURATION_MS",
                            engine_request_start.elapsed().as_millis(),
                        );
                        started = true;
                    }
                    if let Err(error) = write_buffer.push(&app_clone, &content) {
                        return Err(ProviderError::InvalidResponse {
                            message: error.to_string(),
                        });
                    }
                }
                ExecutionEvent::RunCompleted { .. } | ExecutionEvent::RunStarted { .. } => {}
                ExecutionEvent::Error { message, .. } => {
                    return Err(ProviderError::InvalidResponse { message });
                }
                _ => {}
            }
        }
        Ok(())
    });

    // Send the final response text as delta events
    for line in final_response.lines() {
        let chunk = format!("{}\n", line);
        let _ = event_tx.send(ExecutionEvent::TextDelta { content: chunk });
    }

    let _ = event_tx.send(ExecutionEvent::RunCompleted {
        execution_id: job.assistant_message_id.clone(),
        success: true,
        summary: None,
    });

    drop(event_tx);
    let _ = worker.await;

    finish_write_buffer.finish_with_complete(app).await?;

    trace_runtime_phase("engine_stream", "complete");
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
