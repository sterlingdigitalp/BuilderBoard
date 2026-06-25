//! Generalized ExecutionContext.
//!
//! Replaces PaneExecutionContext assumptions. Contains only what an execution
//! needs: project scope, filesystem, environment, optional credentials,
//! cancellation, policy, etc. Credential material is optional.
//! Local engines (Ollama, MLX, LM Studio, sidecars) require none.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::auth::{CredentialHandle, CredentialService};
use crate::filesystem_tools::scope::ApprovedScope;

/// Lightweight reference to project context for execution.
#[derive(Clone, Debug)]
pub struct ProjectContext {
    pub id: String,
    pub name: String,
    pub approved_root: PathBuf,
}

/// Execution policy / constraints for the run.
#[derive(Clone, Debug, Default)]
pub struct ExecutionPolicy {
    pub allow_shell: bool,
    pub allow_network: bool,
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_delete: bool,
    pub allow_git: bool,
    pub allow_packages: bool,
    pub allow_processes: bool,
    pub max_tokens: Option<u32>,
    pub timeout_ms: Option<u64>,
    // Future: sandbox profile, permission model, etc.
}

/// Normalized execution context passed to every engine.
///
/// This is deliberately execution-focused (not chat or provider focused).
#[derive(Clone)]
pub struct ExecutionContext {
    /// Unique identifier for this execution run (for tracing, cancellation, artifacts).
    pub execution_id: String,

    /// Optional pane (for chat-like UIs).
    pub pane_id: Option<String>,

    /// Project + filesystem boundary (core for Skills, local tools, etc.).
    pub project: Option<ProjectContext>,
    pub filesystem_scope: Option<ApprovedScope>,

    /// Environment variables / context variables available to the engine.
    pub environment: HashMap<String, String>,

    /// Execution policy (permissions, limits).
    pub policy: ExecutionPolicy,

    /// Optional cancellation signal (engines should honor).
    /// For now a simple Arc flag; future can be more sophisticated.
    pub cancellation: Option<Arc<std::sync::atomic::AtomicBool>>,

    /// Optional credentials / identity. Engines that don't need auth leave this None.
    pub credential: Option<CredentialHandle>,
    /// The service is provided only if the engine declares it needs auth.
    /// Local engines can ignore this entirely.
    pub credential_service: Option<Arc<CredentialService>>,

    /// Future extensibility: attachments, memory refs, builder config, etc.
    pub attachments: Vec<String>, // placeholder for paths or refs
    pub memory_refs: Vec<String>, // placeholder

    /// Additional free-form metadata for the engine (engine-specific hints).
    pub metadata: HashMap<String, String>,

    /// Preferred working directory for CLI engines (e.g. for --cwd).
    pub cwd: Option<std::path::PathBuf>,
}

impl ExecutionContext {
    /// Minimal context for a local / no-auth engine.
    pub fn local(execution_id: impl Into<String>) -> Self {
        Self {
            execution_id: execution_id.into(),
            pane_id: None,
            project: None,
            filesystem_scope: None,
            environment: HashMap::new(),
            policy: ExecutionPolicy::default(),
            cancellation: None,
            credential: None,
            credential_service: None,
            attachments: vec![],
            memory_refs: vec![],
            metadata: HashMap::new(),
            cwd: None,
        }
    }

    /// Construct from existing pane/project data (used by OpenAI adapter during migration).
    pub fn from_pane_project(
        execution_id: impl Into<String>,
        pane_id: Option<String>,
        project: Option<ProjectContext>,
        scope: Option<ApprovedScope>,
        cwd: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            execution_id: execution_id.into(),
            pane_id,
            project,
            filesystem_scope: scope,
            environment: HashMap::new(),
            policy: ExecutionPolicy::default(),
            cancellation: None,
            credential: None,
            credential_service: None,
            attachments: vec![],
            memory_refs: vec![],
            metadata: HashMap::new(),
            cwd,
        }
    }
}
