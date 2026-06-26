# Builder Isolation Audit

This document summarizes potential areas where Builder state could leak between panes, leading to violations of the Core Promise (four independent Builder panes).

## 1. Globals & Singletons

### Engine Registry
- **File:** `src-tauri/src/execution/engine.rs`
- **Definition:** `static REGISTRY: OnceLock<EngineRegistry> = OnceLock::new();`
- **Risk:** Shared mapping of engine ID to Engine instance. The instances (`OpenAIExecutionEngine`, `GrokBuildExecutionEngine`, `ToolExecutionEngine`) are currently stateless (empty structs or simple configuration). *Low risk of state leakage*, but potential for thread contention or concurrent initialization issues if mutated (which `OnceLock` prevents after init).

### Builder Registry
- **File:** `src-tauri/src/builders/mod.rs`
- **Definition:** `static REGISTRY: OnceLock<BuilderRegistry> = OnceLock::new();`
- **Risk:** Shared definition of available Builders. Read-only after initialization. *No state leakage risk.*

### Tool Registry
- **File:** `src-tauri/src/execution/tools/registry.rs`
- **Definition:** `static GLOBAL_REGISTRY: LazyLock<Arc<RwLock<ToolRegistry>>> = LazyLock::new(|| Arc::new(RwLock::new(ToolRegistry::new())));`
- **Risk:** Shared mapping of tool ID to Tool instance. `ToolRegistry::register_default_tools` acquires a write lock. This is technically mutable state, although tools are expected to be registered only at startup. The Tool implementations (`ShellTool`, `ReadTool`, etc.) are zero-sized types (stateless). *Low risk of state leakage*, but `RwLock` introduces potential contention and deadlock vectors during initialization if called concurrently (e.g. from multiple panes at startup if `register_default_tools` isn't called upfront on main thread).

### Diagnostics & Telemetry
- **File:** `src-tauri/src/runtime_diagnostics.rs`
- **Definition:** `static COMMAND_THREAD_BLOCK_MS: AtomicU64 = AtomicU64::new(0);`
- **Risk:** Shared counter for blocking duration. *No functional state leakage*, but metrics are globally aggregated, obscuring per-pane performance data.

## 2. Shared Caches

### Project Scope Cache
- **File:** `src-tauri/src/project_scope_cache.rs`
- **Definition:** Managed by Tauri state (`app.manage(project_scope_cache)`), passed as `Arc<ProjectScopeCache>`.
- **Implementation:** `entries: Mutex<HashMap<ScopeCacheKey, CachedProjectScope>>`
- **Risk:** *High risk of interference.* If multiple Builders operate on the same project (allowed by Phase 1 definitions), they share the same cached scope. If one Builder invalidates the project (`invalidate_project`), it forces a re-computation for the other Builder, causing contention. The cache key is just `project_id` and `approved_root`. While the scope itself (`ApprovedScope`) seems to just be a root path representation, sharing mutable cache infrastructure across independent panes creates a coupling vector.

### Filesystem Intent Validation Cache (Read Cache)
- **File:** `src-tauri/src/storage/commands.rs` (in `prepare_filesystem_enrichment_with_scope`)
- **Definition:** `let mut read_cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();`
- **Risk:** *No cross-pane leakage.* This cache is isolated per request/execution context (created inside the `prepare_filesystem_enrichment_with_scope` function scope). It only caches reads within a single turn.

## 3. Storage & Database (Shared Repositories)

- **File:** `src-tauri/src/storage/db.rs`
- **Definition:** `connection: Mutex<Connection>` inside `Database` managed by Tauri.
- **Risk:** *High risk of contention.* SQLite is fundamentally single-writer. A `Mutex` around a single connection means all four panes writing telemetry, messages, or stream deltas will block each other. This is not a "leak" of logical state, but a leakage of execution performance (blocking).
- **Logical Leakage:** Repositories (`PaneRepository`, `MessageRepository`, `ProjectRepository`) query the database. They rely heavily on `pane_id` and `project_id` to isolate data. If a query forgets a `WHERE pane_id = ?` clause, data could leak. (e.g. `MessageRepository::list_for_pane` correctly scopes by `pane_id`).

## 4. Execution State

- **File:** `src-tauri/src/execution/context.rs`, `src-tauri/src/execution/manager.rs`
- **Risk:** `ExecutionContext` seems cleanly instanced per execution, carrying `execution_id`, `project_id`, `fs_scope`, and `cancellation`.
- **Risk (Grok Build Engine):** `src-tauri/src/execution/grok_build.rs`. Grok Build spawns a subprocess. If two panes invoke Grok Build simultaneously in the same directory, they might clobber each other's build outputs, lock files, or temporary artifacts.

## 5. Shared Conversations & Context

- **File:** `src-tauri/src/stream_execution.rs`
- **Risk:** `Conversation` context is built per request. The risk here is if the frontend inadvertently sends the wrong `conversation_id` or mixes messages when multiple panes are active. The backend relies entirely on the frontend correctly scoping the conversation to the pane.

## 6. Tauri State & Command Handlers

- **Tauri State (`manage()`):**
  - `Database`
  - `CredentialService`
  - `OAuthService`
  - `StreamPersistenceService`
  - `ProjectScopeCache`
- **Risk:** These are all shared instances (via `Arc` or internal `Mutex`). Any mutable state inside these services is a potential vector for cross-pane interference.

## Summary of Findings

1.  **State Leakage:** The application is generally well-structured to avoid logical state leakage. Engines, Tools, and Builders are largely stateless configurations. State is passed down via `ExecutionContext` and `ExecutionRequest`.
2.  **Contention & Blocking (Performance Leakage):** The biggest violation of the "independent pane" promise is shared mutable locks:
    -   `Database` connection lock (`Mutex<Connection>`).
    -   `ToolRegistry` lock (`RwLock<ToolRegistry>`).
    -   `ProjectScopeCache` lock (`Mutex<HashMap>`).
    -   `OAuthService` pending sessions lock (`Mutex<HashMap>`).
3.  **Subprocess Interference:** Independent Builders acting on the same filesystem/project simultaneously (e.g., `npm install`, `cargo build` via shell tools or `GrokBuildExecutionEngine`) will inevitably clash at the OS level (file locks, concurrent modification) unless carefully orchestrated or isolated at the container/workspace level.
