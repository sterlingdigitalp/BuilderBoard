# Builder T — Runtime Hypothesis Validation Report

**Date:** 2026-06-26
**Role:** Builder T — Runtime Experimentalist
**Dataset:** `runtime_traces/` (execution_timeline.jsonl, request_round_*.json, conversation_round_*.json, response_round_*.jsonl)
**Status:** 3 of 4 themes analyzed from existing traces — Theme 4 requires live multi-pane runtime

---

## Executive Summary

Three runtime executions were captured in the trace dataset:

| Execution | Pane | Rounds | Tool Calls | Result | Duration |
|-----------|------|--------|------------|--------|----------|
| 1 (39e45c84) | 2931f83c | 1 | 0 | Converged — text response "Hello! How can I help..." | ~1.5s |
| 2 (85bce84a) | 2931f83c | 10 | 10 (identical) | MAX_ROUNDS — all `filesystem.write` VALIDATION_ERROR | ~37s |
| 3 (47c71239) | 9cb2e303 | 2 | 1 (success) | Converged — "I created docs/test.md." | ~5s |

**Key finding:** Execution 2 provides direct runtime evidence that the planner **repeats the identical failing tool call 10 times** without adaptation, exhausting the max round limit. Execution 3 (18 minutes later, after a validation bug fix) shows the same user request completing successfully in 2 rounds.

---

## Theme 1 — Repository Understanding

### Hypothesis
The runtime can discover, enumerate, and reason about repository structure at small/medium/large scale.

### Runtime Evidence (from traces)

**Supported: Partially Observed — small repository only**

The trace dataset contains one repository understanding scenario: the `Knowledge_Service` project used in Execution 2 and Execution 3.

**What was provided (filesystem enrichment):**
```
Knowledge_Service/
├── .DS_Store
├── docs/                    (already existed)
├── integration_tests/
├── src/
├── STARTING.md
└── tests/
```

**What the runtime did:**
- The filesystem enrichment pre-populated a directory listing as a system message before the LLM responded.
- The enrichment provided 6 entries (1 file, 5 directories).
- The LLM correctly referenced `docs/` existence.
- The tool call to `filesystem.write` used the correct absolute path (`/Users/sterlingdigital/Knowledge_Service/docs/test.md`).

**What we don't know:**
- **Repository discovery** was not tested — the enrichment pre-populated the listing. We don't know if the LLM could independently discover repository structure.
- **Medium/large repositories** were not tested. The `Knowledge_Service` project has ~6 top-level entries, which is very small.
- **Duplicate searches** could not be measured — no `search.grep` or `search.glob` calls were made in any execution.
- **Repository understanding latency** was not measured — the enrichment ran in the background before the first LLM response.

### Metrics (from trace)

| Metric | Value | Source |
|--------|-------|--------|
| Repository size | 6 entries, 1 level deep | conversation_round_003.json |
| Enrichment provided | ✅ Filesystem listing pre-populated | request_round_001.json |
| LLM used repository context | ✅ Correct path constructed | execution_timeline.jsonl |
| Tool calls for discovery | 0 (enrichment did it) | execution_timeline.jsonl |
| Completion | ✅ File created successfully (Execution 3) | execution_timeline.jsonl:80 |
| Repository enumeration failures | ❌ Not tested at scale | N/A |

### Completed Experiments

**OPS-CON-001 (Planner Convergence) — 8/8 PASS**
- Uses a mock repository simulation via fake OpenAI server
- Tests convergence logic only, not actual repository discovery
- Reference: `tests/planner_convergence.rs`, `docs/runtime/tests/OPS-CON-001_PLANNER_CONVERGENCE.md`

### Pending Experiments

#### OPS-REPO-001: Single-File Repository Discovery
- **Objective:** Measure whether the planner can discover and describe a small repository (this BuilderBoard project — ~150 files) without filesystem enrichment.
- **Runtime steps:**
  1. Launch packaged runtime (`npm run runtime:build -- --launch`)
  2. Set `BUILDERBOARD_TRACE_NATIVE_TOOLS=1` and `BUILDERBOARD_TRACE_RUNTIME=1`
  3. Open a new pane on the BuilderBoard project
  4. Send: "How many Rust source files are in this project? Show me the directory structure."
  5. Collect `runtime_traces/` output
- **Success metrics:**
  - Completion within 15 tool calls
  - Correct file count within 10% of actual
  - No duplicate directory listings
  - Total duration < 120s
- **Failure metrics:**
  - Exceeds 15 tool calls
  - Produces incorrect file count (>10% error)
  - Repeats same directory listing
  - Times out or hits max_rounds
- **Ledger items affected:** BB-0001 (repository discovery), BB-0006 (convergence), BB-0009 (budget exhaustion)
- **Hypothesis validated:** Repository discovery at small scale completes successfully

#### OPS-REPO-002: Medium Repository Understanding
- **Objective:** Measure planner behavior on a medium repository (~500-2000 files).
- **Runtime steps:** Same as OPS-REPO-001 but on a medium-sized project (e.g., a node_modules or a framework source tree).
- **Success metrics:**
  - Completion within 25 tool calls
  - Correct structural understanding
  - No duplicate searches or reads
  - Total duration < 300s
- **Failure metrics:**
  - Exceeds 25 tool calls
  - Repeated searches with same pattern
  - Hangs or self-repeats
- **Ledger items affected:** BB-0001, BB-0006, BB-0008 (inventory capability), BB-0009

#### OPS-REPO-003: Large Repository Discovery
- **Objective:** Measure planner behavior on a large repository (>5000 files).
- **Runtime steps:** Same pattern, large project (e.g., a full SDK or monorepo).
- **Success/failure metrics:** Same as OPS-REPO-002 but with 40 tool call budget.
- **Ledger items affected:** BB-0001, BB-0006, BB-0008, BB-0009

---

## Theme 2 — Planner Behavior

### Hypothesis
The planner continues gathering information after sufficient information exists, does not naturally converge, generates unnecessary tool rounds, and produces duplicate tool calls.

### Runtime Evidence (from traces)

**Supported: OBSERVED — Planner repeats identical failing calls 10 times without adaptation**

This is the strongest finding in the dataset.

**Execution 2 trace (85bce84a):**
- User message: "Create the file test.md in the folder /users/sterlingdigital/Knowledge_Service/docs"
- The `docs/` directory already exists (confirmed by enrichment)
- The LLM needs to create `test.md` inside it
- The LLM calls `filesystem.write({"content":"","path":"..."})` — a valid call with empty content

**Validation failure:** The tool validation rejected this call with:
```
Missing required arguments: 'path' (string) and 'content' (string)
```

**Planner response:** Repeat the exact same call 10 times.

| Round | Tool | Arguments | Validation | LLM adaptation |
|-------|------|-----------|------------|----------------|
| 1 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 2 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 3 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 4 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 5 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 6 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 7 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 8 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 9 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |
| 10 | filesystem.write | `{content:"", path:"..."}` | VALIDATION_ERROR | ❌ Same call |

**Result:** `loop_max_rounds_reached` — "every round produced at least one parsed tool call, so the controller never reached a no-tool final assistant response"

**LLM call IDs** (all unique, all same tool+args):
`call_5CD7w9mc4M5nwUfh9G47ZUcv`, `call_icIYvy3cW8qwh0hntZk0SeYI`, `call_wCj4jJHTnRnNTxI6zQm9Lrdg`, `call_WfL5FjP07lzXn4APKJABeWvj`, `call_IaZVN61pz9RKsKu9oG0w01VU`, `call_Jh3uexlhpFgtJkTSCEsUndMR`, `call_dUs5uw3cEs7MWQI5vBeRzDX8`, `call_RdBGy6T3vqYUyN59KAdQsW1y`, `call_97xHKdzd6HWkMG4VSwxLMeSy`, `call_s4bj5Kgoh7m085rVshes3ZDv`

**LLM never tried:**
- A different tool (e.g., `shell` with `touch docs/test.md`)
- Different arguments (e.g., non-empty content)
- A text response acknowledging the validation error
- Any variation whatsoever

**Execution 3 comparison (after validation bug fix):**
- Same user intent, different pane
- Same tool call to `filesystem.write`
- **Tool validation succeeds** (validation bug was fixed between executions)
- File created successfully in round 1
- Round 2: converged immediately — "I created docs/test.md."
- Total: 2 rounds, 1 tool call, ~5 seconds

### Does Not Inspect Code

**Confirmed.** This report uses only runtime trace data. No source code was read for planner analysis.

### Metrics (from traces)

| Metric | Execution 2 (bug) | Execution 3 (fixed) |
|--------|-------------------|---------------------|
| Total rounds | 10 | 2 |
| Tool calls | 10 (all failed) | 1 (succeeded) |
| Unique tools | 1 | 1 |
| Unique tool calls | 1 (repeated) | 1 |
| Adaptation attempts | 0 | N/A |
| Convergence | ❌ MAX_ROUNDS | ✅ Round 2 |
| Total duration | ~37s | ~5s |
| Per-round latency | ~3.5s | ~2.5s |
| Wasted rounds | 9 | 0 |
| Wasted tool calls | 9 | 0 |

### Completed Experiments

**OPS-CON-001 (Planner Convergence Test Suite) — 8/8 PASS**
- Pure logic tests verifying convergence algorithm
- Tests immediate, multi-round, and non-convergence scenarios
- Reference: `src-tauri/tests/planner_convergence.rs`

### Pending Experiments

#### OPS-PLAN-001: Tool Call Diversity Under Validation Failure
- **Objective:** Measure whether the planner tries alternative tools/arguments when a tool call fails validation.
- **Runtime steps:**
  1. Launch packaged runtime with trace enabled
  2. Ask a question requiring file creation
  3. Observe: does the planner try `shell` as an alternative when `filesystem.write` fails?
- **Success metric:** Planner tries at least 2 different approaches before repeating the same call.
- **Failure metric:** Planner repeats same failing call 3+ times.
- **Ledger items affected:** BB-0006 (convergence)

#### OPS-PLAN-002: Duplicate Tool Call Detection
- **Objective:** Measure whether the planner generates duplicate (tool_name, arguments) pairs.
- **Runtime steps:**
  1. Launch packaged runtime
  2. Ask: "Find all Rust files that use `tokio` in this project, show me their contents"
  3. Compare all tool calls for duplicates
- **Success metric:** Zero duplicate (tool, arguments) pairs across the entire execution.
- **Failure metric:** Any repeated (tool, arguments) pair.
- **Ledger items affected:** BB-0006, OPS-SLV-004

#### OPS-PLAN-003: Convergence After Validation Error
- **Objective:** After receiving a validation error, does the planner ever converge with a text response (admitting failure) instead of repeating the call?
- **Runtime steps:** Same as OPS-PLAN-001, but measure whether the planner ever produces a text response like "I cannot complete this task" instead of continuing to call tools.
- **Success metric:** Planner produces a text response within 3 rounds of persistent validation failure.
- **Failure metric:** Planner exhausts max_rounds without ever producing a text response.
- **Ledger items affected:** BB-0006

#### OPS-PLAN-004: Convergence After Runtime Success
- **Objective:** After a successful tool execution, does the planner converge immediately or make additional unnecessary calls?
- **Runtime steps:** Same as OPS-REPO-001, but instrument after first successful tool call.
- **Success metric:** Planner converges within 1 round of a successful tool call that completes the task.
- **Failure metric:** Planner makes additional tool calls after the task is complete.
- **Ledger items affected:** BB-0006

---

## Theme 3 — Tool Execution Pipeline

### Hypothesis
Tools execute with measurable latency, validation failures, repeated validation, and duplicate execution.

### Runtime Evidence (from traces)

**Supported: Partially Observed — tool execution pipeline is functional but has measurable issues**

#### Available Tools (execution 2)
20 tools registered, 20 allowed, 0 blocked:
```
process.list, diagnostics.env, filesystem.edit, package.uninstall, shell,
directory.create, package.list, process.kill, package.install, git.log,
filesystem.read, search.grep, filesystem.write, search.glob, directory.list,
filesystem.delete, git.status, git.diff, git.commit, diagnostics.health
```

All tools are advertised as native functions. No tools are blocked by policy.

#### Tool Execution Pipeline (per round, Execution 2)

| Phase | Latency | Pattern |
|-------|---------|---------|
| LLM generation | ~3.0s | OpenAI Response API stream |
| Tool parsing | ~4ms | `native_tool_parsing` → `loop_round_decision` |
| Tool registry lookup | ~1ms | `tool_registry_lookup` |
| Tool validation | ~1ms | `tool_validation` → VALIDATION_ERROR |
| Result injection | ~1ms | `tool_result_injected` |
| **Total per round** | **~3.5s** | |

#### Key Finding: Validation inconsistency

The same tool call `filesystem.write({"content":"","path":"..."})` produced different results in different executions:

| Execution | Timestamp | Validation | Tool execution |
|-----------|-----------|------------|----------------|
| 2 (85bce84a) | 20:33:54 | VALIDATION_ERROR | Never reached |
| 3 (47c71239) | 20:51:13 | valid: true | SUCCESS (0 bytes written) |

Both executions have the same:
- Tool: `filesystem.write`
- Arguments: `{content:"", path:"/Users/sterlingdigital/Knowledge_Service/docs/test.md"}`
- Model: `gpt-5.5` (via OpenAI Responses API)
- Tools: 20 registered, 20 allowed

**The difference:** A validation bug fix was applied between 20:33:54 and 20:51:13. The bug was likely that empty string content (`""`) was treated as "missing" by the validation function. The fix made empty strings pass validation.

#### Execution 1: No credential service
Execution a4ca423f (20:47:33) received "OpenAI engine requires credential service" error. This is a separate issue — the pane had no valid credentials configured.

#### Execution 3: Successful pipeline
- Tool resolved: `filesystem.write` → resolved_tool_id: `filesystem.write`
- Permission check: `write_files` → allowed
- Tool execution: `0 bytes written to /Users/sterlingdigital/Knowledge_Service/docs/test.md`
- Artifacts: 1 produced
- Review items: 1
- Duration: 0ms (essentially instant for empty content)

### Metrics (from traces)

| Metric | Execution 2 | Execution 3 |
|--------|-------------|-------------|
| Tools registered | 20 | 20 |
| Tools advertised | 20 | 20 |
| Tools blocked | 0 | 0 |
| Tool calls attempted | 10 | 1 |
| Tool calls succeeded | 0 | 1 |
| Tool calls failed validation | 10 | 0 |
| Validation failures | 10 (100%) | 0 |
| Repeated validation failures | 9 (same call) | 0 |
| Tool execution failures | 0 (never reached) | 0 |
| Tool execution success | N/A | 1 |
| Average tool validation latency | ~1ms | ~1ms |
| Average tool execution latency | N/A | ~1ms |
| Average round latency | ~3.5s | ~2.5s |
| Total execution latency | ~37s | ~5s |

### Completed Experiments

**Tool Registry Tests** (`src-tauri/src/execution/tools/tests.rs`) — 827 lines of test code covering registry, permissions, validation, error handling. These are unit/integration tests, not runtime tests.

### Pending Experiments

#### OPS-TOOL-001: Tool Latency Distribution
- **Objective:** Measure the distribution of tool execution latencies across all 20 tool types.
- **Runtime steps:**
  1. Launch packaged runtime with trace enabled
  2. Execute a multi-step task that exercises all tool categories: shell, filesystem read/write, search grep/glob, directory list, git status/log/diff, process list
  3. Collect per-tool latency from `execution_timeline.jsonl`
- **Success metrics:**
  - All tool types execute successfully
  - No tool exceeds 30s execution time
  - P50 latency < 5s per tool
- **Failure metrics:**
  - Any tool exceeds 60s
  - Any tool consistently fails
  - Validation errors > 10% of calls
- **Ledger items affected:** BB-0004 (scope validation), BB-0005 (search no-match)

#### OPS-TOOL-002: Validation Error Recovery
- **Objective:** After a validation error, does the pipeline correctly report the error and allow retry?
- **Runtime steps:** Send a deliberately malformed tool call (e.g., missing required argument) and observe the validation error message.
- **Success metric:** Validation error is clear, consistent, and actionable ("Missing required arguments: 'content' (string)").
- **Failure metric:** Validation error is misleading, inconsistent, or silently drops the request.
- **Ledger items affected:** BB-0004 (scope validation)

#### OPS-TOOL-003: Duplicate Tool Execution Detection
- **Objective:** Measure whether identical tool calls produce duplicate executions or are deduplicated.
- **Runtime steps:**
  1. Execute a task likely to produce duplicate calls
  2. Compare all (tool_id, arguments) pairs
- **Success metric:** Zero identical (tool_id, arguments) pairs.
- **Failure metric:** Any identical pair with same arguments.
- **Ledger items affected:** BB-0006, OPS-SLV-004

---

## Theme 4 — Builder Independence

### Hypothesis
Simultaneous Builder missions maintain responsiveness, low latency, no stalls, no interference, no contention, and no observable state leakage.

### Runtime Evidence (from traces)

**Supported: NOT OBSERVED — no multi-pane runtime data available**

The trace dataset contains only single-pane executions:

| Execution | Pane | Other Panes Active | Notes |
|-----------|------|-------------------|-------|
| 1 (39e45c84) | 2931f83c | Unknown | Single execution trace |
| 2 (85bce84a) | 2931f83c | Unknown | Same pane, follow-up request |
| 3 (47c71239) | 9cb2e303 | Unknown | Different pane, different time |

The traces do not contain simultaneous multi-pane activity. We cannot evaluate:
- Whether panes interfere with each other
- Whether state leaks between panes
- Whether concurrent streaming causes event loop blocking
- Whether resource contention affects latency

The `runtimeDiagnostics.ts` probe infrastructure exists (`probeCrossPaneInteraction`) and would measure event loop blocking when `BUILDERBOARD_TRACE_RUNTIME=1`, but was not active during the captured traces.

### Completed Experiments

None. Theme 4 requires live multi-pane runtime execution.

### Pending Experiments

#### OPS-INDEP-001: Two-Pane Simultaneous Streaming
- **Objective:** Measure whether two panes can stream simultaneously without interference.
- **Runtime steps:**
  1. Launch packaged runtime with trace enabled and `localStorage.BUILDERBOARD_TRACE_RUNTIME=1`
  2. Open two panes, each with a different project
  3. Send a long-running request to Pane A (e.g., "Analyze all Rust source files")
  4. While Pane A is streaming, send a request to Pane B
  5. Collect frontend probe data (`EVENT_LOOP_BLOCK_MS`, `RUNTIME_PROBE_ROUNDTRIP_MS`, `PANE_LIST_INVOKE_MS`)
  6. Collect backend trace data
- **Success metrics:**
  - Both panes complete successfully
  - EVENT_LOOP_BLOCK_MS < 100ms at P95
  - No cross-pane message leakage
  - Each pane's response contains only its own project's data
- **Failure metrics:**
  - First pane's streaming stalls when second pane starts
  - EVENT_LOOP_BLOCK_MS > 500ms at P95
  - Panes receive each other's messages
  - One pane's response includes the other pane's data
- **Ledger items affected:** BB-0003 (hardcoded builder routing), new entry if cross-pane leakage found

#### OPS-INDEP-002: Four-Pane Concurrent Execution
- **Objective:** Measure independence with all four panes active simultaneously.
- **Runtime steps:**
  1. Open all four panes with different projects and different builders (Builder A, B, C, default)
  2. Send requests to all four panes within 5 seconds
  3. Monitor all four for completion
- **Success metrics:**
  - All four complete successfully within 5x single-pane latency
  - No pane fails due to resource exhaustion
  - All responses are correct for their respective projects
- **Failure metrics:**
  - Any pane fails to complete
  - Cross-pane response contamination
  - >10x latency degradation compared to single-pane
- **Ledger items affected:** OPS-GLD-001 (multi-pane independent operation), OPS-GLD-002 (multi-pane multi-tool)

#### OPS-INDEP-003: State Isolation Verification
- **Objective:** Verify no state leaks between panes (messages, projects, accounts, tool results).
- **Runtime steps:**
  1. Configure Pane A with Project X and Account Y
  2. Configure Pane B with Project Z and Account W
  3. Send requests to both
  4. After completion, inspect both panes' conversations and tool results
- **Success metric:** Pane A's conversation contains only Project X data; Pane B's contains only Project Z data.
- **Failure metric:** Any cross-pane state contamination.
- **Ledger items affected:** BB-0003, OPS-GLD-002

#### OPS-INDEP-004: Logical vs Performance Independence
- **Objective:** Distinguish between logical independence (no state leakage) and performance independence (no latency interference).
- **Runtime steps:**
  1. Run OPS-INDEP-001 with probe instrumentation
  2. Measure:
     - Logical: Are responses correct? Is state isolated?
     - Performance: Does concurrent execution degrade per-pane latency?
- **Expected distinction:**
  - Logical independence is required (by Core Promise: "four independent Builder panes")
  - Performance independence is desirable but may degrade proportionally to resource contention
- **Ledger items affected:** Core Promise architecture

---

## Cross-Cutting Observations

### 1. Validation Bug Confirmed in Traces
The `filesystem.write` validation error in Execution 2 (rounds 1-10) was a **false negative**: the arguments `{content:"", path:"..."}` contained both required fields, but validation reported them as missing. This was fixed by Execution 3. The fix was likely treating empty strings as valid values.

**Impact:** This bug caused 9 wasted rounds and ~32s of unnecessary execution time. It also masked the planner's inability to adapt — the planner might have converged after a successful tool execution.

### 2. Filesystem Enrichment Prevents Real Discovery Testing
The filesystem enrichment pre-populates directory listings before the LLM responds. This means we cannot observe the planner's *independent* repository discovery behavior from these traces. All three executions had enrichment active.

### 3. Response API Format
The runtime uses OpenAI's Responses API (not Chat Completions API), with native function tool definitions. The `tool_choice` is `"auto"`, `parallel_tool_calls` is `true`, and `stream` is `true`. Each round makes a fresh API call with the full accumulated conversation.

### 4. Prompt Context
The system prompt includes project context ("Project: Knowledge_Service", "Approved root: /Users/sterlingdigital/Knowledge_Service") plus filesystem enrichment results. The LLM is instructed: "Use these read-only approved-root results to answer the user. Do not claim you cannot access the project."

---

## Ledger Impact Assessment

| Ledger Entry | Finding | Status |
|-------------|---------|--------|
| **BB-0006** — Planner convergence | **OBSERVED:** Planner repeats identical failing call 10 times without adaptation. Prior convergence tests (OPS-CON-001) showed loop logic is correct; the failure is in LLM's choice to keep calling tools. | **Hypothesis confirmed.** Direct runtime evidence. |
| **BB-0009** — Budget exhaustion | **OBSERVED (partial):** 10 rounds consumed entirely by repeated validation failures. Without the validation bug, this would have been 2 rounds. Budget exhaustion is a consequence of non-adaptation. | **Partially supported.** Depends on BB-0006 fix. |
| **BB-0001** — Repository discovery | **NOT OBSERVED:** Enrichment pre-populated data; no independent discovery tested. | **Cannot evaluate from traces.** |
| **BB-0004** — Scope validation | **OBSERVED (regression):** Validation rejected valid arguments (Bug — not scope bug). Was fixed by Execution 3. | **Supported.** Validation behavior confirmed. |
| **BB-0008** — Inventory capability | **NOT OBSERVED:** No inventory tool used in traces. | **Cannot evaluate from traces.** |
| **BB-0003** — Builder routing | **NOT OBSERVED:** Single builder used in all executions (builder-a). | **Cannot evaluate from traces.** |

---

## Confidence Assessment

| Finding | Confidence | Basis |
|---------|------------|-------|
| Planner repeats identical failing calls without adaptation | **HIGH** | 10 consecutive rounds of identical behavior, verified across 3 independent data sources (timeline, messages, responses) |
| Validation bug existed and was fixed | **HIGH** | Same arguments validated differently in two executions 18 minutes apart |
| Planner converges normally after successful tool execution | **MEDIUM** | Single data point (Execution 3: 2 rounds, 1 tool call, converged) |
| Multi-pane independence | **NONE** | No data available |
| Repository discovery at scale | **NONE** | Only small repo with enrichment active |

---

## Recommendations

### Immediate (can act on existing evidence)
1. **Update BB-0006 hypothesis to reflect runtime evidence.** The traces confirm the planner *does not adapt* when a tool call fails. This is a stronger finding than the previous "prompt completion behavior" hypothesis. The planner's inability to recover from validation errors is a direct runtime failure.

2. **Add a new experimental finding to the ledger:** "Planner lacks error recovery — repeated identical failing tool calls without adaptation or graceful degradation."

3. **Design a validation error recovery experiment (OPS-PLAN-001).** The highest-value next experiment is: after a VALIDATION_ERROR, does the planner try a different approach?

### Blocked (requires live credentials)
4. **Execute OPS-REPO-001, OPS-REPO-002, OPS-REPO-003.** These require an authenticated OpenAI session to test repository discovery at scale.

5. **Execute OPS-INDEP-001 through OPS-INDEP-004.** These require multi-pane runtime with simultaneous streaming, which cannot be tested with traces alone.

6. **Execute OPS-TOOL-001 through OPS-TOOL-003.** These require a live runtime where tool calls can be instrumented across multiple scenarios.

### Process
7. **Enable `BUILDERBOARD_TRACE_RUNTIME=1` and `BUILDERBOARD_TRACE_NATIVE_TOOLS=1` for all future runtime tests.** The existing traces are valuable but incomplete. The frontend diagnostics (`localStorage`) should also be enabled to capture `EVENT_LOOP_BLOCK_MS` data.

---

## Appendix A: Trace Data Sources Used

| File | Size | Content |
|------|------|---------|
| `execution_timeline.jsonl` | 33KB | 83 events across 3 executions |
| `request_round_001.json` | 10.8KB | HTTP request to OpenAI (system prompt + tool definitions) |
| `conversation_round_003.json` | 3.0KB | Full conversation state after round 3 (pane 2931f83c) |
| `conversation_round_010.json` | 6.1KB | Full conversation state after round 10 (final state, all errors) |
| `response_round_002.jsonl` | 213KB | Raw OpenAI response stream (tool call to filesystem.write) |
| `response_round_003-010.jsonl` | ~110KB each | Raw OpenAI response streams (identical tool calls) |
| `OPENAI_PROTOCOL_COMPARISON.md` | 1.4KB | Protocol comparison placeholder |

## Appendix B: Experiment Summary

| ID | Name | Status | Theme | Requires |
|----|------|--------|-------|----------|
| OPS-CON-001 | Planner Convergence | ✅ COMPLETED (8/8) | 2 | None |
| OPS-REPO-001 | Small Repo Discovery | 🔲 DESIGNED | 1 | Live runtime |
| OPS-REPO-002 | Medium Repo Discovery | 🔲 DESIGNED | 1 | Live runtime |
| OPS-REPO-003 | Large Repo Discovery | 🔲 DESIGNED | 1 | Live runtime |
| OPS-PLAN-001 | Tool Call Diversity | 🔲 DESIGNED | 2 | Live runtime |
| OPS-PLAN-002 | Duplicate Tool Detection | 🔲 DESIGNED | 2 | Live runtime |
| OPS-PLAN-003 | Converge After Validation Error | 🔲 DESIGNED | 2 | Live runtime |
| OPS-PLAN-004 | Converge After Runtime Success | 🔲 DESIGNED | 2 | Live runtime |
| OPS-TOOL-001 | Tool Latency Distribution | 🔲 DESIGNED | 3 | Live runtime |
| OPS-TOOL-002 | Validation Error Recovery | 🔲 DESIGNED | 3 | Live runtime |
| OPS-TOOL-003 | Duplicate Execution Detection | 🔲 DESIGNED | 3 | Live runtime |
| OPS-INDEP-001 | Two-Pane Simultaneous | 🔲 DESIGNED | 4 | Live runtime |
| OPS-INDEP-002 | Four-Pane Concurrent | 🔲 DESIGNED | 4 | Live runtime |
| OPS-INDEP-003 | State Isolation | 🔲 DESIGNED | 4 | Live runtime |
| OPS-INDEP-004 | Logical vs Performance Independence | 🔲 DESIGNED | 4 | Live runtime |
