//! ToolRegistry — global registry of all available tools.
//!
//! Thread-safe singleton. Tools are registered once at startup.
//! Engines, Skills, and Builders discover tools through this registry.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use crate::execution::manager::ExecutionClass;
use crate::execution::tools::traits::Tool;

/// Global tool registry singleton.
static GLOBAL_REGISTRY: LazyLock<Arc<RwLock<ToolRegistry>>> =
    LazyLock::new(|| Arc::new(RwLock::new(ToolRegistry::new())));

/// Return a handle to the global tool registry.
pub fn global_tool_registry() -> Arc<RwLock<ToolRegistry>> {
    GLOBAL_REGISTRY.clone()
}

/// The ToolRegistry owns discovery, registration, lookup, and capability matching.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool. Returns an error if a tool with the same ID already exists.
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<(), String> {
        let id = tool.id().to_string();
        if self.tools.contains_key(&id) {
            return Err(format!("Tool '{}' is already registered", id));
        }
        self.tools.insert(id, tool);
        Ok(())
    }

    /// Get a tool by its unique ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(id).cloned()
    }

    /// List all registered tools.
    pub fn list(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Find tools that support a given execution class.
    pub fn find_by_class(&self, class: &ExecutionClass) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| t.supported_execution_classes().contains(class))
            .cloned()
            .collect()
    }

    /// Find tools by category name.
    pub fn find_by_category(&self, category: &str) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| t.category_name() == category)
            .cloned()
            .collect()
    }

    /// Find a tool by its display name (case-insensitive, partial match).
    pub fn find_by_name(&self, name: &str) -> Vec<Arc<dyn Tool>> {
        let lower = name.to_lowercase();
        self.tools
            .values()
            .filter(|t| {
                t.display_name().to_lowercase().contains(&lower)
                    || t.description().to_lowercase().contains(&lower)
            })
            .cloned()
            .collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Register all default tools into the global registry.
/// Called once at application startup.
pub fn register_default_tools() -> Result<usize, String> {
    let registry = global_tool_registry();
    let mut reg = registry.write().map_err(|e| e.to_string())?;

    let mut count = 0;

    macro_rules! reg_tool {
        ($t:expr) => {{
            reg.register(Arc::new($t))?;
            count += 1;
        }};
    }

    reg_tool!(crate::execution::tools::shell::ShellTool);
    reg_tool!(crate::execution::tools::filesystem::ReadTool);
    reg_tool!(crate::execution::tools::filesystem::WriteTool);
    reg_tool!(crate::execution::tools::filesystem::EditTool);
    reg_tool!(crate::execution::tools::filesystem::DeleteTool);
    reg_tool!(crate::execution::tools::directory::ListTool);
    reg_tool!(crate::execution::tools::directory::CreateTool);
    reg_tool!(crate::execution::tools::package::InstallTool);
    reg_tool!(crate::execution::tools::package::UninstallTool);
    reg_tool!(crate::execution::tools::package::ListTool);
    reg_tool!(crate::execution::tools::git::StatusTool);
    reg_tool!(crate::execution::tools::git::DiffTool);
    reg_tool!(crate::execution::tools::git::CommitTool);
    reg_tool!(crate::execution::tools::git::LogTool);
    reg_tool!(crate::execution::tools::process::ListTool);
    reg_tool!(crate::execution::tools::process::KillTool);
    reg_tool!(crate::execution::tools::search::GrepTool);
    reg_tool!(crate::execution::tools::search::GlobTool);
    reg_tool!(crate::execution::tools::diagnostics::HealthTool);
    reg_tool!(crate::execution::tools::diagnostics::EnvTool);

    Ok(count)
}
