# BuilderBoard Runtime Latency Analysis

This analysis is based on the runtime traces captured in `src-tauri/runtime_traces/execution_timeline.jsonl` for a representative execution task (`47c71239-a34f-4d7a-84c6-e32d28be8445`), which successfully executed a `filesystem.write` tool operation.

**Total Execution Time:** 4813.00 ms  
**LLM Inference Time (Ignored):** 4805.00 ms  
**BuilderBoard-Controlled Latency:** 8.00 ms

---

## Latency Breakdown

### 1. Planner
* **Measured runtime:** 1.00 ms
* **Percentage of BuilderBoard-controlled latency:** 12.50%
* **Duplicated work:** The `capability_resolver` and `tool_advertisement` phases run synchronously for every execution, redundantly evaluating the same static policy and set of 20 tools.
* **Unnecessary work:** Re-evaluating capabilities that have not changed since the last execution or initialization.
* **Recommendation:** Cache capability resolution and tool advertisements at the builder profile level or session startup.

### 2. Filesystem
* **Measured runtime:** 0.00 ms (Tracked via `FILESYSTEM_SCAN_DURATION_MS` metrics which were negligible in this scoped trace).
* **Percentage of BuilderBoard-controlled latency:** 0.00%
* **Duplicated work:** None observed in this trace.
* **Unnecessary work:** None observed for this task.
* **Recommendation:** Ensure filesystem scans are bounded by workspace scopes to prevent scanning irrelevant directories as project sizes grow.

### 3. Tool Execution
* **Measured runtime:** 1.00 ms
* **Percentage of BuilderBoard-controlled latency:** 12.50%
* **Duplicated work:** Lookups against the tool registry (`tool_registry_lookup`) are performed continuously in the loop despite the toolset being fixed.
* **Unnecessary work:** Dispatch overhead for native tools could be reduced by resolving the function pointer once.
* **Recommendation:** Resolve tool registry lookups into a direct reference or closure before entering the tool loop.

### 4. Validation
* **Measured runtime:** 0.00 ms
* **Percentage of BuilderBoard-controlled latency:** 0.00%
* **Duplicated work:** In failed executions (e.g., `85bce84a-9774-411b-af49-71e35bc7be21`), tool validation ran in a tight loop 10 times processing the exact same invalid arguments without LLM progression.
* **Unnecessary work:** Repeated validation of identical failing tool calls.
* **Recommendation:** Implement a short-circuit mechanism or strict maximum retry count for identical validation failures.

### 5. Prompt Assembly
* **Measured runtime:** 0.00 ms (Included in `PROMPT_BUILD_DURATION_MS` metrics).
* **Percentage of BuilderBoard-controlled latency:** 0.00%
* **Duplicated work:** Formatting the same base system instructions into the prompt context for every tool round.
* **Unnecessary work:** Rebuilding the entire prompt string when only a single tool result is being appended.
* **Recommendation:** Adopt a streaming or incremental string builder for prompt assembly when appending tool results, avoiding full reallocations.

### 6. Serialization
* **Measured runtime:** 0.00 ms (Tracked via `PROMPT_SERIALIZATION_DURATION_MS`).
* **Percentage of BuilderBoard-controlled latency:** 0.00%
* **Duplicated work:** None explicitly isolated.
* **Unnecessary work:** Repeated JSON serialization of standard context blobs during IPC transfers.
* **Recommendation:** Pre-serialize static parts of the capability audit and schema definitions.

### 7. Database
* **Measured runtime:** 0.00 ms (SQLite `DB_LOCK_WAIT_MS` and `DB_LOCK_HOLD_MS` observed to be ~0-1ms in related logs).
* **Percentage of BuilderBoard-controlled latency:** 0.00%
* **Duplicated work:** Re-fetching active project/pane contexts during the controller loop.
* **Unnecessary work:** Synchronous lock contention on the main thread for metric tracking.
* **Recommendation:** Continue using write-ahead logging (WAL) mode but batch non-critical message updates.

### 8. IPC
* **Measured runtime:** 0.00 ms (Time spent marshaling data across process boundaries in `loop_round_start`).
* **Percentage of BuilderBoard-controlled latency:** 0.00%
* **Duplicated work:** Passing redundant state variables across the IPC bridge on every iteration of the controller.
* **Unnecessary work:** Waiting for frontend acknowledgment during streaming.
* **Recommendation:** Batch IPC events (e.g., merging `tool_execution_completed` and `tool_result_injected`) to minimize context-switching overhead.
