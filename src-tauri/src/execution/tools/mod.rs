//! Phase 9A — Tool Runtime v1
//!
//! Provider-neutral Tool Runtime for all ExecutionEngines.
//! Tools are engine-independent. Engines invoke tools via ToolRegistry.
//! Tools produce structured artifacts and emit normalized ExecutionEvents.
//!
//! Architecture:
//!   ExecutionRequest
//!     → ExecutionManager
//!       → ToolRegistry → Tool::execute()
//!         → ExecutionEvent → Timeline / Artifacts / Review

pub mod context;
pub mod permissions;
pub mod registry;
pub mod results;
pub mod traits;

pub mod diagnostics;
pub mod directory;
pub mod filesystem;
pub mod git;
pub mod helpers;
pub mod package;
pub mod process;
pub mod search;
pub mod shell;

pub use context::ToolContext;
pub use permissions::{PermissionLevel, ToolPermission};
pub use registry::{global_tool_registry, register_default_tools, ToolRegistry};
pub use results::{ReviewItem, ToolArtifact, ToolOutput, ToolResult};
pub use traits::Tool;

#[cfg(test)]
pub mod tests;
