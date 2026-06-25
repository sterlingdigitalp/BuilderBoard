use std::path::PathBuf;

use serde_json::Value;

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::helpers;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::{ToolArtifact, ToolOutput, ToolResult};
use crate::execution::tools::traits::{Tool, ToolId};

// ---------------------------------------------------------------------------
// GrepTool
// ---------------------------------------------------------------------------

pub struct GrepTool;

impl Tool for GrepTool {
    fn id(&self) -> ToolId {
        ToolId("search.grep")
    }
    fn display_name(&self) -> String {
        "Search Content".to_string()
    }
    fn description(&self) -> String {
        "Search file contents using regex patterns.".to_string()
    }
    fn category_name(&self) -> String {
        "filesystem".to_string()
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
        vec![ToolPermission::ReadFiles]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let pattern = args.get("pattern").and_then(|v| v.as_str());
        if pattern.is_none() || pattern.unwrap().is_empty() {
            return Err("Missing required argument: 'pattern'".to_string());
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_read, "read_files", &|e| on_event(e))?;

        let pattern = args["pattern"].as_str().unwrap();
        let path = args.get("path").and_then(|v| v.as_str());
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50);
        let fixed_string = args
            .get("fixed_string")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let context_lines = args.get("context").and_then(|v| v.as_u64()).unwrap_or(0);
        let include = args.get("include").and_then(|v| v.as_str());
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "search.grep".to_string(),
            execution_id: exec_id.clone(),
            args: format!("grep '{}'", pattern),
        });

        let search_root = resolve_search_root(&ctx, path)?;

        let (stdout, stderr, success) = run_search_command(
            &pattern,
            &search_root,
            max_results,
            fixed_string,
            context_lines,
            include,
        );

        let match_count = stdout.lines().count();
        let summary = format!("{} match(es) for '{}'", match_count, pattern);

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "search.grep".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: summary.clone(),
        });

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "search.grep".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(summary.clone()),
        });

        helpers::emit_timeline(
            &exec_id,
            "search.grep",
            if success { "completed" } else { "completed" },
            &summary,
            &|e| on_event(e),
        );

        let artifact = ToolArtifact {
            artifact_type: "search.results".to_string(),
            summary: summary.clone(),
            content: Some(stdout.clone()),
            path: Some(search_root.to_string_lossy().to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success,
            exit_code: Some(if success { 0 } else { 1 }),
            output: ToolOutput::new(stdout, stderr, summary),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

fn run_search_command(
    pattern: &str,
    root: &std::path::Path,
    max_results: u64,
    fixed_string: bool,
    context: u64,
    include: Option<&str>,
) -> (String, String, bool) {
    let mut use_rg = false;
    if let Ok(rg_check) = std::process::Command::new("rg").arg("--version").output() {
        use_rg = rg_check.status.success();
    }

    if use_rg {
        let mut cmd = std::process::Command::new("rg");
        cmd.args(["--no-heading", "--line-number"]);
        if fixed_string {
            cmd.arg("-F");
        }
        if context > 0 {
            cmd.args(["-C", &context.to_string()]);
        }
        cmd.arg("--max-count").arg(max_results.to_string());
        if let Some(ext) = include {
            cmd.args(["--glob", &format!("*.{}", ext)]);
        }
        cmd.arg(pattern);
        cmd.arg(root);

        if let Ok(output) = cmd.output() {
            return (
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
                output.status.success(),
            );
        }
    }

    let mut cmd = std::process::Command::new("grep");
    cmd.args(["-rn", "--color=never"]);
    if fixed_string {
        cmd.arg("-F");
    }
    if context > 0 {
        cmd.args(["-C", &context.to_string()]);
    }
    cmd.arg(pattern);
    cmd.arg(root.to_string_lossy().as_ref());

    if let Ok(output) = cmd.output() {
        (
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.success(),
        )
    } else {
        (String::new(), "grep not found".to_string(), false)
    }
}

// ---------------------------------------------------------------------------
// GlobTool
// ---------------------------------------------------------------------------

pub struct GlobTool;

impl Tool for GlobTool {
    fn id(&self) -> ToolId {
        ToolId("search.glob")
    }
    fn display_name(&self) -> String {
        "Find Files".to_string()
    }
    fn description(&self) -> String {
        "Find files matching a glob pattern.".to_string()
    }
    fn category_name(&self) -> String {
        "filesystem".to_string()
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
        vec![ToolPermission::ReadFiles]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let pattern = args.get("pattern").and_then(|v| v.as_str());
        if pattern.is_none() || pattern.unwrap().is_empty() {
            return Err("Missing required argument: 'pattern'".to_string());
        }
        Ok(())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_read, "read_files", &|e| on_event(e))?;

        let pattern = args["pattern"].as_str().unwrap();
        let path = args.get("path").and_then(|v| v.as_str());
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "search.glob".to_string(),
            execution_id: exec_id.clone(),
            args: format!("glob '{}'", pattern),
        });

        let search_root = resolve_search_root(&ctx, path)?;

        let matches = find_files_glob(&search_root, pattern, max_results);
        let matched = matches.len();
        let listing = matches.join("\n");
        let summary = format!("{} file(s) matching '{}'", matched, pattern);

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "search.glob".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: summary.clone(),
        });

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "search.glob".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(summary.clone()),
        });

        helpers::emit_timeline(&exec_id, "search.glob", "completed", &summary, &|e| {
            on_event(e)
        });

        let artifact = ToolArtifact {
            artifact_type: "search.results".to_string(),
            summary: summary.clone(),
            content: Some(listing),
            path: Some(search_root.to_string_lossy().to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(format!("{} file(s) found", matched), String::new(), summary),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

fn find_files_glob(root: &std::path::Path, pattern: &str, max_results: u64) -> Vec<String> {
    let mut use_fd = false;
    if let Ok(fd_check) = std::process::Command::new("fd").arg("--version").output() {
        use_fd = fd_check.status.success();
    }

    if use_fd {
        let mut cmd = std::process::Command::new("fd");
        cmd.args(["--type", "f", "--max-results", &max_results.to_string()]);
        cmd.arg(pattern);
        cmd.arg(root);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|l| l.to_string())
                    .collect();
            }
        }
    }

    let mut cmd = std::process::Command::new("find");
    cmd.arg(root.to_string_lossy().as_ref());
    cmd.args(["-type", "f", "-name", pattern]);
    cmd.arg("-maxdepth").arg("10");

    if let Ok(output) = cmd.output() {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .take(max_results as usize)
            .map(|l| l.to_string())
            .collect()
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_search_root(ctx: &ToolContext, path: Option<&str>) -> Result<PathBuf, String> {
    if let Some(p) = path {
        let pb = PathBuf::from(p);
        if pb.is_absolute() {
            if let Some(ref scope) = ctx.filesystem_scope {
                scope
                    .resolve_path(p)
                    .map_err(|_| format!("Path '{}' is outside the approved filesystem scope", p))
            } else {
                Ok(pb)
            }
        } else {
            let base = ctx
                .project_root
                .clone()
                .or_else(|| ctx.cwd.clone())
                .ok_or_else(|| {
                    "Relative path requires project_root or cwd in context".to_string()
                })?;
            let candidate = base.join(&pb);
            if let Some(ref scope) = ctx.filesystem_scope {
                let candidate_str = candidate.to_string_lossy();
                scope.resolve_path(&candidate_str).map_err(|_| {
                    format!(
                        "Path '{}' (resolved to '{}') is outside scope",
                        p,
                        candidate.display()
                    )
                })
            } else {
                Ok(candidate)
            }
        }
    } else {
        ctx.project_root
            .clone()
            .or_else(|| ctx.cwd.clone())
            .or_else(|| std::env::current_dir().ok())
            .ok_or_else(|| "No search root available".to_string())
    }
}
