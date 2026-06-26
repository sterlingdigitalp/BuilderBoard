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
// ReadTool
// ---------------------------------------------------------------------------

pub struct ReadTool;

impl Tool for ReadTool {
    fn id(&self) -> ToolId {
        ToolId("filesystem.read")
    }
    fn display_name(&self) -> String {
        "Read File".to_string()
    }
    fn description(&self) -> String {
        "Read the contents of a file.".to_string()
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
            Err("Missing required argument: 'path'".to_string())
        } else {
            Ok(())
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
            tool_id: "filesystem.read".to_string(),
            execution_id: exec_id.clone(),
            args: path.to_string(),
        });

        let resolved = resolve_safe_path(&ctx, path)?;
        let content = std::fs::read_to_string(&resolved)
            .map_err(|e| format!("Failed to read '{}': {}", resolved.display(), e))?;

        on_event(ExecutionEvent::ToolOutput {
            tool_id: "filesystem.read".to_string(),
            execution_id: exec_id.clone(),
            channel: "result".to_string(),
            content: format!("{} bytes read", content.len()),
        });

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "filesystem.read".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!(
                "Read {} bytes from {}",
                content.len(),
                resolved.display()
            )),
        });

        helpers::emit_timeline(
            &exec_id,
            "filesystem.read",
            "completed",
            &format!("Read {} bytes from {}", content.len(), resolved.display()),
            &|e| on_event(e),
        );

        let content_len = content.len();
        let artifact = ToolArtifact {
            artifact_type: "file.content".to_string(),
            summary: format!("Read: {}", resolved.display()),
            content: Some(content.clone()),
            path: Some(resolved.to_string_lossy().to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                content,
                String::new(),
                format!("Read {} bytes", content_len),
            ),
            artifacts: vec![artifact],
            review_items: vec![],
        })
    }
}

// ---------------------------------------------------------------------------
// WriteTool
// ---------------------------------------------------------------------------

pub struct WriteTool;

impl Tool for WriteTool {
    fn id(&self) -> ToolId {
        ToolId("filesystem.write")
    }
    fn display_name(&self) -> String {
        "Write File".to_string()
    }
    fn description(&self) -> String {
        "Write content to a file. Creates parent directories if needed.".to_string()
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
        if args
            .get("path")
            .and_then(|v| v.as_str())
            .map_or(false, |s| !s.is_empty())
            && args.get("content").and_then(|v| v.as_str()).is_some()
        {
            return Ok(());
        }
        Err("Missing required arguments: 'path' (string) and 'content' (string)".to_string())
    }

    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String> {
        helpers::check_permission(&ctx, ctx.allow_write, "write_files", &|e| on_event(e))?;

        let path = args["path"].as_str().unwrap();
        let content = args["content"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "filesystem.write".to_string(),
            execution_id: exec_id.clone(),
            args: format!("{} ({} bytes)", path, content.len()),
        });

        let resolved = resolve_create_safe_path(&ctx, path)?;

        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create parent directories for '{}': {}",
                    resolved.display(),
                    e
                )
            })?;
        }

        std::fs::write(&resolved, content)
            .map_err(|e| format!("Failed to write '{}': {}", resolved.display(), e))?;

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "filesystem.write".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!(
                "Wrote {} bytes to {}",
                content.len(),
                resolved.display()
            )),
        });

        helpers::emit_timeline(
            &exec_id,
            "filesystem.write",
            "completed",
            &format!("Wrote {} bytes to {}", content.len(), resolved.display()),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "filesystem.write".to_string(),
            summary: format!("Wrote {} bytes to {}", content.len(), resolved.display()),
            details: Some(format!("path={}", resolved.display())),
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "filesystem.write".to_string(),
            execution_id: exec_id.clone(),
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "file.written".to_string(),
            summary: format!("Wrote: {}", resolved.display()),
            content: Some(content.to_string()),
            path: Some(resolved.to_string_lossy().to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                format!("{} bytes written to {}", content.len(), resolved.display()),
                String::new(),
                "File written successfully".to_string(),
            ),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// EditTool
// ---------------------------------------------------------------------------

pub struct EditTool;

impl Tool for EditTool {
    fn id(&self) -> ToolId {
        ToolId("filesystem.edit")
    }
    fn display_name(&self) -> String {
        "Edit File".to_string()
    }
    fn description(&self) -> String {
        "Perform an exact string replacement in a file.".to_string()
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
        vec![ToolPermission::ReadFiles, ToolPermission::WriteFiles]
    }

    fn validate(&self, args: &Value) -> Result<(), String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let old = args.get("old_string").and_then(|v| v.as_str());
        let new = args.get("new_string").and_then(|v| v.as_str());
        if path.is_none() {
            return Err("Missing 'path'".to_string());
        }
        if old.is_none() {
            return Err("Missing 'old_string'".to_string());
        }
        if new.is_none() {
            return Err("Missing 'new_string'".to_string());
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
        helpers::check_permission(&ctx, ctx.allow_write, "write_files", &|e| on_event(e))?;

        let path = args["path"].as_str().unwrap();
        let old_string = args["old_string"].as_str().unwrap();
        let new_string = args["new_string"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "filesystem.edit".to_string(),
            execution_id: exec_id.clone(),
            args: format!("replace in {}", path),
        });

        let resolved = resolve_safe_path(&ctx, path)?;
        let content = std::fs::read_to_string(&resolved)
            .map_err(|e| format!("Failed to read '{}': {}", resolved.display(), e))?;

        if !content.contains(old_string) {
            return Err(format!("'old_string' not found in {}", resolved.display()));
        }

        let new_content = content.replace(old_string, new_string);
        std::fs::write(&resolved, &new_content)
            .map_err(|e| format!("Failed to write '{}': {}", resolved.display(), e))?;

        let replacements = content.matches(old_string).count();
        let diff_summary = format!("{} replacement(s) made", replacements);

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "filesystem.edit".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Edited {}: {}", resolved.display(), diff_summary)),
        });

        helpers::emit_timeline(
            &exec_id,
            "filesystem.edit",
            "completed",
            &format!("Edited {}: {}", resolved.display(), diff_summary),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "filesystem.edit".to_string(),
            summary: format!("Edited {}: {}", resolved.display(), diff_summary),
            details: Some(format!("replaced {} occurrences", replacements)),
            severity: "info".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "filesystem.edit".to_string(),
            execution_id: exec_id.clone(),
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "file.edited".to_string(),
            summary: format!("Edited: {}", resolved.display()),
            content: Some(new_content),
            path: Some(resolved.to_string_lossy().to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                diff_summary,
                String::new(),
                "Edit applied successfully".to_string(),
            ),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// DeleteTool
// ---------------------------------------------------------------------------

pub struct DeleteTool;

impl Tool for DeleteTool {
    fn id(&self) -> ToolId {
        ToolId("filesystem.delete")
    }
    fn display_name(&self) -> String {
        "Delete File".to_string()
    }
    fn description(&self) -> String {
        "Delete a file or empty directory.".to_string()
    }
    fn category_name(&self) -> String {
        "filesystem".to_string()
    }

    fn supported_execution_classes(&self) -> Vec<ExecutionClass> {
        vec![
            ExecutionClass::Implementation,
            ExecutionClass::Debugging,
            ExecutionClass::General,
        ]
    }

    fn permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::DeleteFiles]
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
        helpers::check_permission(&ctx, ctx.allow_delete, "delete_files", &|e| on_event(e))?;

        let path = args["path"].as_str().unwrap();
        let exec_id = ctx.execution_id.clone();

        on_event(ExecutionEvent::ToolStarted {
            tool_id: "filesystem.delete".to_string(),
            execution_id: exec_id.clone(),
            args: path.to_string(),
        });

        let resolved = resolve_safe_path(&ctx, path)?;

        if resolved.is_dir() {
            std::fs::remove_dir(&resolved).map_err(|e| {
                format!("Failed to delete directory '{}': {}", resolved.display(), e)
            })?;
        } else {
            std::fs::remove_file(&resolved)
                .map_err(|e| format!("Failed to delete file '{}': {}", resolved.display(), e))?;
        }

        on_event(ExecutionEvent::ToolFinished {
            tool_id: "filesystem.delete".to_string(),
            execution_id: exec_id.clone(),
            summary: Some(format!("Deleted {}", resolved.display())),
        });

        helpers::emit_timeline(
            &exec_id,
            "filesystem.delete",
            "completed",
            &format!("Deleted {}", resolved.display()),
            &|e| on_event(e),
        );

        let review = ReviewItem {
            action: "filesystem.delete".to_string(),
            summary: format!("Deleted {}", resolved.display()),
            details: None,
            severity: "warning".to_string(),
        };

        on_event(ExecutionEvent::ReviewItemCreated {
            tool_id: "filesystem.delete".to_string(),
            execution_id: exec_id,
            action: review.action.clone(),
            summary: review.summary.clone(),
            details: review.details.clone(),
        });

        let artifact = ToolArtifact {
            artifact_type: "file.deleted".to_string(),
            summary: format!("Deleted: {}", resolved.display()),
            content: None,
            path: Some(resolved.to_string_lossy().to_string()),
            mime_type: None,
        };

        Ok(ToolResult {
            success: true,
            exit_code: Some(0),
            output: ToolOutput::new(
                format!("Deleted {}", resolved.display()),
                String::new(),
                "File deleted".to_string(),
            ),
            artifacts: vec![artifact],
            review_items: vec![review],
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_safe_path(ctx: &ToolContext, path_str: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path_str);

    if path.is_absolute() {
        if let Some(ref scope) = ctx.filesystem_scope {
            let path_str = path.to_string_lossy();
            scope.resolve_path(&path_str).map_err(|_| {
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
                let path_str = candidate.to_string_lossy();
                scope.resolve_path(&path_str).map_err(|_| {
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

fn resolve_create_safe_path(ctx: &ToolContext, path_str: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path_str);

    if path.is_absolute() {
        if let Some(ref scope) = ctx.filesystem_scope {
            scope.resolve_create_path(path_str).map_err(|_| {
                format!(
                    "Path '{}' is outside the approved filesystem scope",
                    path_str
                )
            })
        } else {
            Ok(path)
        }
    } else if let Some(ref root) = ctx.project_root {
        let candidate = root.join(&path);
        if let Some(ref scope) = ctx.filesystem_scope {
            let path_str = candidate.to_string_lossy();
            scope.resolve_create_path(&path_str).map_err(|_| {
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
