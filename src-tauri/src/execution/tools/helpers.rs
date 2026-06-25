//! Shared helpers for permission enforcement, timeline, and standardized events.
//!
//! Reduces boilerplate across all tool implementations.
//! Not a redesign — pure convenience functions.

use crate::execution::event::ExecutionEvent;
use crate::execution::tools::context::ToolContext;

/// Check a permission boolean. If denied, emit PermissionCheck + return Err.
/// If allowed, emit PermissionCheck and continue.
pub fn check_permission(
    ctx: &ToolContext,
    allowed: bool,
    permission_name: &str,
    on_event: &dyn Fn(ExecutionEvent),
) -> Result<(), String> {
    if allowed {
        on_event(ExecutionEvent::PermissionCheck {
            tool_id: String::new(), // caller fills this in
            permission: permission_name.to_string(),
            allowed: true,
            reason: None,
        });
        Ok(())
    } else {
        on_event(ExecutionEvent::PermissionCheck {
            tool_id: String::new(),
            permission: permission_name.to_string(),
            allowed: false,
            reason: Some("Denied by execution policy".to_string()),
        });
        Err(format!(
            "{} is not allowed by the current policy",
            permission_name
        ))
    }
}

/// Emit a TimelineEntry event for a completed tool execution.
pub fn emit_timeline(
    execution_id: &str,
    tool_id: &str,
    phase: &str,
    summary: &str,
    on_event: &dyn Fn(ExecutionEvent),
) {
    on_event(ExecutionEvent::TimelineEntry {
        execution_id: execution_id.to_string(),
        phase: phase.to_string(),
        tool_id: Some(tool_id.to_string()),
        summary: summary.to_string(),
    });
}
