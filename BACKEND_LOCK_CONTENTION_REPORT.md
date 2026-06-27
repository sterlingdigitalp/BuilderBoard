
# Backend Lock Contention Report

This report audits every `Mutex`, `RwLock`, `Arc<RwLock>`, `OnceLock`, and shared synchronization primitive in the backend.

## 1. `ProjectScopeCache` (in `src-tauri/src/project_scope_cache.rs`)

**Type:** `Mutex<HashMap<ScopeCacheKey, CachedProjectScope>>`
- **What it protects:** The in-memory cache of resolved and validated project scopes. It maps project IDs and approved roots to their `CachedProjectScope`.
- **Expected Contention:** Very low. Scopes are cached to avoid repeated disk reads. Contention only happens on cache misses or cache invalidation, which are rare compared to the frequency of tool executions that read the cache.
- **Blocks Concurrent Builders:** Yes, momentarily. A Builder validating a path or resolving scope needs the lock. If multiple Builders miss the cache simultaneously, one will block. But operations inside the lock are fast (mostly HashMap lookups and inserts).
- **Affects Version 1:** Yes. Project scope validation is a core Version 1 feature for filesystem security.
- **Estimated Runtime Impact:** Negligible. The locked sections are very short.

## 2. `OpenAIProvider` (in `src-tauri/src/providers/mod.rs`)

**Type:** `OnceLock<reqwest::blocking::Client>`
- **What it protects:** Lazy initialization of a synchronous HTTP client.
- **Expected Contention:** None. It's a `OnceLock` initialized on the first synchronous request. Subsequent accesses are lock-free reads.
- **Blocks Concurrent Builders:** No. Builders use the async client (`reqwest::Client`) primarily, but even if using the blocking client, initialization only blocks the very first caller.
- **Affects Version 1:** Yes, provides the core OpenAI LLM integration.
- **Estimated Runtime Impact:** Zero after initialization.

## 3. `BuilderRegistry` (in `src-tauri/src/builders/mod.rs`)

**Type:** `OnceLock<BuilderRegistry>`
- **What it protects:** The global, static registry of available Builders (execution policies).
- **Expected Contention:** None. Initialized once at startup or first use.
- **Blocks Concurrent Builders:** No. Read-only access after initialization.
- **Affects Version 1:** Yes. Builder definition is core to Version 1.
- **Estimated Runtime Impact:** Zero after initialization.

## 4. `Database` (in `src-tauri/src/storage/db.rs`)

**Type:** `Mutex<rusqlite::Connection>`
- **What it protects:** The single SQLite database connection used by the backend.
- **Expected Contention:** Moderate to High. All database operations (reading/writing chats, tokens, project states) across all concurrent Builders and the UI go through this single lock.
- **Blocks Concurrent Builders:** Yes. Only one thread can execute a database query at any given time. If Builder A is saving a long conversation history, Builder B must wait to read a tool definition.
- **Affects Version 1:** Yes, deeply embedded.
- **Estimated Runtime Impact:** Significant under heavy load (e.g., 4 active Builders streaming lots of output). The database uses WAL mode, but the single connection mutex serialize all queries at the application level.

## 5. `EngineRegistry` (in `src-tauri/src/execution/engine.rs`)

**Type:** `OnceLock<EngineRegistry>`
- **What it protects:** The global registry of execution engines (e.g., standard, structured).
- **Expected Contention:** None. Initialized once.
- **Blocks Concurrent Builders:** No.
- **Affects Version 1:** Yes.
- **Estimated Runtime Impact:** Zero after initialization.

## 6. `ToolRegistry` (in `src-tauri/src/execution/tools/registry.rs`)

**Type:** `LazyLock<Arc<RwLock<ToolRegistry>>>`
- **What it protects:** The global registry of available tools (capabilities).
- **Expected Contention:** Low. The registry is populated at startup (write lock). During execution, Builders only acquire read locks to look up tool definitions. Since it's an `RwLock`, multiple readers can access it concurrently.
- **Blocks Concurrent Builders:** No, not during normal execution (multiple readers allowed). It would only block if new tools were registered at runtime, which typically isn't done after startup.
- **Affects Version 1:** Yes.
- **Estimated Runtime Impact:** Negligible due to concurrent read access.

## 7. `OAuthService` Pending Sessions (in `src-tauri/src/auth/oauth_service.rs`)

**Type:** `Arc<Mutex<HashMap<String, PendingOAuthSession>>>`
- **What it protects:** In-flight OAuth flows.
- **Expected Contention:** Very low. OAuth flows are initiated by user interaction and happen rarely (e.g., during login).
- **Blocks Concurrent Builders:** No. Builders do not initiate OAuth flows.
- **Affects Version 1:** Yes, for authentication.
- **Estimated Runtime Impact:** Negligible.

## 8. Mock/Test OAuth Sync Primitives (in `src-tauri/src/auth/oauth_service.rs` tests)

**Type:** `Arc<Mutex<Vec<String>>>`, `Arc<Mutex<Option<(String, String, String)>>>`, `Arc<Mutex<Option<String>>>`
- **What it protects:** Test state and mock assertion data.
- **Expected Contention:** Low (only in tests).
- **Blocks Concurrent Builders:** N/A (test only).
- **Affects Version 1:** No.
- **Estimated Runtime Impact:** None.

## 9. `MemoryCredentialStore` (in `src-tauri/src/auth/credential_service.rs`)

**Type:** `Mutex<HashMap<String, String>>`
- **What it protects:** An in-memory store for credentials (used as a fallback or in tests).
- **Expected Contention:** Low. Credential access is fast, though needed whenever an API call is made.
- **Blocks Concurrent Builders:** Yes, momentarily when fetching credentials.
- **Affects Version 1:** Yes, if used instead of the keychain.
- **Estimated Runtime Impact:** Very small. The critical section is just a HashMap lookup.

## 10. Diagnostics and Telemetry (in `src-tauri/src/runtime_diagnostics.rs` and `tests`)

**Type:** `AtomicU64` (`COMMAND_THREAD_BLOCK_MS`, `TEST_DB_COUNTER`), `AtomicUsize`
- **What it protects:** Lock-free counters for observability and test metrics.
- **Expected Contention:** Low (atomic operations are fast).
- **Blocks Concurrent Builders:** No.
- **Affects Version 1:** Yes (observability).
- **Estimated Runtime Impact:** Negligible (lock-free hardware instructions).

## 11. Execution Cancellation Flags (in `cli.rs`, `context.rs`, `grok_build.rs`, `oauth_service.rs`)

**Type:** `Arc<AtomicBool>`
- **What it protects:** Lock-free boolean flags to signal cancellation of tasks, executions, or auth flows.
- **Expected Contention:** Low. Written once (when cancelled), read frequently.
- **Blocks Concurrent Builders:** No.
- **Affects Version 1:** Yes, allows stopping builders safely.
- **Estimated Runtime Impact:** Negligible.
