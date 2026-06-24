//! CLIExecutionEngine - reusable infrastructure for all CLI-native execution engines.
//!
//! Owns:
//! - process creation & lifecycle (tokio::process::Command)
//! - stdout / stderr streaming
//! - cwd, environment
//! - cancellation (via AtomicBool)
//! - exit status, timeout
//! - line-by-line NDJSON reading + hook for structured event parsing
//!
//! Knows NOTHING about Grok, OpenAI, or specific protocols.
//! Used by GrokBuildExecutionEngine and future CLI engines (Claude Code, LM Studio, Ollama CLI, etc.).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

use crate::execution::event::ExecutionEvent;
use crate::execution::engine::ExecutionError;

/// Configuration for launching a CLI process.
#[derive(Clone, Debug)]
pub struct CLIProcessConfig {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<std::path::PathBuf>,
    pub env: HashMap<String, String>,
    pub timeout: Option<Duration>,
}

/// Reusable CLI execution helper.
/// Does not implement ExecutionEngine itself — concrete engines (GrokBuild etc.)
/// implement ExecutionEngine and delegate process management here.
#[derive(Default, Clone)]
pub struct CLIExecutionEngine;

impl CLIExecutionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Run a CLI process, stream its stdout lines (expected NDJSON), parse via hook,
    /// emit ExecutionEvents, handle cancellation/timeout/cleanup.
    ///
    /// Returns the exit code on success.
    pub async fn run_and_stream_events(
        &self,
        config: CLIProcessConfig,
        event_parser: Box<dyn Fn(serde_json::Value) -> Vec<ExecutionEvent> + Send + Sync>,
        on_event: Arc<dyn Fn(ExecutionEvent) + Send + Sync>,
        cancellation: Option<Arc<AtomicBool>>,
    ) -> Result<i32, ExecutionError> {
        let mut cmd = Command::new(&config.program);
        cmd.args(&config.args);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
        }

        // Merge/override env (inherit by default, override provided)
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let mut child: Child = cmd
            .spawn()
            .map_err(|e| ExecutionError::Internal {
                message: format!("failed to spawn {}: {}", config.program, e),
            })?;

        let stdout = child.stdout.take().ok_or_else(|| ExecutionError::Internal {
            message: "failed to capture stdout".to_string(),
        })?;

        let stderr = child.stderr.take();

        let mut reader = BufReader::new(stdout).lines();

        // Spawn stderr reader (non-blocking, emit warnings)
        if let Some(stderr) = stderr {
            let on_event_clone = Arc::clone(&on_event);
            tokio::spawn(async move {
                let mut err_reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = err_reader.next_line().await {
                    if !line.trim().is_empty() {
                        on_event_clone(ExecutionEvent::Warning {
                            message: format!("stderr: {}", line),
                        });
                    }
                }
            });
        }

        // Cancellation watcher
        let cancel_flag = cancellation.clone();

        // Main read loop with cancellation check
        let read_future = async {
            while let Ok(Some(line)) = reader.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                // Check cancellation
                if let Some(flag) = &cancel_flag {
                    if flag.load(Ordering::SeqCst) {
                        let _ = child.kill().await;
                        on_event(ExecutionEvent::Cancelled { reason: Some("user cancelled".into()) });
                        break;
                    }
                }

                // Try parse as JSON
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    for ev in event_parser(value) {
                        on_event(ev);
                    }
                } else {
                    // Non-JSON line: treat as text delta (fallback)
                    on_event(ExecutionEvent::TextDelta { content: line });
                }
            }

            // Wait for exit
            let status = child.wait().await.map_err(|e| ExecutionError::Internal {
                message: format!("wait failed: {}", e),
            })?;

            Ok::<_, ExecutionError>(status.code().unwrap_or(-1))
        };

        // Apply timeout if configured
        let result = if let Some(dur) = config.timeout {
            match timeout(dur, read_future).await {
                Ok(r) => r,
                Err(_) => {
                    let _ = child.kill().await;
                    on_event(ExecutionEvent::Error {
                        code: "timeout".to_string(),
                        message: "CLI process timed out".to_string(),
                    });
                    Err(ExecutionError::Internal {
                        message: "timeout".to_string(),
                    })
                }
            }
        } else {
            read_future.await
        };

        // Final cleanup
        if let Some(flag) = &cancel_flag {
            if flag.load(Ordering::SeqCst) {
                let _ = child.kill().await;
            }
        }

        result
    }
}
