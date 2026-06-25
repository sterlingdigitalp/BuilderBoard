use std::process::Command;

use serde_json::Value;

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::helpers;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::{ReviewItem, ToolArtifact, ToolOutput, ToolResult};
use crate::execution::tools::traits::{Tool, ToolId};

fn run_git(
    ctx: &ToolContext,
    args: &[&str],
    exec_id: &str,
    on_event: &dyn Fn(ExecutionEvent),
) -> Result<(String, String, Option<i32>), String> {
    let cwd = ctx
        .cwd
        .as_ref()
        .or(ctx.project_root.as_ref())
        .ok_or_else(|| "No working directory set for git operation".to_string())?;

    let display_cmd = format!("git {}", args.join(" "));
    on_event(ExecutionEvent::ToolOutput {
        tool_id: "git".to_string(),
        execution_id: exec_id.to_string(),
        channel: "stdout".to_string(),
        content: format!("$ {}", display_cmd),
    });

    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stdout.is_empty() {
        on_event(ExecutionEvent::ToolOutput {
            tool_id: "git".to_string(),
            execution_id: exec_id.to_string(),
            channel: "stdout".to_string(),
            content: stdout.clone(),
        });
    }
    if !stderr.is_empty() {
        on_event(ExecutionEvent::ToolOutput {
            tool_id: "git".to_string(),
            execution_id: exec_id.to_string(),
            channel: "stderr".to_string(),
            content: stderr.clone(),
        });
    }

    let exit_code = output.status.code();
    if exit_code != Some(0) {
        return Err(format!(
            "Git {} failed (exit {:?}): {}",
            args[0], exit_code, stderr
        ));
    }

    Ok((stdout, stderr, exit_code))
}

// ---------------------------------------------------------------------------
// StatusTool
// ---------------------------------------------------------------------------

pub struct StatusTool;

impl Tool for StatusTool {
    fn id(&self) -> ToolId {
        ToolId("git.status")
    }
    fn display_name(&self) -> String {
        "Git Status".to_string()
    }
    fn description(&self) -> String {
        "Show the working tree status.".to_string()
    }
    fn category_name(&self) -> String {
        "git".to_string()
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
        vec![ToolPermission::Git]
    }

    fn validate(&self, _args: &Value) -> Result<(), String> {
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        _args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_git, "git", &|e| on_event(e))?;

        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "git.status".to_string(),
            execution_id: exec_id.clone(),
            args: "git status".to_string(),
        });

        let (stdout, stderr, _) = run_git(&ctx, &["status", "--porcelain"], &exec_id, on_event)?;

        let changed = stdout.lines().filter(|l| !l.is_empty()).count();
        let summary = if changed == 0 {
            "Working tree clean".to_string()
        } else {
            format!("{} file(s) changed", changed)
        };

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "git.status".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(summary.clone()),
        });

        helpers::emit_timeline(&exec_id, "git.status", "completed", &summary, &|e| {
            on_event(e)
        });

        let artifact = ToolArtifact {
            artifact_type: "git.status".to_string(),
            summary: summary.clone(),
            content: Some(stdout.clone()),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(stdout, stderr, summary),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// DiffTool
// ---------------------------------------------------------------------------

pub struct DiffTool;

impl Tool for DiffTool {
    fn id(&self) -> ToolId {
        ToolId("git.diff")
    }
    fn display_name(&self) -> String {
        "Git Diff".to_string()
    }
    fn description(&self) -> String {
        "Show changes in the working tree or between commits.".to_string()
    }
    fn category_name(&self) -> String {
        "git".to_string()
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
        vec![ToolPermission::Git]
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
        helpers::check_permission(&ctx, ctx.allow_git, "git", &|e| on_event(e))?;

        let exec_id = ctx.execution_id.clone();

        let staged = args
            .get("staged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "git.diff".to_string(),
            execution_id: exec_id.clone(),
            args: if staged {
                "git diff --staged".to_string()
            } else {
                format!("git diff {}", path)
            },
        });

        // Build git args properly: git diff [--staged] [-- <path>]
        let mut git_args = vec!["diff"];
        if staged {
            git_args.push("--staged");
        }
        git_args.push("--");
        git_args.push(path);

        let (stdout, stderr, _) = run_git(&ctx, &git_args, &exec_id, on_event)?;

        let line_count = stdout.lines().count();
        let summary = format!("{} lines of diff", line_count);

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "git.diff".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(summary.clone()),
        });

        helpers::emit_timeline(&exec_id, "git.diff", "completed", &summary, &|e| {
            on_event(e)
        });

        let artifact = ToolArtifact {
            artifact_type: "git.diff".to_string(),
            summary: summary.clone(),
            content: Some(stdout.clone()),
            path: None,
            mime_type: Some("text/x-diff".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(stdout, stderr, summary),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// CommitTool
// ---------------------------------------------------------------------------

pub struct CommitTool;

impl Tool for CommitTool {
    fn id(&self) -> ToolId {
        ToolId("git.commit")
    }
    fn display_name(&self) -> String {
        "Git Commit".to_string()
    }
    fn description(&self) -> String {
        "Stage and commit changes.".to_string()
    }
    fn category_name(&self) -> String {
        "git".to_string()
    }

    fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
        vec![
            ExecutionClass::Implementation,
            ExecutionClass::Debugging,
            ExecutionClass::General,
        ]
    }

    fn permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::Git]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let msg = args.get("message").and_then(|v| v.as_str());
        if msg.is_none() || msg.unwrap().is_empty() {
            return Err("Missing required argument: 'message'".to_string());
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_git, "git", &|e| on_event(e))?;

        let message = args["message"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "git.commit".to_string(),
            execution_id: exec_id.clone(),
            args: format!("commit: {}", message),
        });

        run_git(&ctx, &["add", "-A"], &exec_id, on_event)?;

        let (stdout, stderr, _) = run_git(&ctx, &["commit", "-m", message], &exec_id, on_event)?;

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "git.commit".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Committed: {}", message)),
        });

        helpers::emit_timeline(
            &exec_id,
            "git.commit",
            "completed",
            &format!("Committed: {}", message),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "git.commit".to_string(),
            summary: format!("Git commit: {}", message),
            details: Some(stdout.clone()),
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "git.commit".to_string(),
            execution_id: exec_id,
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let stdout_clone = stdout.clone();
        let artifact = ToolArtifact {
            artifact_type: "git.commit".to_string(),
            summary: format!("Commit: {}", message),
            content: Some(stdout),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(stdout_clone, stderr, format!("Committed: {}", message)),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// LogTool
// ---------------------------------------------------------------------------

pub struct LogTool;

impl Tool for LogTool {
    fn id(&self) -> ToolId {
        ToolId("git.log")
    }
    fn display_name(&self) -> String {
        "Git Log".to_string()
    }
    fn description(&self) -> String {
        "Show commit history.".to_string()
    }
    fn category_name(&self) -> String {
        "git".to_string()
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
        vec![ToolPermission::Git]
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
        helpers::check_permission(&ctx, ctx.allow_git, "git", &|e| on_event(e))?;

        let exec_id = ctx.execution_id.clone();
        let max_count = args.get("max_count").and_then(|v| v.as_u64()).unwrap_or(10);

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "git.log".to_string(),
            execution_id: exec_id.clone(),
            args: format!("git log -{}", max_count),
        });

        let (stdout, stderr, _) = run_git(
            &ctx,
            &[
                "log",
                &format!("-{}", max_count),
                "--oneline",
                "--decorate",
                "--graph",
            ],
            &exec_id,
            on_event,
        )?;

        let commit_count = stdout.lines().filter(|l| !l.is_empty()).count();
        let summary = format!("{} commit(s)", commit_count);

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "git.log".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(summary.clone()),
        });

        helpers::emit_timeline(&exec_id, "git.log", "completed", &summary, &|e| on_event(e));

        let artifact = ToolArtifact {
            artifact_type: "git.log".to_string(),
            summary: summary.clone(),
            content: Some(stdout.clone()),
            path: None,
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(stdout, stderr, summary),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}
