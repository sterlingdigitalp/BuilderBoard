# Phase 0 — Core Runtime Olympics

## Philosophy

### Why Phase 0 Exists

Phase 0 exists because BuilderBoard must work before it can be improved.

Traditional software projects measure progress by implementation milestones: "80% of features done," "tests passing," "architecture complete." These measures are seductive because they are easy to produce, but they are dangerous because they are uncorrelated with actual user outcomes.

BuilderBoard rejects this model. A feature that compiles but cannot be used is not a feature. An architecture that is elegant but does not deliver reliable runtime behavior is technical debt.

Phase 0 codifies the minimum behavior BuilderBoard must demonstrate before any additional product functionality is considered complete. It answers one question:

> Can a real user successfully accomplish real engineering work using the running application?

### Runtime First

Runtime First means:

- **Runtime behavior is the primary measure of software quality.** Not test coverage. Not code quality. Not architectural purity.
- **Every feature is measured by whether it works in the running application.** Not by whether it compiles or passes unit tests.
- **Certification is continuous.** The runtime is tested every release. There is no separate "QA phase."
- **Implementation is fungible.** If the runtime works, the implementation can be changed freely. If the runtime breaks, no implementation quality matters.

### Core Promise

> BuilderBoard exists to allow a single user to accomplish everything possible with one AI software engineering assistant simultaneously across four independent Builder panes. Until this works reliably, BuilderBoard is not considered complete.

This is the single standard against which all work is measured. The Core Promise is permanently documented in `CORE_PROMISE.md` and governed by the twelve Engineering Laws in `ENGINEERING_LAWS.md`.

---

## Olympic Structure

Phase 0 defines four medal tiers. Each tier represents a higher standard of runtime reliability and capability.

### Bronze — Single Pane, Single Tool

The application starts. A user can open a Builder pane, send a message, and receive a response. A single tool call (any category) executes and returns results without error.

Minimum acceptable for any use.

### Silver — Single Pane, Multi-Tool

A user can send a complex request that requires chaining multiple tool calls (e.g., "find TODO comments then show git status"). The runtime correctly resolves, advertises, chains, and terminates without infinite loops or errors.

Minimum acceptable for productive use.

### Gold — Multi-Pane, Multi-Tool

A user can operate four Builder panes simultaneously, each performing independent multi-tool workflows. Panes do not interfere with each other. Each pane terminates correctly.

Minimum acceptable for the Core Promise.

### Future Levels — Beyond Phase 0

- **Platinum**: Concurrent long-running workflows across panes with progress reporting.
- **Diamond**: Cross-pane coordination — tools in one pane can reference results from another.
- **Legendary**: Self-healing runtime — if a pane fails, the runtime can recover and retry.

These are out of scope for Phase 0.

---

## Core Runtime Events

Each Olympic event is a single, testable runtime behavior. Events are structured so new ones can be added by copying the template.

### Living Capability Definition

The Olympics are a **living capability definition**, not a fixed checklist. They grow with BuilderBoard:

- **New capabilities** require new events. Before a feature is complete, its Olympic event must be added and passed.
- **Existing events** can be refined as pass criteria become clearer or latency targets become more demanding.
- **No event is ever removed.** Events may only be retired (moved to a retired section) if the feature they test is intentionally removed. Retired events are preserved for historical traceability.
- **Certification is capability-based, not count-based.** Passing more events means higher capability, not "more completion." The certification score reflects the runtime's capability to handle real user workflows.

This means the Olympics will never be "finished." As BuilderBoard's capabilities expand, the Olympics expand with them. The certification bar rises over time — this is intentional.

### Discovery vs Regression Olympics

Runtime Olympics serve two distinct purposes:

**Discovery Olympics** are open-ended explorations of runtime behavior. Builder T designs and executes them to find new failures, challenge assumptions, and uncover issues not yet in the ledger. Discovery events may produce unexpected results — that is their purpose. When a discovery event reveals a failure, a new ledger entry is created.

**Regression Olympics** are deterministic re-executions of specific events linked to a particular ledger entry. They verify that a fix has been correctly applied and that no regressions were introduced. Regression events have well-defined pass criteria and expected outcomes. They are executed after every implementation.

| Dimension | Discovery Olympics | Regression Olympics |
|-----------|-------------------|-------------------|
| **Purpose** | Find new failures | Verify existing fixes |
| **Lead** | Builder T | Builder T |
| **Validation** | Not required | Builder V |
| **Pass criteria** | Exploratory | Deterministic |
| **Ledger linkage** | Creates new entry | References existing entry |
| **Frequency** | Continuous | After every implementation |

### Structure of an Event

Each event contains:

| Field | Description |
|-------|-------------|
| **Event ID** | Unique identifier (e.g., `OPS-BRZ-001`) |
| **Name** | Human-readable name |
| **Mission** | What the user is trying to accomplish |
| **Expected User Outcome** | What the user sees and experiences |
| **Pass Criteria** | Measurable conditions that must be true |
| **Latency Target** | Maximum acceptable time from request to response |
| **Metrics Collected** | Data recorded during execution |
| **Regression Linkage** | Links to any related regression test or prior issue |
| **Certification Weight** | Contribution to overall certification score |

---

## Bronze Events

### OPS-BRZ-001: Application Launch

- **Mission**: Start the application.
- **Expected User Outcome**: The application window appears. The user sees a workspace with at least one Builder pane available.
- **Pass Criteria**: Application launches without crash. Main window renders within 10 seconds.
- **Latency Target**: 10 seconds to interactive.
- **Metrics Collected**: cold_start_ms, warm_start_ms, render_complete_ms.
- **Regression Linkage**: N/A (foundational).
- **Certification Weight**: 5%.

### OPS-BRZ-002: Basic Chat

- **Mission**: Send a message and receive a response.
- **Expected User Outcome**: User types a simple question into a Builder pane. The assistant responds naturally. No tool calls needed.
- **Pass Criteria**: Response appears in the pane within 30 seconds. Response is coherent and related to the question.
- **Latency Target**: TTFT < 5 seconds. Full response < 30 seconds.
- **Metrics Collected**: ttft_ms, total_response_ms, response_length_tokens.
- **Regression Linkage**: N/A (foundational).
- **Certification Weight**: 10%.

### OPS-BRZ-003: Tool Discovery

- **Mission**: Builder pane knows about available tools.
- **Expected User Outcome**: The assistant can discover and reference tools in its responses.
- **Pass Criteria**: Tool advertisement is injected as a system message. The assistant's first response references at least one tool by name.
- **Latency Target**: N/A (measured once at conversation start).
- **Metrics Collected**: tools_advertised_count, tool_reference_found.
- **Regression Linkage**: Phase 9A.4 capability advertisement fix.
- **Certification Weight**: 5%.

### OPS-BRZ-004: Single Tool Execution — Read File

- **Mission**: Ask the assistant to read a file.
- **Expected User Outcome**: User asks "read src/main.rs" or equivalent. The assistant calls `filesystem.read` and returns the file contents.
- **Pass Criteria**: Tool executes. No error. File contents appear in the response. Loop terminates within 2 rounds.
- **Latency Target**: Tool execution < 10 seconds. Total < 40 seconds.
- **Metrics Collected**: tool_rounds, tool_execution_ms, tool_success.
- **Regression Linkage**: Phase 9A.5 tool loop fix.
- **Certification Weight**: 10%.

### OPS-BRZ-005: Single Tool Execution — Shell Command

- **Mission**: Ask the assistant to run a shell command.
- **Expected User Outcome**: User asks "run `ls -la`" or equivalent. The assistant calls `shell` and returns the command output.
- **Pass Criteria**: Tool executes. Command output appears. Loop terminates within 2 rounds.
- **Latency Target**: Tool execution < 15 seconds. Total < 45 seconds.
- **Metrics Collected**: tool_rounds, tool_execution_ms, exit_code.
- **Regression Linkage**: Phase 9A.2 shell tool tests.
- **Certification Weight**: 10%.

### OPS-BRZ-006: Single Tool Execution — Git Status

- **Mission**: Ask the assistant to show git status.
- **Expected User Outcome**: User asks "show git status." The assistant calls `git.status` and returns the status output.
- **Pass Criteria**: Tool executes. Git output appears. Loop terminates within 2 rounds.
- **Latency Target**: Tool execution < 10 seconds. Total < 40 seconds.
- **Metrics Collected**: tool_rounds, tool_execution_ms, tool_success.
- **Regression Linkage**: Phase 9A.2 git tool tests.
- **Certification Weight**: 10%.

### OPS-BRZ-007: Single Tool Execution — Search

- **Mission**: Ask the assistant to find TODO comments.
- **Expected User Outcome**: User asks "find TODO comments." The assistant calls `search.grep` and returns matching lines.
- **Pass Criteria**: Tool executes. Results appear. Loop terminates within 2 rounds.
- **Latency Target**: Tool execution < 15 seconds. Total < 45 seconds.
- **Metrics Collected**: tool_rounds, result_count, tool_execution_ms.
- **Regression Linkage**: Phase 9A.2 search tool tests.
- **Certification Weight**: 10%.

### OPS-BRZ-008: Tool Error Handling — Unknown Tool

- **Mission**: Ask the assistant to use a non-existent tool.
- **Expected User Outcome**: The assistant should recognize the tool doesn't exist and respond with an error or explanation. The runtime should not crash.
- **Pass Criteria**: Runtime does not crash. Assistant responds with an error message or corrective suggestion. Loop terminates.
- **Latency Target**: Total < 30 seconds.
- **Metrics Collected**: tool_rounds, error_type, loop_terminated.
- **Regression Linkage**: Phase 9A.5 error handling paths.
- **Certification Weight**: 5%.

### OPS-BRZ-009: Tool Error Handling — Invalid Arguments

- **Mission**: Ask the assistant to read a file without specifying a path.
- **Expected User Outcome**: The assistant should recognize validation failed and respond with a request for the missing argument.
- **Pass Criteria**: Runtime does not crash. Assistant responds with validation error. Loop terminates.
- **Latency Target**: Total < 30 seconds.
- **Metrics Collected**: tool_rounds, error_type, loop_terminated.
- **Regression Linkage**: Phase 9A.5 validation error handling.
- **Certification Weight**: 5%.

---

## Silver Events

### OPS-SLV-001: Multi-Tool Chaining (2 tools)

- **Mission**: Ask the assistant to perform two sequential operations, e.g., "find TODO comments then show git status."
- **Expected User Outcome**: The assistant executes `search.grep`, receives result, then executes `git.status`, and synthesizes a final response combining both results.
- **Pass Criteria**: Two distinct tool calls. Both succeed. Final response references both results. Loop terminates.
- **Latency Target**: Total < 60 seconds.
- **Metrics Collected**: tool_rounds, tools_chained, all_succeeded, loop_terminated.
- **Regression Linkage**: Phase 9A.5 multi-tool chaining.
- **Certification Weight**: 10%.

### OPS-SLV-002: Multi-Tool Chaining (3+ tools)

- **Mission**: Ask the assistant to perform three or more sequential operations, e.g., "list the src directory, read the main file, and show git log."
- **Expected User Outcome**: The assistant chains three or more tool calls in sequence. Each receives a result. Final response synthesizes all information.
- **Pass Criteria**: Three or more distinct tool calls. All succeed. Loop terminates within max rounds. No duplicate tool calls.
- **Latency Target**: Total < 90 seconds.
- **Metrics Collected**: tool_rounds, tools_chained, all_succeeded, duplicates_detected, loop_terminated.
- **Regression Linkage**: Phase 9A.5 tool loop fix.
- **Certification Weight**: 10%.

### OPS-SLV-003: Loop Termination on Success

- **Mission**: Complete a multi-tool request and verify the loop terminates immediately.
- **Expected User Outcome**: After the last tool result is injected, the assistant produces a final response without further tool calls. No "Maximum number of tool call rounds reached."
- **Pass Criteria**: Loop terminates at round < 10. Final response is not the fallthrough message.
- **Latency Target**: Total < 60 seconds.
- **Metrics Collected**: final_round, fallthrough_triggered, total_rounds.
- **Regression Linkage**: Phase 9A.5 root cause fix.
- **Certification Weight**: 10%.

### OPS-SLV-004: No Duplicate Tool Calls

- **Mission**: Verify the runtime does not repeat identical tool calls.
- **Expected User Outcome**: The assistant calls each tool at most once per logical operation. No identical tool+arguments pair appears in consecutive rounds.
- **Pass Criteria**: No round contains the same (tool_name, arguments) as a previous round. (Only enforceable post-hoc via log inspection.)
- **Latency Target**: N/A (observability).
- **Metrics Collected**: round_arguments_duplicates, duplicate_free.
- **Regression Linkage**: Phase 9A.5 duplicate call analysis.
- **Certification Weight**: 5%.

---

## Gold Events

### OPS-GLD-001: Multi-Pane Independent Operation

- **Mission**: Open four Builder panes. Send different requests to each simultaneously.
- **Expected User Outcome**: All four panes start processing. Each pane completes independently. No pane blocks another. All responses are correct.
- **Pass Criteria**: Four panes started. All four complete. Each result is correct for its specific request. No cross-pane interference.
- **Latency Target**: Total < 120 seconds (all panes complete).
- **Metrics Collected**: pane_count, panes_completed, max_latency_ms, any_interference.
- **Regression Linkage**: Core Promise requirement.
- **Certification Weight**: 10%.

### OPS-GLD-002: Multi-Pane Multi-Tool

- **Mission**: Four panes, each performing a multi-tool workflow simultaneously.
- **Expected User Outcome**: Each pane independently chains tools, receives results, and produces a final response. All panes complete without conflict.
- **Pass Criteria**: Four panes, each executing 2+ tool calls. All tools succeed. All panes terminate. No cross-pane interaction.
- **Latency Target**: Total < 180 seconds.
- **Metrics Collected**: pane_count, panes_completed, tools_per_pane, all_succeeded, any_interference.
- **Regression Linkage**: Core Promise requirement.
- **Certification Weight**: 10%.

---

## Runtime Certification Maturity Model

Certification is not pass/fail — it is a maturity model with five levels. Each level represents a measurable threshold of runtime capability.

| Level | Name | Requirement | Meaning |
|-------|------|-------------|---------|
| **Not Certified** | — | No formal certification executed | Runtime may or may not work; no evidence |
| **Bronze Certified** | Single Pane, Single Tool | All Bronze events pass (≥70%) | Application launches, basic chat works, single tool calls execute. Minimum acceptable for any use. |
| **Silver Certified** | Single Pane, Multi-Tool | All Bronze + Silver events pass (≥95%) | Multi-tool chaining, loop termination, no duplicates. Minimum acceptable for productive use. |
| **Gold Certified** | Multi-Pane, Multi-Tool | All Bronze + Silver + Gold events pass (≥115%) | Four independent panes, each with multi-tool capability. Core Promise achieved. |
| **Production Certified** | Stable & Monitored | Sustained Gold certification over 30+ days with no regressions | Runtime stable in production use. Requires continuous monitoring and automated certification. |

### Scoring

Total certification score = weighted sum of passed events.

| Tier | Maximum Score |
|------|---------------|
| Bronze | 70% |
| Silver | 25% |
| Gold | 20% |
| **Total** | **115%** (some weight overlap for multi-tier scoring) |

Minimum Bronze score: 70%.
Minimum Silver score: 95%.
Minimum Gold score: 115%.

### Certification Boundaries

A certification level is only valid if all events at that level AND all lower levels pass. You cannot skip levels:
- Silver requires Bronze first.
- Gold requires Silver first.
- Production requires sustained Gold.

### Level Transitions

- **Promotion**: When all requirements for a higher level are met, the runtime is promoted.
- **Demotion**: If a regression causes the runtime to fall below a level's threshold, the runtime is demoted to the next lower level.
- **Recertification**: After demotion, the runtime must be recertified at the lower level before climbing again.

---

## Adding New Events

To add a new Olympic event:

1. Copy the template from `docs/runtime/templates/OLYMPIC_EVENT_TEMPLATE.md`.
2. Fill in all fields.
3. Assign an Event ID following the convention: `OPS-{TIER}-{NNN}` where TIER is BRZ, SLV, GLD, PLT, DMD, or LEG.
4. Add the event to the appropriate section above.
5. Assign a certification weight.
6. Builder C reviews and approves the new event.
7. The new event is documented in the ledger.

All events are considered additive. Removing an event is a breaking change to the certification framework. Events may only be retired, never deleted.

## Related Documents

| Document | Purpose |
|----------|---------|
| `CORE_PROMISE.md` | The single reason BuilderBoard exists |
| `ENGINEERING_LAWS.md` | Seven permanent engineering principles |
| `RUNTIME_ENGINEERING_GUIDE.md` | Complete engineering philosophy handbook |
| `RUNTIME_CERTIFICATION.md` | Current certification status |
| `RUNTIME_WORKFLOW.md` | Complete runtime lifecycle workflow |
| `RUNTIME_FIRST_CHECKLIST.md` | Release checklist |
| `RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification |
| `templates/OLYMPIC_EVENT_TEMPLATE.md` | Template for creating new events |
