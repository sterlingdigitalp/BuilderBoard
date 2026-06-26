# Runtime Architecture Audit against Core Definition

This document details every place where BuilderBoard's current runtime architecture and implementation diverge from its Core Definition for Version 1. The findings are ranked in descending order of impact on the Core Promise (multi-pane independence, runtime capability, reliability, and latency).

### 1. Hardcoded Builder Routing (Independence Violation)
*   **Impact Rank:** 1 (Highest - Breaks Core Independence Promise)
*   **Divergence:** The `Core Definition` states: "Changing one Builder must not affect another", "Builders do not share context", and each Builder is independent. However, in `src-tauri/src/stream_execution.rs:136`, builder routing is hardcoded to specific string identities (`"builder-a"`, `"builder-b"`, `"builder-c"`).
*   **Issue Tracking:** BB-0003
*   **Effect:** The `ExecutionManager` is bypassed for non-builder paths, and adding or isolating new builders requires source code modification, violating the architectural independence of the Builder registry and isolation boundaries.

### 2. Planner Convergence & Deduplication Failure
*   **Impact Rank:** 2 (Breaks Reliability and Latency)
*   **Divergence:** The `Core Definition` requires Builders to "reliably complete general engineering requests" with "acceptable reliability and latency." Currently, the execution planner lacks cost-awareness and a semantic convergence condition (stopping when it has enough information).
*   **Issue Tracking:** BB-0006, BB-0009
*   **Effect:** This causes excessive tool call rounds, duplicate `(tool_name, arguments)` pairs, and frequent exhaustion of the planner budget before tasks complete. It creates a cascade of latency and unreliability.

### 3. Suboptimal Repository Discovery Architecture
*   **Impact Rank:** 3 (Breaks Capability Promise)
*   **Divergence:** The `Core Definition` dictates that a Builder must be capable of "understanding a software project" and "searching code".
*   **Issue Tracking:** BB-0001, BB-0008
*   **Effect:** Because there is no single, fast repository inventory tool, the planner relies on composing inefficient directory listing and file search operations. This compounds with the convergence issue (Rank 2) to cause repository-scale tasks to consistently time out or exhaust planner limits.

### 4. Database Concurrency Contention (Latency and Responsiveness)
*   **Impact Rank:** 4 (Breaks Responsiveness)
*   **Divergence:** The `Core Definition` explicitly requires that "The application should remain responsive throughout normal operation" and have "acceptable reliability and latency."
*   **Effect:** `src-tauri/src/storage/db.rs` wraps the SQLite `Connection` in a `Mutex<Connection>`. The `runtime_blocking_diagnostic.rs` tests (and related telemetry) show that this single mutex serializes concurrent readers. When four independent panes are attempting to read/write state simultaneously (which is the core premise of the app), they heavily contend for this single lock, producing runtime latency spikes that breach the Bronze latency limits (BB-0007).

### 5. Single Shared Tool Registry Mutex
*   **Impact Rank:** 5 (Breaks Independence/Scalability)
*   **Divergence:** Independence requires that "Builders do not interfere with one another."
*   **Effect:** `src-tauri/src/execution/tools/registry.rs` uses a global singleton `LazyLock<Arc<RwLock<ToolRegistry>>>`. Although it is an `RwLock`, having a single global registry rather than per-pane or per-workspace registries limits future capabilities for pane-specific tool configurations or isolated dynamic tool loading, and can introduce contention during initialization or tool resolution.


### 6. Missing Tool Permission Audit Trail (Capability Gap)
*   **Impact Rank:** 6 (Breaks Security/User Control)
*   **Divergence:** Phase 8.9D/8.9C architectures and Builder rules demand that tool permissions MUST be explicit, reviewable, and auditable at runtime. The UX must surface what tools a Builder is requesting and let the user approve/deny/permit-per-session.
*   **Effect:** Currently, the system has an incomplete implementation of the tool permission auditing capability, preventing the user from retaining full auditable control of all Builders.
