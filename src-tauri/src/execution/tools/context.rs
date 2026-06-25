use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::filesystem_tools::scope::ApprovedScope;

#[derive(Clone)]
pub struct ToolContext {
    pub execution_id: String,
    pub pane_id: Option<String>,
    pub project_root: Option<PathBuf>,
    pub filesystem_scope: Option<ApprovedScope>,
    pub cwd: Option<PathBuf>,
    pub environment: HashMap<String, String>,
    pub cancellation: Option<Arc<AtomicBool>>,
    pub timeout_ms: Option<u64>,
    pub allow_shell: bool,
    pub allow_network: bool,
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_delete: bool,
    pub allow_git: bool,
    pub allow_packages: bool,
    pub allow_processes: bool,
}

impl ToolContext {
    pub fn local(execution_id: impl Into<String>) -> Self {
        Self {
            execution_id: execution_id.into(),
            pane_id: None,
            project_root: None,
            filesystem_scope: None,
            cwd: None,
            environment: HashMap::new(),
            cancellation: None,
            timeout_ms: None,
            allow_shell: true,
            allow_network: true,
            allow_read: true,
            allow_write: true,
            allow_delete: true,
            allow_git: true,
            allow_packages: true,
            allow_processes: true,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(false)
    }
}
