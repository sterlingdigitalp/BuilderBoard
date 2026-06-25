use std::process::Command;

use serde_json::Value;

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::helpers;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::{ReviewItem, ToolArtifact, ToolOutput, ToolResult};
use crate::execution::tools::traits::{Tool, ToolId};

// ---------------------------------------------------------------------------
// ListTool
// ---------------------------------------------------------------------------

pub struct ListTool;

impl Tool for ListTool {
    fn id(&self) -> ToolId {
        ToolId("process.list")
    }
    fn display_name(&self) -> String {
        "List Processes".to_string()
    }
    fn description(&self) -> String {
        "List running processes.".to_string()
    }
    fn category_name(&self) -> String {
        "system".to_string()
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
        vec![ToolPermission::Processes]
    }

    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_processes, "processes", &|e| on_event(e))?;

        let exec_id = ctx.execution_id.clone();
        let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("");

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "process.list".to_string(),
            execution_id: exec_id.clone(),
            args: if filter.is_empty() {
                "all processes".to_string()
            } else {
                format!("filter: {}", filter)
            },
        });

        let output = if cfg!(target_os = "windows") {
            Command::new("tasklist")
                .output()
                .map_err(|e| format!("Failed to list processes: {}", e))?
        } else {
            let mut cmd = Command::new("ps");
            cmd.args(["aux"]);
            cmd.output()
                .map_err(|e| format!("Failed to list processes: {}", e))?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let filtered: String = if filter.is_empty() {
            stdout.clone()
        } else {
            stdout
                .lines()
                .filter(|l| l.to_lowercase().contains(&filter.to_lowercase()))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let filtered_count = filtered.lines().count();

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "process.list".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: format!("{} process(es)", filtered_count),
        });

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "process.list".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Listed {} process(es)", filtered_count)),
        });

        helpers::emit_timeline(
            &exec_id,
            "process.list",
            "completed",
            &format!("Listed {} process(es)", filtered_count),
            &|e| on_event(e),
        );

        let artifact = ToolArtifact {
            artifact_type: "process.list".to_string(),
            summary: format!(
                "Process listing (filter: {})",
                if filter.is_empty() { "all" } else { filter }
            ),
            content: Some(filtered),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: output.status.code(),
            output: ToolOutput::new(stdout, stderr, format!("{} process(es)", filtered_count)),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// KillTool
// ---------------------------------------------------------------------------

pub struct KillTool;

impl Tool for KillTool {
    fn id(&self) -> ToolId {
        ToolId("process.kill")
    }
    fn display_name(&self) -> String {
        "Kill Process".to_string()
    }
    fn description(&self) -> String {
        "Terminate a running process by PID.".to_string()
    }
    fn category_name(&self) -> String {
        "system".to_string()
    }

    fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
        vec![
            ExecutionClass::Implementation,
            ExecutionClass::Debugging,
            ExecutionClass::General,
        ]
    }

    fn permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::Processes]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let pid = args.get("pid").and_then(|v| v.as_u64());
        if pid.is_none() {
            return Err("Missing required argument: 'pid'".to_string());
        }
        let signal = args.get("signal").and_then(|v| v.as_str());
        if let Some(sig) = signal {
            let valid = [
                "SIGTERM", "SIGKILL", "SIGINT", "SIGHUP", "SIGSTOP", "SIGCONT",
            ];
            if !valid.contains(&sig) {
                return Err(format!("Invalid signal '{}'. Valid: {:?}", sig, valid));
            }
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_processes, "processes", &|e| on_event(e))?;

        let pid = args["pid"].as_u64().unwrap();
        let signal = args
            .get("signal")
            .and_then(|v| v.as_str())
            .unwrap_or("SIGTERM");
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "process.kill".to_string(),
            execution_id: exec_id.clone(),
            args: format!("kill {} ({})", pid, signal),
        });

        let output = if cfg!(target_os = "windows") {
            Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output()
                .map_err(|e| format!("Failed to kill process: {}", e))?
        } else {
            Command::new("kill")
                .args(["-s", signal, &pid.to_string()])
                .output()
                .map_err(|e| format!("Failed to kill process: {}", e))?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            on_event(ExecutionEvent::ToolFailed {
                tool_id: "process.kill".to_string(),
                execution_id: exec_id.clone(),
                code: output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                message: stderr.clone(),
            });
            helpers::emit_timeline(
                &exec_id,
                "process.kill",
                "failed",
                &format!("Failed to kill process {}", pid),
                &|e| on_event(e),
            );
            return Err(format!("Failed to kill process {}: {}", pid, stderr));
        }

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "process.kill".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Killed process {} with {}", pid, signal)),
        });

        helpers::emit_timeline(
            &exec_id,
            "process.kill",
            "completed",
            &format!("Killed process {} with {}", pid, signal),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "process.kill".to_string(),
            summary: format!("Killed process {} with {}", pid, signal),
            details: None,
            severity: "warning".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "process.kill".to_string(),
            execution_id: exec_id,
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let stdout_clone = stdout.clone();
        let artifact = ToolArtifact {
            artifact_type: "process.killed".to_string(),
            summary: format!("Killed process {}", pid),
            content: Some(stdout),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: output.status.code(),
            output: ToolOutput::new(stdout_clone, stderr, format!("Killed process {}", pid)),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}
