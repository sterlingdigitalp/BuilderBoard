# OPS-CON-001: Planner Convergence Test Suite

## Target Ledger Entry

**BB-0006** — Planner lacks convergence detection for repository-scale enumeration.

## Objective

Verify that the planner loop reliably converges (produces a final text response) under
three conditions:
1. **Immediate convergence** — LLM returns text with no tool calls on the first round.
2. **Multi-round convergence** — LLM returns tool calls for N rounds, then returns text
   on round N+1 (terminating normally).
3. **Non-convergence detection** — LLM always returns tool calls; the planner hits the
   hard `max_tool_rounds` limit and produces a fallthrough message.

## Pass / Fail Criteria

| Scenario | Pass | Fail |
|----------|------|------|
| Immediate (1-round) | Terminates in round 1 with correct text, no tool calls | Crashes, errors, or continues past round 1 |
| Multi-round (N+1) | Terminates in round N+1 (< max_rounds) with correct text | Hits max_rounds, produces wrong text, or duplicates tool calls |
| Non-convergence | Hits max_rounds (=10) with fallthrough message | Terminates early, panics, or produces incorrect message |

## Test Cases

### OPS-CON-001-A: Immediate Convergence

**Setup:** Fake OpenAI server returns a single SSE stream with text content and
`stop` finish reason. No `tool_calls` in any chunk.

**Input:** `Conversation` with one user message: `"Hello"`.

**Expected:**
- Planner makes exactly 1 provider request.
- Planner returns the text `"Hello from the assistant"`.
- Planner does NOT attempt any tool execution.
- `MissionMetrics` records `complete_success`.

### OPS-CON-001-B: Multi-Round Convergence (2 rounds)

**Setup:** Fake OpenAI server returns two SSE streams sequentially:
1. Round 1: tool call to `search({"pattern":"foo"})`, finish_reason=`tool_calls`.
2. Round 2: text `"Found it: foo is defined in bar.rs"`, finish_reason=`stop`.

**Input:** `Conversation` with one user message: `"Find foo"`.

**Expected:**
- Planner makes exactly 2 provider requests.
- Round 1: detects tool call `search`, simulates execution (adds tool result to
  conversation).
- Round 2: detects no tool calls, terminates with text containing `"Found it"`.
- `MissionMetrics.rounds < max_rounds` (i.e. 2 < 10).

### OPS-CON-001-C: Multi-Round Convergence (5 rounds)

Same as OPS-CON-001-B but with 5 tool-call rounds before the final text round.
Verifies convergence holds for longer chains.

### OPS-CON-001-D: Non-Convergence Detection

**Setup:** Fake OpenAI server returns 10 identical SSE streams, each with a
tool-call to `search({"pattern":"foo"})` and finish_reason=`tool_calls`.

**Input:** `Conversation` with one user message: `"Search forever"`.

**Expected:**
- Planner makes exactly 10 provider requests.
- Planner hits `max_rounds` (=10) and produces the fallthrough message:
  `"Maximum number of tool call rounds reached. Please refine your request."`
- `MissionMetrics` records `complete_failed("max_tool_rounds", ...)`.

### OPS-CON-001-E: Tool Call Argument Parsing

**Setup:** Fake OpenAI server returns SSE with a tool call containing complex
JSON arguments: `filesystem_write({"path":"/tmp/test.txt","content":"hello"})`.

**Input:** `Conversation` with one user message: `"Write a file"`.

**Expected:**
- Planner extracts exactly 1 tool call with `tool_name = "filesystem_write"`.
- Arguments parsed correctly: `path = "/tmp/test.txt"`, `content = "hello"`.
- No parse errors or truncation.

## Measurement

| Metric | Collection Point | Target |
|--------|-----------------|--------|
| Rounds used | Planner loop counter | < 10 for A/B/C; = 10 for D |
| Provider requests | Fake server request counter | = rounds used |
| Final text content | Planner return value | Correct per scenario |
| Tool calls extracted | Event collector count | Correct per scenario |
| Argument fidelity | Parsed arguments | Exact match to source |

## Implementation

See `src-tauri/tests/planner_convergence.rs` for runner implementations.

The tests use a `spawn_round_sequence_server` that accepts sequential TCP
connections and serves one pre-planned SSE body per connection, enabling
deterministic multi-round simulation without a real LLM.

## Relationship to Other Events

| Event | Relationship |
|-------|-------------|
| OPS-SLV-001 (2-tool chain) | Higher-level: tests convergence WITH real tool execution |
| OPS-SLV-002 (3+ chain) | Higher-level: tests convergence across real tool boundaries |
| OPS-SLV-003 (loop termination) | Replaced by this suite (more precise) |
| OPS-SLV-004 (no duplicates) | Quality gate for convergence detection |
