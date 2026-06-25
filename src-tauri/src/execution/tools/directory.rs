use std::path::PathBuf;

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
        ToolId("directory.list")
    }
    fn display_name(&self) -> String {
        "List Directory".to_string()
    }
    fn description(&self) -> String {
        "List entries in a directory.".to_string()
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
        if !args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
        {
            Ok(())
        } else {
            Err("Missing required argument: 'path'".to_string())
        }
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_read, "read_files", &|e| on_event(e))?;

        let path = args["path"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "directory.list".to_string(),
            execution_id: exec_id.clone(),
            args: path.to_string(),
        });

        let resolved = resolve_dir_path(&ctx, path)?;
        let entries = std::fs::read_dir(&resolved)
            .map_err(|e| format!("Failed to list '{}': {}", resolved.display(), e))?;

        let mut files = Vec::new();
        let mut dirs = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                dirs.push(name);
            } else {
                files.push(name);
            }
        }

        files.sort();
        dirs.sort();

        let mut listing = String::new();
        listing.push_str(&format!(
            "{} entries in {}:\n",
            files.len() + dirs.len(),
            resolved.display()
        ));
        for d in &dirs {
            listing.push_str(&format!("  dir {}\n", d));
        }
        for f in &files {
            listing.push_str(&format!("  file {}\n", f));
        }

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "directory.list".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: listing.clone(),
        });

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "directory.list".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!(
                "Listed {} entries in {}",
                files.len() + dirs.len(),
                resolved.display()
            )),
        });

        helpers::emit_timeline(
            &exec_id,
            "directory.list",
            "completed",
            &format!(
                "Listed {} entries in {}",
                files.len() + dirs.len(),
                resolved.display()
            ),
            &|e| on_event(e),
        );

        let artifact = ToolArtifact {
            artifact_type: "directory.listing".to_string(),
            summary: format!("Directory listing: {}", resolved.display()),
            content: Some(listing),
            path: Some(resolved.to_string_lossy().to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                format!("{} files, {} directories", files.len(), dirs.len()),
                String::new(),
                format!("Listed {} entries", files.len() + dirs.len()),
            ),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// CreateTool
// ---------------------------------------------------------------------------

pub struct CreateTool;

impl Tool for CreateTool {
    fn id(&self) -> ToolId {
        ToolId("directory.create")
    }
    fn display_name(&self) -> String {
        "Create Directory".to_string()
    }
    fn description(&self) -> String {
        "Create a directory and all parent directories.".to_string()
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
        vec![ToolPermission::WriteFiles]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        if !args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
        {
            Ok(())
        } else {
            Err("Missing required argument: 'path'".to_string())
        }
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_write, "write_files", &|e| on_event(e))?;

        let path = args["path"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "directory.create".to_string(),
            execution_id: exec_id.clone(),
            args: path.to_string(),
        });

        let resolved = resolve_dir_path(&ctx, path)?;

        std::fs::create_dir_all(&resolved)
            .map_err(|e| format!("Failed to create directory '{}': {}", resolved.display(), e))?;

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "directory.create".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Created directory {}", resolved.display())),
        });

        helpers::emit_timeline(
            &exec_id,
            "directory.create",
            "completed",
            &format!("Created directory {}", resolved.display()),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "directory.create".to_string(),
            summary: format!("Created directory {}", resolved.display()),
            details: None,
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "directory.create".to_string(),
            execution_id: exec_id,
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "directory.created".to_string(),
            summary: format!("Created directory: {}", resolved.display()),
            content: None,
            path: Some(resolved.to_string_lossy().to_string()),
            mime_type: None,
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                format!("Created {}", resolved.display()),
                String::new(),
                "Directory created".to_string(),
            ),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_dir_path(ctx: &ToolContext, path_str: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path_str);
    if path.is_absolute() {
        if let Some(ref scope) = ctx.filesystem_scope {
            scope.resolve_path(path_str).map_err(|_| {
                format!(
                    "Path '{}' is outside the approved filesystem scope",
                    path_str
                )
            })
        } else {
            Ok(path)
        }
    } else {
        if let Some(ref root) = ctx.project_root {
            let candidate = root.join(&path);
            if let Some(ref scope) = ctx.filesystem_scope {
                let candidate_str = candidate.to_string_lossy();
                scope.resolve_path(&candidate_str).map_err(|_| {
                    format!(
                        "Path '{}' (resolved to '{}') is outside the approved filesystem scope",
                        path_str,
                        candidate.display()
                    )
                })
            } else {
                Ok(candidate)
            }
        } else {
            Err("Relative paths require a project_root or cwd in context".to_string())
        }
    }
}
