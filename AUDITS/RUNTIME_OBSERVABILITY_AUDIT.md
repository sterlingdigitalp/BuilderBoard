# Runtime Observability Audit

This document analyzes whether each item in the `RUNTIME_ENGINEERING_LEDGER.md` could currently be diagnosed using runtime logs alone, and recommends missing instrumentation.

## Current Audit by Ledger Entry

### Backend / Planner Issues (Largely Diagnosable)

The backend planner loop has significant instrumentation through `native_tool_trace.rs`, `PerfSpan`, and `trace_runtime_phase`. The request and response traces capture execution rounds.

*   **BB-0006 — Planner lacks convergence detection for repository-scale enumeration:** Diagnosable. `execution_timeline.jsonl` and `response_round_*.jsonl` show the loop behavior, duplicate tool calls, and round termination events.
*   **BB-0008 — No fast repository inventory capability:** Diagnosable. The lack of capability is evident from the extensive tool chains traced in the output files.
*   **BB-0002 — Tool validation failures cause planner retry cascades:** Diagnosable. Validation failures are captured in the tool traces and execution timelines.
*   **BB-0009 — Planner budget consumed by inefficient multi-tool sequences:** Diagnosable. Execution round count and redundant requests are captured in native traces.
*   **BB-0001 — Repository-scale discovery missions exhaust planner budget:** Diagnosable. Hard limits ("Maximum number of tool call rounds reached") are emitted in backend traces and planner loop logs.
*   **BB-0007 — Runtime latency exceeds acceptable threshold for engineering tasks:** Diagnosable. The `PerfSpan` instrumentation captures duration for each phase and request (e.g. `TOTAL_REQUEST_DURATION_MS`, `TTFT_MS`, `ENGINE_STREAM_TOTAL_MS`).
*   **BB-0010 — Builders cannot complete general engineering requests:** Diagnosable. This is the top-level synthesis, and its constituent failures are traced via the backend logging mechanisms.
*   **BB-0003 — Hardcoded builder routing bypasses ExecutionManager:** Partially Diagnosable. Routing choices (builder vs engine) are captured in `trace_runtime_phase("execution_manager_decision", ...)` and the `execution_manager` JSON event, though the exact string matching in `stream_execution.rs` is an architectural choice.

### Tool Execution Issues (Not Fully Diagnosable)

The individual tool execution files (`src-tauri/src/execution/tools/*.rs` and `src-tauri/src/filesystem_tools/scope.rs`) lack granular internal diagnostic logs.

*   **BB-0004 — Filesystem scope resolver rejects non-existent paths:** Not fully diagnosable via logs alone. The `scope.rs` validation paths fail with generic user-facing error strings (e.g. "outside scope" or "failed to resolve path"), without tracing *why* the path traversal failed internally (e.g. `canonicalize()` failure vs logic failure).
*   **BB-0005 — Search tool reports failure on no-match result:** Not fully diagnosable via logs alone. The search tool (`search.rs`) returns error results on empty grep outputs without logging the `stderr` or exit codes internally before returning the generic error.

### Frontend Issues (Undiagnosable)

The frontend lacks explicit error/debug tracing for API calls and state management.

*   **BB-0011 — Frontend data loading uses Promise.all with no error isolation:** Undiagnosable. The frontend data fetching hooks (`usePaneChat.ts`) do not use `console.log` or `console.error` to trace which API calls succeed or fail.
*   **BB-0012 — sendMessage stale closure on selectedBuilderId:** Undiagnosable. There is no trace instrumentation tracking frontend state transitions or the exact payload being composed at execution time before dispatching to the backend.

---

## Recommended Missing Instrumentation

To ensure full runtime diagnosability, the following instrumentation should be added:

1.  **Frontend State & API Logging:**
    *   Add `console.debug` and `console.error` inside frontend hooks (like `usePaneChat.ts`) to trace data fetching progress, isolated error captures, and payload composition before calling Tauri commands.

2.  **Tool-Level Granular Tracing:**
    *   **Scope Validation (`scope.rs`):** Add internal `trace_runtime_phase` or `println!` (when trace enabled) for path resolution steps. It should explicitly log when a path is rejected due to `canonicalize()` failure versus failing the `is_within_root` check.
    *   **Subprocess Execution (`search.rs`):** Add tracing for shell commands that captures the raw arguments, exit code, and `stderr` before normalizing to a tool failure.

3.  **Builder Routing Tracing:**
    *   Add explicit tracing in `stream_execution.rs` and `execution_manager.rs` that logs the *exact* route decision branch taken (e.g., "Routed via BuilderRegistry", "Routed via EngineRegistry fallback").

