use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::helpers;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::{ReviewItem, ToolArtifact, ToolOutput, ToolResult};
use crate::execution::tools::traits::{Tool, ToolId};

pub struct ShellTool;

impl Tool for ShellTool {
    fn id(&self) -> ToolId {
        ToolId("shell")
    }

    fn display_name(&self) -> String {
        "Shell".to_string()
    }

    fn description(&self) -> String {
        "Execute shell commands with real-time streaming of stdout/stderr.".to_string()
    }

    fn category_name(&self) -> String {
        "execution".to_string()
    }

    fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
        vec![
            ExecutionClass::Implementation,
            ExecutionClass::Debugging,
            ExecutionClass::Testing,
            ExecutionClass::General,
        ]
    }

    fn permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::Shell]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required argument: 'command'".to_string())?;
        if cmd.trim().is_empty() {
            return Err("'command' must not be empty".to_string());
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_shell, "shell", &|e| on_event(e))?;

        let command = args["command"].as_str().unwrap_or_default().to_string();
        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let timeout = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .or(ctx.timeout_ms);

        let execution_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "shell".to_string(),
            execution_id: execution_id.clone(),
            args: command.clone(),
        });

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        } else if let Some(ref cwd) = ctx.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let stdout_handle = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;
        let stderr_handle = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr".to_string())?;

        let (output_tx, output_rx) = mpsc::channel::<(&'static str, String)>();

        // Thread 1: read stdout
        let tx1 = output_tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout_handle);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if tx1.send(("stdout", line)).is_err() {
                        break;
                    }
                }
            }
        });

        // Thread 2: read stderr
        let tx2 = output_tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr_handle);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if tx2.send(("stderr", line)).is_err() {
                        break;
                    }
                }
            }
        });
        drop(output_tx);

        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();
        let deadline = timeout.map(|ms| std::time::Instant::now() + Duration::from_millis(ms));
        let mut timed_out = false;

        loop {
            if ctx.is_cancelled() {
                let _ = child.kill();
                on_event(ExecutionEvent::ToolFailed {
                    tool_id: "shell".to_string(),
                    execution_id: execution_id.clone(),
                    code: "CANCELLED".to_string(),
                    message: "Shell execution was cancelled".to_string(),
                });
                helpers::emit_timeline(
                    &execution_id,
                    "shell",
                    "cancelled",
                    "Shell execution cancelled",
                    &|e| on_event(e),
                );
                return Err("Shell execution was cancelled".to_string());
            }

            let remaining =
                deadline.map(|d| d.saturating_duration_since(std::time::Instant::now()));
            if remaining.map_or(false, |r| r.is_zero()) {
                timed_out = true;
                let _ = child.kill();
                break;
            }

            let timeout_dur = remaining.unwrap_or(Duration::from_millis(200));

            match output_rx.recv_timeout(timeout_dur) {
                Ok(("stdout", line)) => {
                    stdout_buf.push_str(&line);
                    stdout_buf.push('\n');
                    on_event(ExecutionEvent::ToolOutput {
                        tool_id: "shell".to_string(),
                        execution_id: execution_id.clone(),
                        channel: "stdout".to_string(),
                        content: line,
                    });
                }
                Ok(("stderr", line)) => {
                    stderr_buf.push_str(&line);
                    stderr_buf.push('\n');
                    on_event(ExecutionEvent::ToolOutput {
                        tool_id: "shell".to_string(),
                        execution_id: execution_id.clone(),
                        channel: "stderr".to_string(),
                        content: line,
                    });
                }
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if deadline.is_some() {
                        let now = std::time::Instant::now();
                        if now >= deadline.unwrap() {
                            timed_out = true;
                            let _ = child.kill();
                            break;
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        let status = child.wait().ok();
        let exit_code = status.and_then(|s| s.code());

        if timed_out {
            on_event(ExecutionEvent::ToolFailed {
                tool_id: "shell".to_string(),
                execution_id: execution_id.clone(),
                code: "TIMEOUT".to_string(),
                message: "Shell command timed out".to_string(),
            });
            helpers::emit_timeline(
                &execution_id,
                "shell",
                "failed",
                "Shell command timed out",
                &|e| on_event(e),
            );
            return Err("Shell command timed out".to_string());
        }

        if exit_code == Some(0) {
            let summary = "Shell command completed with exit code 0".to_string();
            on_event(ExecutionEvent::ToolFinished {
                tool_id: "shell".to_string(),
                execution_id: execution_id.clone(),
                summary: Some(summary.clone()),
            });
            helpers::emit_timeline(&execution_id, "shell", "completed", &summary, &|e| {
                on_event(e)
            });
        } else {
            let msg = format!("Shell command failed with exit code {:?}", exit_code);
            on_event(ExecutionEvent::ToolFailed {
                tool_id: "shell".to_string(),
                execution_id: execution_id.clone(),
                code: exit_code.map(|c| c.to_string()).unwrap_or_default(),
                message: msg.clone(),
            });
            helpers::emit_timeline(&execution_id, "shell", "failed", &msg, &|e| on_event(e));
        }

        let review_item = ReviewItem {
            action: "shell.exec".to_string(),
            summary: format!("Shell command: {}", &command[..command.len().min(100)]),
            details: Some(format!(
                "exit_code={:?}, stdout={} bytes, stderr={} bytes",
                exit_code,
                stdout_buf.len(),
                stderr_buf.len()
            )),
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "shell".to_string(),
            execution_id: execution_id.clone(),
            action: review_item.action.clone(),
            summary: review_item.summary.clone(),
            details: review_item.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "shell.transcript".to_string(),
            summary: format!("Shell command: {}", &command[..command.len().min(80)]),
            content: Some(format!(
                "$ {}\n\nstdout:\n{}\nstderr:\n{}",
                command, stdout_buf, stderr_buf
            )),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: exit_code == Some(0),
            exit_code,
            output: ToolOutput::new(
                stdout_buf,
                stderr_buf,
                format!("Exit code: {:?}", exit_code),
            ),
            artifacts: vec![artifact],
            review_items: vec![review_item],
        })
    }
}
