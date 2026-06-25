//! Core Tool trait.
//!
//! All tools implement this trait. Engines invoke tools through this interface.
//! Tools never call engines — they emit ExecutionEvents via the on_event callback.

use crate::execution::event::ExecutionEvent;
use crate::execution::manager::ExecutionClass;
use crate::execution::tools::context::ToolContext;
use crate::execution::tools::permissions::ToolPermission;
use crate::execution::tools::results::ToolDescriptor;
use crate::execution::tools::results::ToolResult;
use serde_json::Value;

/// Unique, immutable identifier for a tool.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolId(pub &'static str);

impl ToolId {
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The core Tool trait — all tools implement this.
pub trait Tool: Send + Sync {
    /// Unique tool identifier (e.g., "shell", "filesystem.read", "git.status").
    fn id(&self) -> ToolId;

    /// Human-readable name.
    fn display_name(&self) -> String;

    /// Description of what the tool does.
    fn description(&self) -> String;

    /// Which execution classes this tool supports.
    fn supported_execution_classes(&self) -> Vec<ExecutionClass>;

    /// Permissions this tool requires.
    fn permissions(&self) -> Vec<ToolPermission>;

    /// Validate arguments before execution.
    fn validate(&self, args: &Value) -> Result<(), String>;

    /// Execute the tool with the given context and arguments.
    /// Emits events via the on_event callback.
    fn execute(
        &self,
        ctx: ToolContext,
        args: Value,
        on_event: &dyn Fn(ExecutionEvent),
    ) -> Result<ToolResult, String>;

    /// Describe this tool for discovery / frontend.
    fn describe(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: self.id().to_string(),
            display_name: self.display_name(),
            description: self.description(),
            category: self.category_name(),
            permissions: self
                .permissions()
                .iter()
                .map(|p| p.as_str().to_string())
                .collect(),
            supported_engines: vec![],
            supported_execution_classes: self
                .supported_execution_classes()
                .iter()
                .map(|c| format!("{:?}", c))
                .collect(),
        }
    }

    /// Category name used for grouping in the UI.
    fn category_name(&self) -> String {
        "general".to_string()
    }
}
