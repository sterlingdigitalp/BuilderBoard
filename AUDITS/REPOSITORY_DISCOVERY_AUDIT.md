# Repository Discovery Audit

This document provides an audit of every code path involved in repository understanding, specifically addressing why Builder T experiences failures during repository-scale discovery missions (e.g., `OPS-BRZ-007`).

## Executive Summary

Builder T's repository discovery failures (tracked as **BB-0001**) are a composite symptom of multiple underlying runtime deficiencies. The root causes are distributed across the **planner** and the lack of a dedicated **repository inventory** tool. The failures are **not** due to prompting or fundamental tool execution failures (as of the recent fixes for BB-0004 and BB-0005).

The primary failure mode is the exhaustion of the planner's hardcoded budget (`max_tool_rounds = 10`), causing missions to terminate prematurely with the error: *"Maximum number of tool call rounds reached."*

---

## Detailed Audit by Category

### 1. Tools: NOT THE ROOT CAUSE (Post-Stabilization)
Earlier versions experienced significant tool validation retries (BB-0002) caused by filesystem scope issues (BB-0004) and no-match search failures (BB-0005). 

- **Current State:** As recorded in `LEDGER_REVISION_2_SUMMARY.md`, BB-0004 and BB-0005 have been resolved. The underlying filesystem tools (`directory.list`, `filesystem.read`, `search.grep`, `search.glob`) now successfully execute and return valid results. 
- **Conclusion:** Tool failure is no longer the blocker for repository discovery.

### 2. Prompting: NOT THE ROOT CAUSE
The system correctly constructs tool advertisements and parses responses.

- **Tool Advertisement:** `build_tool_advertisement` in `src-tauri/src/execution/capability_resolver.rs` correctly builds a schema-driven prompt containing the available tools, their descriptions, and usage examples.
- **Parsing:** `parse_tool_calls` robustly extracts JSON tool calls from various Markdown fence formats (e.g., ````tool_call````, ````json````, and compact).
- **Conclusion:** The LLM receives correct instructions and its responses are parsed correctly. Prompting is working as designed.

### 3. Planner: PRIMARY CONTRIBUTOR (BB-0006, BB-0009)
The execution loop in `src-tauri/src/stream_execution.rs` manages multi-step tool workflows. This planner has two major design flaws that lead to budget exhaustion:

- **Missing Convergence Detection (BB-0006):** The planner lacks the ability to recognize when it has gathered sufficient evidence. It blindly loops until the LLM stops returning tool calls or the `max_tool_rounds` (hardcoded to 10) is hit. As seen in `OPS-CON-001`, the planner will repeatedly re-read files or issue redundant searches.
- **Inefficient Sequences (BB-0009):** The planner does not optimize its tool sequence. It operates sequentially rather than in parallel for discovery, quickly eating into the 10-round limit when faced with a large repository. 
- **Conclusion:** The planner's inability to logically terminate a search and its inefficient use of rounds directly lead to the `max_tool_rounds` failure.

### 4. Repository Inventory: CAPABILITY GAP (BB-0008)
The current toolset is too granular for high-level repository understanding.

- **Current Workflow:** To answer "count source files" or "discover project structure," the planner must invoke `directory.list` recursively or use `search.glob` multiple times. 
- **The Gap:** There is no single tool (e.g., `repository.inventory`) that provides a holistic overview (directory tree depth, file counts by type, root contents) in one round. 
- **Impact:** Because the planner is forced to compose high-level understanding from low-level primitives (`directory.list`), it invariably exceeds the 10-round budget on any non-trivial repository.
- **Conclusion:** The lack of a fast repository inventory capability acts as a strict bottleneck, practically guaranteeing budget exhaustion during repository-scale tasks.

---

## Final Verdict

Builder T's failures in repository discovery are caused by the combination of **Planner Inefficiencies (BB-0006, BB-0009)** and the lack of a **Repository Inventory (BB-0008)** capability. 

To resolve **BB-0001**, the runtime requires:
1. A new `repository.inventory` tool to collapse structure discovery into a single tool call.
2. A more intelligent planner that detects convergence to avoid redundant tool execution.