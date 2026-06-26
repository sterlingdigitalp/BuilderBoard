# Runtime Latency Report

This report identifies every source of runtime latency traced within the BuilderBoard runtime, ranked by estimated contribution to overall latency.

## 1. LLM Generation & Engine Latency
*Estimated Contribution: Highest (Seconds to Tens of Seconds)*
- **`llm_duration_ms`**: Total time spent generating tokens across all LLM calls in a mission.
- **`ENGINE_REQUEST_DURATION_MS`**: Time taken for a single request to the LLM engine to complete.
- **`ENGINE_STREAM_TOTAL_MS`**: Total time taken to stream the entire response from the LLM engine.

## 2. Tool Execution Latency
*Estimated Contribution: High (Seconds, highly dependent on the tool)*
- **`tool_duration_ms`**: Total time spent executing tools during a mission.
- **`first_tool_latency_ms`**: Time taken from mission start until the completion of the first tool call.

## 3. Mission Planning & Reasoning Latency
*Estimated Contribution: High (Seconds)*
- **`planning_duration_ms`**: Time spent by the agent planning and making decisions between tool invocations.

## 4. Time To First Token (TTFT)
*Estimated Contribution: Moderate (Hundreds of milliseconds to Seconds)*
- **`TTFT_MS`**: Time from request initiation until the first token is received from the LLM engine and processed.

## 5. Filesystem Operations Latency
*Estimated Contribution: Moderate (Tens to Hundreds of milliseconds, I/O bound)*
- **`FILESYSTEM_SCAN_DURATION_MS`**: Time spent scanning and reading the local filesystem (e.g., during context enrichment).

## 6. Prompt Engineering & Serialization Latency
*Estimated Contribution: Low (Milliseconds to Tens of milliseconds, CPU bound)*
- **`PROMPT_BUILD_DURATION_MS`**: Time spent assembling the context and formatting the prompt for the LLM.
- **`PROMPT_SERIALIZATION_DURATION_MS`**: Time spent serializing the prompt structures into JSON strings.
- **`PROMPT_INJECTION_SIZE`**: (Not a duration, but contributes to build/serialization time based on payload size).

## 7. Database Concurrency Overhead
*Estimated Contribution: Very Low (Sub-millisecond to a few milliseconds)*
- **`DB_LOCK_WAIT_MS`**: Time spent waiting to acquire a lock on the database.
- **`DB_LOCK_HOLD_MS`**: Time a database lock is actively held during operations.

## 8. IPC and Application Framework Overhead
*Estimated Contribution: Lowest (Microseconds to a few milliseconds)*
- **`TAURI_COMMAND_DURATION_MS`**: IPC communication overhead for invoking a Tauri backend command from the frontend.
- **`MAIN_THREAD_BLOCK_MS`**: Tracked block time of the main thread (should ideally be zero).

## Aggregated Mission Latency Metrics
*(These are sums of the above phases, not independent sources of latency)*
- **`total_duration_ms`**: Total time for a complete mission.
- **`TOTAL_REQUEST_DURATION_MS`**: Total time serving an entire request lifecycle.
