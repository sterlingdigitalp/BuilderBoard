# BuilderBoard Backend Duplicate Work Audit

This document identifies areas in the backend where operations are redundantly performed during a single Builder request.

### 1. Filesystem Scan Duplication
*   **Where it occurs:** `src-tauri/src/filesystem_intent.rs` (`build_bundle_for_intents`) and `src-tauri/src/storage/commands.rs` (`execute_filesystem_tool_calls`).
*   **Why it occurs:** The intent router generates multiple separate `FilesystemToolCall` instructions for related concepts (e.g., a `SecurityReview` intent generates multiple `SearchFiles` calls for "auth", "oauth", "token", etc.). When executed, each call sequentially triggers a completely independent, recursive filesystem traversal (`fs::read_dir`) in `FilesystemService`.
*   **Estimated runtime cost:** High (O(N) full filesystem traversals, where N is the number of active search/find queries).
*   **Whether it is necessary:** No.
*   **Recommendation:** Consolidate search and find calls. Perform a single batched filesystem traversal that evaluates all active glob patterns and text queries simultaneously against each discovered file.

### 2. Registry Lookup Duplication
*   **Where it occurs:** `src-tauri/src/execution/manager.rs` (`resolve_stream_route` and `resolve`).
*   **Why it occurs:** `resolve_stream_route` queries the global builder registry (`global_builder_registry().get(route_id).is_some()`) to check if the route is a valid Builder. If true, it delegates to `resolve_for_chat` -> `resolve`, which performs the exact same registry HashMap lookup (`builder_reg.get(name)`) a second time to fetch the builder profile.
*   **Estimated runtime cost:** Negligible (HashMap lookup), but logically redundant.
*   **Whether it is necessary:** No.
*   **Recommendation:** Retrieve the `Arc<Builder>` once in `resolve_stream_route` using `if let Some(builder) = ...` and pass the resolved reference down the chain, or construct the `ExecutionResolution` directly.

### 3. Capability Validation Duplication
*   **Where it occurs:** `src-tauri/src/execution/capability_resolver.rs` (`resolve`).
*   **Why it occurs:** The `resolve` method independently calls both `resolve_allowed_tools` and `resolve_profile_tools`. Both of these helper functions iterate over the entire `ToolRegistry::list()` and redundantly evaluate `is_permission_allowed(perm, policy)` for every tool's permissions.
*   **Estimated runtime cost:** Low (the tool list is small), but performs redundant iterations and policy checks.
*   **Whether it is necessary:** No. The `profile_tools` set is strictly a subset of `allowed_tools`.
*   **Recommendation:** Compute `allowed_tools` once. Then, derive `profile_tools` by filtering the already-validated `allowed_tools` list based on the requested capability profile, eliminating the second registry iteration and permission check.

### 4. Prompt Construction Duplication
*   **Where it occurs:** `src-tauri/src/storage/commands.rs` (`prepare_stream_execution_db_only` and `enrich_conversation_with_filesystem`).
*   **Why it occurs:** During preparation, a system message containing the Project name and Approved root is appended to the conversation. Immediately after, if filesystem enrichment yields matches, a second, separate system message containing the formatted tool results is constructed and appended.
*   **Estimated runtime cost:** Low, but it unnecessarily fragments the context window with redundant system wrappers.
*   **Whether it is necessary:** No.
*   **Recommendation:** Combine the project metadata and the formatted filesystem tool results into a single, cohesive system message block during the final prompt construction phase.

### 5. Tool Preparation and Instantiation Duplication
*   **Where it occurs:** `src-tauri/src/stream_execution.rs` (Phase 9A.3 Tool Call Loop).
*   **Why it occurs:** Inside the `for (tool_name, arguments) in &tool_calls` loop, `exec_ctx_to_tool_ctx(&routing_context)` is invoked for *each* tool call. This forces the redundant cloning of the routing context (including the `environment` HashMap) for every tool executed in the same round. Additionally, the global `tool_registry` is read-locked and searched sequentially for each tool.
*   **Estimated runtime cost:** Medium (redundant memory allocations, HashMap cloning, and lock contention on the global registry).
*   **Whether it is necessary:** No. The routing context and available tools do not mutate between tool executions within a single round.
*   **Recommendation:** Instantiate the base `ToolContext` once outside the tool execution loop. Clone only what is strictly required for each execution. Additionally, resolve the tool references once before the loop begins.

### 6. Architectural Orchestration Duplication
*   **Where it occurs:** `src-tauri/src/stream_execution.rs` vs. `src-tauri/src/execution/tool_engine.rs`.
*   **Why it occurs:** The stream execution loop manually implements tool registry lookups, validation (`tool.validate`), event emission, and execution logic. This is an exact duplication of the orchestration responsibilities already encapsulated by the `ToolExecutionEngine`.
*   **Estimated runtime cost:** Maintenance overhead and duplicated procedural execution logic.
*   **Whether it is necessary:** No.
*   **Recommendation:** Refactor the stream loop to construct an `ExecutionRequest::Tool` and route it through the `ToolExecutionEngine`, centralizing tool preparation, validation, and execution.
