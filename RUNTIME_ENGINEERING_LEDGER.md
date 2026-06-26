# Runtime Engineering Ledger

*Permanent record of runtime failures and engineering issues.*

---

## Structure

Each ledger entry records:
- **Title**: What the issue is.
- **Status**: OPEN / IN PROGRESS / RESOLVED / CLOSED.
- **Priority**: P0 (critical — blocks certification) / P1 (high — prevents normal use) / P2 (medium — impairs use) / P3 (low — cosmetic or enhancement).
- **Category**: Planner / Tool Execution / Repository Discovery / Runtime / Certification / Other.
- **Olympic Events**: Links to specific Phase 0 Olympic events affected.
- **Affected Files**: Source files involved (where known).
- **Observed Behavior**: What the runtime actually does.
- **Expected Behavior**: What the runtime should do.
- **Evidence**: Olympic test results, logs, metrics.
- **Success Criteria**: How we know the issue is resolved.
- **Assigned**: Builder T / Builder V / Builder C / (unassigned).

---

## BB-0001

| Field | Value |
|-------|-------|
| **Title** | Repository-scale discovery missions exhaust the planner before producing a result |
| **Status** | OPEN |
| **Priority** | P0 |
| **Category** | Planner / Repository Discovery |
| **Olympic Events** | OPS-BRZ-007 (search), OPS-BRZ-004 (read), OPS-SLV-002 (multi-tool chaining) |
| **Affected Files** | `src-tauri/src/execution/tools/search.rs`, `src-tauri/src/execution/mod.rs` (planner), `src-tauri/src/providers/mod.rs` |

### Observed Behavior

Repository-scale discovery consistently exhausts the available planning/tool budget before sufficient information is gathered. Observed across Knowledge_Service, BuilderBoard, PepFox, and Director Desk repositories. Mission terminates with: "Maximum number of tool call rounds reached."

### Expected Behavior

BuilderBoard should efficiently explore repository structure, gather sufficient information, recognize when enough evidence exists, and produce a correct answer.

Target: <10 tool calls, <10 seconds, successful completion.

### Evidence

- Olympic Event OPS-BRZ-007: FAIL (all repositories)
- Olympic Event OPS-BRZ-004: FAIL when file path unknown
- BuilderBoard: tool exhaustion at 18+ rounds, 11+ failures
- Director Desk: 30 calls, 18 failures
- PepFox: 14 calls, 9 failures
- Knowledge_Service: 20 calls, 6 failures

### Contrasting Evidence

Targeted operations against known files succeed (OPS-BRZ-004 pass when path is known). Filesystem access, file reading, and repository comprehension all function. The failure is specifically autonomous repository discovery.

### Current Hypothesis

The planner struggles during repository exploration. Possible causes: inefficient exploration strategy, excessive retries, poor stopping criteria, repository tool validation failures, missing repository-level inventory capability.

### Success Criteria

BuilderBoard can complete repository-scale discovery missions successfully, within latency targets, without exhausting planner rounds. Examples: count source files, locate major implementation components, inventory repository, classify source languages.

### Assigned

Builder C

---

## BB-0002

| Field | Value |
|-------|-------|
| **Title** | Repository tool validation failures cause planner exhaustion |
| **Status** | OPEN |
| **Priority** | P1 |
| **Category** | Tool Execution |
| **Olympic Events** | OPS-BRZ-008 (unknown tool), OPS-BRZ-009 (invalid arguments), OPS-BRZ-005 (shell), OPS-BRZ-004 (read) |
| **Affected Files** | `src-tauri/src/execution/capability_resolver.rs`, `src-tauri/src/execution/tools/search.rs`, `src-tauri/src/filesystem_tools/scope.rs` |

### Observed Behavior

Repository inspection missions generate large numbers of tool validation failures. The planner repeatedly retries after failed tool invocations, consuming the tool-call budget without making meaningful progress.

### Expected Behavior

Repository tools should validate successfully on first attempt. Failed validations should not cause repeated retries that exhaust the planner budget.

### Evidence

- BuilderBoard: 18 calls, 11 failures
- Director Desk: 30 calls, 18 failures
- PepFox: 14 calls, 9 failures
- Knowledge_Service: 20 calls, 6 failures

### Success Criteria

Repository tools validate successfully on first attempt in >95% of invocations. Failed validations do not trigger exponential retry cascades.

### Assigned

Builder C

---

## BB-0003

| Field | Value |
|-------|-------|
| **Title** | Phase 8.9F IV&V finding — hardcoded builder routing |
| **Status** | OPEN |
| **Priority** | P1 |
| **Category** | Runtime |
| **Olympic Events** | OPS-BRZ-002 (basic chat), OPS-GLD-001 (multi-pane) |
| **Affected Files** | `src-tauri/src/stream_execution.rs:136` |

### Observed Behavior

Builder routing is hardcoded rather than dynamic. This blocks multi-pane independence and prevents the main branch from being merged.

### Expected Behavior

Builder routing should be dynamic based on pane selection.

### Success Criteria

Routing is determined by pane context. Main branch can be merged.

### Assigned

(unassigned)
