# Tool Pipeline Performance Report

This report outlines the lifecycle of a tool call executed via the `ToolExecutionEngine`, tracing through validation, resolution, execution, serialization, and response formatting, based on empirical measurements and code analysis in `src-tauri/src/execution/`.

## Measurement Setup
A unit test simulating an invocation of the `shell` tool using the `ToolExecutionEngine` was constructed (`simulate_tool_call_trace`). The test captures timestamps when standard `ExecutionEvent`s are emitted during the lifecycle.

## Pipeline Lifecycle Timings

A single run of `echo 'tracing shell tool'` via `ToolExecutionEngine::execute()` resulted in a total duration of **21.7ms** (21713 µs).

The pipeline phases can be broken down as follows:

| Event | Time since start (µs) | Delta (µs) | Description |
| :--- | :--- | :--- | :--- |
| `RunStarted` | 5 | 5 | The engine receives the `ExecutionRequest` and begins processing. |
| `PermissionCheck` | 55 | 50 | **Validation & Resolution Phase:** The engine resolves the tool from `ToolRegistry`, validates arguments against the tool schema, checks capabilities in `ToolContext`, and verifies permissions via `helpers::check_permission()`. |
| `ToolStarted` | 67 | 12 | **Execution Phase (Spawn):** The tool successfully passes validation and permission checks, and begins execution. For `shell`, a child process (`sh -c`) is prepared. |
| `ToolOutput` | 21,608 | 21,541 | **Execution Phase (I/O & Streams):** The vast majority of the duration (~21ms) is spent awaiting the spawn, I/O thread reading from `stdout`, and buffering the output. |
| `ToolFinished` | 21,646 | 37 | **Response Formatting Phase:** The tool's child process has successfully exited (`exit_code = 0`). The tool packs `stdout` and `stderr` buffers into a `ToolOutput`. |
| `TimelineEntry` | 21,652 | 5 | **Telemetry:** Emitting timeline events via helper functions. |
| `ReviewItemCreated` | 21,672 | 20 | **Serialization / Response Construction:** Building the `ReviewItem` containing action descriptions and buffer byte counts. |
| `ArtifactCreated` | 21,700 | 27 | **Serialization / Response Construction:** Constructing the `ToolArtifact` payload containing the stdout/stderr transcript. |
| `ReviewItemCreated` | 21,703 | 3 | Emitted at engine level. |
| `RunCompleted` | 21,706 | 2 | **Completion:** The engine finalizes the request. |

*(Total Engine execution duration measured externally: 21,713 µs).*

## Phase Analysis and Unnecessary Work Identified

### 1. Validation & Resolution (50 µs)
- **What happens:** The manager resolves the execution engine. Inside `ToolExecutionEngine::execute`, the engine looks up the tool in the `global_tool_registry()` (using an `RwLock`), maps context limits, calls `tool.validate()`, and `helpers::check_permission`.
- **Inefficiencies / Unnecessary Work:**
  - `ToolExecutionEngine::execute` acquires a read lock on the `global_tool_registry()` for *every* request. For single embedded invocations, this is fast (µs), but under heavy parallel load could become a bottleneck.
  - The arguments passed to `tool.validate()` and `tool.execute()` are fully cloned: `tool.execute(tool_ctx, tool_req.arguments.clone(), ...)`

### 2. Execution (Spawn & I/O) (~21.5 ms)
- **What happens:** The `ShellTool` clones strings, prepares a `std::process::Command`, spawns the child process, and spawns *two* dedicated threads via `thread::spawn` just to read `stdout` and `stderr` through a channel (`mpsc::channel`).
- **Inefficiencies / Unnecessary Work:**
  - **Thread Allocation Overhead:** The `ShellTool` creates two new OS threads per invocation. Given that `ToolExecutionEngine::execute` runs in a `tokio` future context (`Pin<Box<dyn Future...>>`), this synchronous thread spawning and `mpsc` blocking channel is an impedance mismatch. `ShellTool` is blocking an async executor thread inside `output_rx.recv_timeout()`.
  - **String Copying:** Standard output strings are copied multiple times: read from `BufReader` -> passed through `mpsc` -> appended to `stdout_buf` -> cloned for `ExecutionEvent::ToolOutput`.

### 3. Serialization & Response Formatting (~50 µs)
- **What happens:** The `ShellTool` produces a `ToolResult` containing `ToolOutput`, `ToolArtifact`, and `ReviewItem` structs. The engine iterates over these and maps them to `ExecutionEvent`s.
- **Inefficiencies / Unnecessary Work:**
  - The `ShellTool` eagerly constructs full `String` summaries and copies the entire stdout/stderr buffers into `ToolArtifact` content blocks.
  - Furthermore, `ToolExecutionEngine::execute` clones strings from the result struct again when converting them to events: `action: item.action.clone()`, `summary: item.summary.clone()`, etc.

## Recommendations
1. **Adopt Async I/O for Tools:** Replace `std::process::Command`, `thread::spawn`, and `mpsc` in `ShellTool` (and similar process-spawning tools) with `tokio::process::Command`. This eliminates the need to spawn expensive OS threads for stream reads and avoids blocking the async executor.
2. **Reduce String Cloning:** Instead of cloning large `serde_json::Value` arguments or string buffers, consider passing by reference where possible during validation, and rely on `Arc<str>` or `Bytes` for buffering large command outputs to avoid redundant deep copies in Artifact generation.