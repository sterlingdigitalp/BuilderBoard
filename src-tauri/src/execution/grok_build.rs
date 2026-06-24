//! GrokBuildExecutionEngine
//!
//! Integrates the "grok" CLI (grok-build model) through the generalized
//! ExecutionEngine abstraction.
//!
//! Reuses CLIExecutionEngine for all process management, lifecycle,
//! cancellation, stdout streaming, etc.
//!
//! Command shape (per spec):
//!   grok -p "<formatted conversation>" \
//!        --model grok-build \
//!        --cwd <project approved root or cwd> \
//!        --output-format streaming-json \
//!        --always-approve
//!
//! Parses both simple NDJSON (type/data) and richer sessionUpdate events
//! emitted by the agent.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::execution::capabilities::{EngineCapabilities, Locality, SupportedFeatures, Transport};
use crate::execution::cli::{CLIProcessConfig, CLIExecutionEngine};
use crate::execution::context::ExecutionContext;
use crate::execution::engine::{ExecutionEngine, ExecutionError, ExecutionResult};
use crate::execution::event::ExecutionEvent;
use crate::execution::request::{ChatRequest, ExecutionRequest};
use crate::models::Conversation;

#[derive(Default)]
pub struct GrokBuildExecutionEngine {
    cli: CLIExecutionEngine,
}

impl GrokBuildExecutionEngine {
    pub fn new() -> Self {
        Self {
            cli: CLIExecutionEngine::new(),
        }
    }

    fn build_prompt(&self, chat: &ChatRequest) -> String {
        let conv = &chat.conversation;
        if conv.messages.is_empty() {
            return String::new();
        }

        conv.messages
            .iter()
            .map(|msg| {
                // Use role names compatible with LLM prompts
                let role = match &msg.role {
                    crate::models::MessageRole::System => "system",
                    crate::models::MessageRole::User => "user",
                    crate::models::MessageRole::Assistant => "assistant",
                    crate::models::MessageRole::Tool => "tool",
                };
                format!("{}: {}", role, msg.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn resolve_cwd(&self, context: &ExecutionContext) -> PathBuf {
        if let Some(cwd) = &context.cwd {
            return cwd.clone();
        }
        if let Some(proj) = &context.project {
            return proj.approved_root.clone();
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn parse_grok_event(&self, value: Value) -> Vec<ExecutionEvent> {
        let mut out = vec![];

        // Rich agent protocol (common for grok-build / agent mode)
        if let Some(su) = value.get("sessionUpdate").and_then(|v| v.as_str()) {
            match su {
                "agent_message_chunk" => {
                    if let Some(text) = value
                        .get("content")
                        .and_then(|c| c.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        if !text.is_empty() {
                            out.push(ExecutionEvent::TextDelta {
                                content: text.to_string(),
                            });
                        }
                    }
                }
                "agent_thought_chunk" => {
                    if let Some(text) = value
                        .get("content")
                        .and_then(|c| c.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        if !text.is_empty() {
                            out.push(ExecutionEvent::ReasoningDelta {
                                content: text.to_string(),
                            });
                        }
                    }
                }
                "tool_call" | "tool_call_started" => {
                    let name = value.get("tool").and_then(|v| v.as_str()).unwrap_or("tool").to_string();
                    out.push(ExecutionEvent::ToolCallStarted {
                        call_id: value.get("id").and_then(|v| v.as_str()).unwrap_or(&name).to_string(),
                        name,
                        arguments: value.get("arguments").map(|a| a.to_string()),
                    });
                }
                "tool_call_finished" | "tool_result" => {
                    out.push(ExecutionEvent::ToolCallFinished {
                        call_id: value.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        result: value.get("result").map(|r| r.to_string()),
                        error: value.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()),
                    });
                }
                _ => {
                    if su.contains("end") || su.contains("complete") {
                        out.push(ExecutionEvent::RunCompleted {
                            execution_id: "".to_string(), // filled by caller if needed
                            success: true,
                            summary: None,
                        });
                    }
                }
            }
            return out;
        }

        // Simple NDJSON format from --output-format streaming-json docs
        if let Some(typ) = value.get("type").and_then(|v| v.as_str()) {
            match typ {
                "text" => {
                    if let Some(data) = value.get("data").and_then(|v| v.as_str()) {
                        if !data.is_empty() {
                            out.push(ExecutionEvent::TextDelta {
                                content: data.to_string(),
                            });
                        }
                    }
                }
                "thought" => {
                    if let Some(data) = value.get("data").and_then(|v| v.as_str()) {
                        if !data.is_empty() {
                            out.push(ExecutionEvent::ReasoningDelta {
                                content: data.to_string(),
                            });
                        }
                    }
                }
                "end" => {
                    out.push(ExecutionEvent::RunCompleted {
                        execution_id: value
                            .get("sessionId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        success: true,
                        summary: None,
                    });
                }
                "error" => {
                    out.push(ExecutionEvent::Error {
                        code: "cli_error".to_string(),
                        message: value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown cli error")
                            .to_string(),
                    });
                }
                _ => {}
            }
        } else if value.get("text").is_some() {
            // plain json final?
            if let Some(t) = value.get("text").and_then(|v| v.as_str()) {
                out.push(ExecutionEvent::TextDelta {
                    content: t.to_string(),
                });
                out.push(ExecutionEvent::RunCompleted {
                    execution_id: "".into(),
                    success: true,
                    summary: None,
                });
            }
        }

        out
    }
}

impl ExecutionEngine for GrokBuildExecutionEngine {
    fn engine_id(&self) -> &'static str {
        "grok"
    }

    fn capabilities(&self) -> EngineCapabilities {
        let mut caps = EngineCapabilities::default();
        caps.engine_id = "grok".to_string();
        caps.locality = Locality::Hybrid; // local process, remote intelligence
        caps.supported_transports = vec![Transport::ProcessCli, Transport::Stdio];
        caps.features = SupportedFeatures {
            chat: true,
            streaming: true,
            reasoning: true,
            tool_use: true,
            images: false,
            embeddings: false,
            structured_output: true,
            multimodal: false,
            filesystem: true,
            shell: true,
            subagents: true,
            worktrees: true,
            cancellation: true,
        };
        caps.tags = vec!["grok".into(), "cli".into(), "xai".into()];
        caps.description = Some("Grok Build via local grok CLI (grok-build model)".into());
        caps
    }

    fn display_name(&self) -> String {
        "Grok".to_string()
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "grok-build".to_string(),
            "Composer 2.5 Fast".to_string(),
            "Grok Build".to_string(),
        ]
    }

    fn supported_effort_levels(&self) -> Vec<String> {
        vec!["low".to_string(), "medium".to_string(), "high".to_string(), "max".to_string()]
    }

    fn health(&self) -> String {
        // Check if grok CLI binary is available
        if std::process::Command::new("grok").arg("--version").output().is_ok() {
            "available".to_string()
        } else if std::path::Path::new("/Users/sterlingdigital/.grok/bin/grok").exists() {
            "available".to_string()
        } else {
            "cli missing".to_string()
        }
    }

    fn execute(
        &self,
        context: ExecutionContext,
        request: ExecutionRequest,
        on_event: Box<dyn Fn(ExecutionEvent) + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ExecutionResult> + Send>> {
        let chat_req = match request {
            ExecutionRequest::Chat(c) => c,
            other => {
                let kind = other.kind().to_string();
                return Box::pin(async move {
                    Err(ExecutionError::UnsupportedRequest { kind })
                });
            }
        };

        let prompt = self.build_prompt(&chat_req);
        if prompt.trim().is_empty() {
            on_event(ExecutionEvent::RunCompleted {
                execution_id: context.execution_id.clone(),
                success: true,
                summary: Some("empty prompt".into()),
            });
            return Box::pin(async { Ok(()) });
        }

        let cwd = self.resolve_cwd(&context);
        let execution_id = context.execution_id.clone();

        on_event(ExecutionEvent::RunStarted {
            execution_id: execution_id.clone(),
            engine_id: self.engine_id().to_string(),
            request_kind: "chat".to_string(),
        });

        // Wrap for sharing with CLI and parser
        let on_event: Arc<dyn Fn(ExecutionEvent) + Send + Sync> = Arc::new(on_event);

        let program = "grok".to_string();
        let args = vec![
            "-p".to_string(),
            prompt,
            "--model".to_string(),
            "grok-build".to_string(),
            "--cwd".to_string(),
            cwd.to_string_lossy().to_string(),
            "--output-format".to_string(),
            "streaming-json".to_string(),
            "--always-approve".to_string(),
            "--max-turns".to_string(),
            "8".to_string(), // reasonable default for this integration
            "--no-alt-screen".to_string(),
        ];

        let config = CLIProcessConfig {
            program,
            args,
            cwd: Some(cwd),
            env: HashMap::new(), // inherit
            timeout: Some(Duration::from_secs(300)), // 5 min safety
        };

        let parser = {
            let this = self.clone(); // cheap
            let exec_id = execution_id.clone();
            Box::new(move |v: serde_json::Value| -> Vec<ExecutionEvent> {
                let mut evs = this.parse_grok_event(v);
                for ev in &mut evs {
                    if let ExecutionEvent::RunCompleted { execution_id: ref mut eid, .. } = ev {
                        if eid.is_empty() {
                            *eid = exec_id.clone();
                        }
                    }
                }
                evs
            })
        };

        let cancel = context.cancellation.clone();
        let cli = self.cli.clone();
        Box::pin(async move {
            match cli
                .run_and_stream_events(config, parser, on_event, cancel)
                .await
            {
                Ok(_exit) => Ok(()),
                Err(e) => Err(e),
            }
        })
    }

    fn supports(&self, request: &ExecutionRequest) -> bool {
        matches!(request, ExecutionRequest::Chat(_))
    }
}

impl Clone for GrokBuildExecutionEngine {
    fn clone(&self) -> Self {
        Self {
            cli: self.cli.clone(),
        }
    }
}
