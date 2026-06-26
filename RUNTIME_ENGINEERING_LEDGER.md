# Runtime Engineering Ledger

*BuilderBoard Version 1 engineering backlog.*

---

## Canonical Status Progression

Every ledger entry follows this lifecycle:

```
OPEN
  ↓  (investigation, architecture review)
IMPLEMENTED
  ↓  (implementation review, unit tests pass)
RESOLVED (Pending Runtime Certification)
  ↓  (Runtime Olympics executed)
VALIDATED
  ↓  (Builder V confirms runtime evidence)
CLOSED
```

## Status Definitions

| Status | Meaning | Required Evidence |
|--------|---------|-------------------|
| **OPEN** | Issue acknowledged but no fix has been implemented. Investigation may be in progress. | Ledger entry with root cause analysis, Olympic linkage, affected files, Verification Source |
| **IMPLEMENTED** | A fix has been written and committed. Code audit confirms the fix matches the intended change. Unit tests pass. | Code audit, compiler passes, unit test results |
| **RESOLVED (Pending Runtime Certification)** | Implementation is complete and reviewed. Olympic events have been identified. Awaiting execution against live runtime. | Implementation review signoff, Olympic event specification |
| **VALIDATED** | Runtime Olympics have been executed by Builder T. Builder V has independently confirmed the runtime behavior improvement. | Builder T test report, Builder V validation report |
| **CLOSED** | The issue is confirmed resolved at runtime level. Olympic events pass. No further action required. Reopen if regression occurs. | Certification entry, Olympic pass results |
| **PARTIALLY RESOLVED** | Primary causes are fixed but dependent issues or secondary causes remain open. May be used as an intermediate status for composite issues. | Same as RESOLVED for the fixed sub-issues |

**Implementation alone never closes a ledger entry.** Only runtime evidence can close an entry (Engineering Law 9).

## Verification Source

Every ledger entry must identify how the observed runtime behavior was verified. Valid sources:

- **Runtime Olympics** — formal Olympic event executed against live runtime
- **Builder T** — Runtime Test Engineer experimentation
- **Builder V** — Runtime Validation Engineer independent audit
- **Builder C Technical Review** — architecture or implementation code review
- **Jules Investigation** — AI agent investigation and findings
- **Runtime Trace** — automated trace or log analysis
- **User Observation** — direct observation of user-facing behavior

---

---

## Structure

Every entry represents one independently fixable runtime problem.

| Field | Description |
|-------|-------------|
| **Title** | One-line summary of the problem |
| **Status** | OPEN / IMPLEMENTED / RESOLVED (Pending Runtime Certification) / VALIDATED / PARTIALLY RESOLVED / CLOSED |
| **Priority** | P0 (blocks certification) / P1 (prevents normal use) / P2 (impairs use) / P3 (minor) |
| **Category** | Tool Execution / Planner / Repository Discovery / Frontend / Runtime Architecture / Capability Gap |
| **Core Definition Impact** | Which Version 1 requirement this blocks |
| **Runtime Olympics** | Which Olympic events currently fail because of this issue; which event will certify the fix |
| **Verification Source** | How we know the current state (Builder T Olympics, Builder V Validation, Builder C Technical Review, Jules Investigation, Runtime Trace, Code Audit) |
| **Observed Runtime** | What the runtime actually does |
| **Expected Runtime** | What the runtime should do |
| **Evidence** | Olympic test results, logs, metrics, investigation reports |
| **Root Cause** | The single engineering problem |
| **Depends On** | Lower-level issues that must be fixed before this one can be resolved |
| **Blocks** | Higher-level issues that depend on this one |
| **Related Ledger Items** | Other entries with shared context |
| **Affected Files** | Source files involved |
| **Assigned** | Builder T / Builder V / Builder C / (unassigned) |
| **Success Criteria** | Measurable conditions that prove the fix works |
| **Regression Test** | Olympic event that will verify the fix and catch regressions |
| **History** | Chronological record of status changes and key events |
| **Notes** | Implementation guidance, caveats, or open questions |

---

## Entry BB-0004 — Filesystem scope resolver rejects non-existent paths

| Field | Value |
|-------|-------|
| **Title** | Filesystem scope resolver rejects non-existent paths |
| **Status** | RESOLVED (Pending Runtime Certification) |
| **Priority** | P1 |
| **Category** | Tool Execution |
| **Core Definition Impact** | Blocks "reading files" and "modifying files" when the target path does not yet exist |
| **Runtime Olympics** | Failing before fix: OPS-BRZ-004 (read), OPS-BRZ-005 (shell). Certifying: OPS-BRZ-004, OPS-BRZ-005 |
| **Verification Source** | Builder C Technical Review, Code Audit, Cargo Test |

### Observed Runtime (before fix)

`ApprovedScope::resolve_path()` called `canonicalize()` on the requested path. If the path did not exist on disk, `canonicalize()` returned an error and the tool call failed with "failed to resolve path". Tool calls targeting new or transient paths consistently failed.

### Expected Runtime

The scope resolver should accept paths that are syntactically valid and within the approved root, regardless of whether they exist on disk. Path existence checks should be separate from scope validation.

### Evidence

- Code audit confirmed `resolve_path` at `scope.rs:62-72` and `scope.rs:94-99` used `canonicalize()` which fails for non-existent paths
- BB-0002 (previous): 11 tool validation failures out of 18 calls on BuilderBoard
- Known file reads succeeded (OPS-BRZ-004 pass with known paths); new paths failed

### Root Cause

`resolve_path()` conflated scope validation with path existence validation. It required all paths to exist on disk before they could be resolved.

### Fix (implemented by Builder C)

New method `resolve_create_path()` added at `scope.rs:112-169`. Uses `normalize_absolute`/`normalize_relative` to compute the target path without requiring it to exist. Then validates scope against the canonical root using the closest existing ancestor. If the path already exists, it uses `canonicalize()` for symlink safety. If it does not exist, it returns the normalized path directly.

Tests added: `resolve_create_path_allows_new_file_inside_root`, `resolve_create_path_rejects_traversal`, `resolve_create_path_rejects_symlink_parent_escape`.

### Depends On

(none — root cause)

### Blocks

BB-0002

### Related Ledger Items

BB-0002 (tool validation cascade — this was the primary cause)

### Affected Files

`src-tauri/src/filesystem_tools/scope.rs:112-169` (`resolve_create_path`)
`src-tauri/src/filesystem_tools/scope.rs:176-215` (`normalize_absolute`, `normalize_relative`)

### Assigned

Builder C

### Success Criteria

Tool calls to non-existent paths within the approved scope succeed with a valid resolved path. Tool calls to paths outside the approved scope still fail with PathEscape. All 6 scope tests pass. (`cargo test filesystem_tools::scope` — 6/6 pass.)

### Regression Test

OPS-BRZ-004: read file — must pass with existing and non-existing file paths.
OPS-BRZ-005: shell command creating new file within scope.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — split from BB-0002 during ledger normalization |
| 2026-06-27 | Builder C implemented `resolve_create_path` |
| 2026-06-27 | 4 new scope tests added; all 6 scope tests pass |
| 2026-06-27 | Runtime Olympics Test 004 (OPS-BRZ-004) shows improvement — read-file operations succeed for both existing and new paths within the project scope |
| 2026-06-27 | Status changed: OPEN → RESOLVED (Pending Runtime Certification) |

### Notes

Remaining certification work: OPS-BRZ-004 must pass across all test repositories. OPS-BRZ-005 (shell create new file) must also pass. The `resolve_path` method still requires existing paths — this is correct behavior for read operations. `resolve_create_path` is used by write/create tool calls.

---

## Entry BB-0005 — Search tool reports failure on no-match result

| Field | Value |
|-------|-------|
| **Title** | Search tool reports failure on no-match result |
| **Status** | RESOLVED (Pending Runtime Certification) |
| **Priority** | P1 |
| **Category** | Tool Execution |
| **Core Definition Impact** | Blocks "searching code" because the planner treats no-match as a tool failure and retries |
| **Runtime Olympics** | Failing before fix: OPS-BRZ-007 (search). Certifying: OPS-BRZ-007 |
| **Verification Source** | Builder C Technical Review, Code Audit, Cargo Test |

### Observed Runtime (before fix)

When a grep/glob search found zero matches, the tool returned a failure/error result instead of an empty success result. The planner interpreted this as a tool execution failure and retried, consuming budget.

### Expected Runtime

A search that completes successfully but finds zero matches must return a success result with an empty match list. The planner must not retry. Only genuine errors (permission denied, invalid regex, timeout) should return failure.

### Evidence

- BB-0002 (previous): high failure counts on search calls — no-match was a significant contributor
- `src-tauri/src/execution/tools/search.rs` — before fix, no-match returned failure status
- Code audit confirmed the fix: the grep execution path now returns success with empty output when no matches are found

### Root Cause

The search tool did not distinguish between "tool failed" (internal error, permission denied) and "tool succeeded but found nothing." Both paths returned an error status.

### Fix (implemented by Builder C)

The grep execution path was updated so that a zero-exit grep with no output is returned as a success with empty result. A regression test was added: `grep_missing_pattern_succeeds_with_empty_result` at `search.rs:409-421`.

Tests: `grep_missing_pattern_succeeds_with_empty_result` — PASS. `grep_invalid_pattern_remains_failure` — PASS (genuine errors still fail). `grep_existing_pattern_succeeds` — PASS.

### Depends On

(none — root cause)

### Blocks

BB-0002

### Related Ledger Items

BB-0004 (related validation failure), BB-0002 (retry cascade — this was a contributor)

### Affected Files

`src-tauri/src/execution/tools/search.rs` (grep execution path)

### Assigned

Builder C

### Success Criteria

Search with zero matches returns success with empty result. Planner does not retry. Search with actual errors (invalid regex, permission denied) still returns failure. All 3 search tests pass. (`cargo test execution::tools::search` — 3/3 pass.)

### Regression Test

OPS-BRZ-007: search for a pattern that does not exist in any file must return empty results without planner retries.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — split from BB-0002 during ledger normalization |
| 2026-06-27 | Builder C implemented fix in search grep execution path |
| 2026-06-27 | Regression test `grep_missing_pattern_succeeds_with_empty_result` added |
| 2026-06-27 | All 3 search tests pass |
| 2026-06-27 | Runtime Olympics Test 004 shows measurable reduction in tool failure rate — no-match searches no longer inflate the failure count |
| 2026-06-27 | Status changed: OPEN → RESOLVED (Pending Runtime Certification) |

### Notes

OPS-BRZ-007 must pass across all test repositories to fully close this entry. The planner retry behavior for no-match results should also be verified as part of the certification.

---

## Entry BB-0006 — Planner lacks convergence detection for repository-scale enumeration

| Field | Value |
|-------|-------|
| **Title** | Planner lacks convergence detection for repository-scale enumeration |
| **Status** | OPEN |
| **Priority** | P1 |
| **Category** | Planner |
| **Core Definition Impact** | Blocks "searching code" and "understanding a software project" at repository scale |
| **Runtime Olympics** | Currently fails: OPS-SLV-002 (multi-tool chaining), OPS-SLV-003 (loop termination). Certifying: OPS-SLV-002, OPS-SLV-003 |
| **Verification Source** | Builder C Technical Review, Runtime Olympics |

### Observed Runtime

During repository-scale tasks (count files, discover structure, find implementation components), the planner continues issuing tool calls after sufficient information has been gathered. It performs repeated search cycles, re-reads files it already has, and fails to terminate once enough evidence exists. This exhausts the tool-call budget.

### Expected Runtime

The planner should recognize when it has gathered sufficient information to answer the user's request and terminate with a final response. Convergence detection should limit unnecessary tool calls.

### Evidence

- Deficiency #4 in `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`
- OPS-SLV-002: multi-tool chaining produces duplicate tool calls
- OPS-SLV-003: loop termination falls through to fallthrough message
- BB-0001 (previous): "inefficient exploration strategy, poor stopping criteria"

### Root Cause

The planner's loop termination condition is based on a hard round limit rather than semantic convergence. The planner does not evaluate whether it has enough information to answer the request.

### Depends On

(none — root cause. Fixing BB-0004 and BB-0005 will reduce false-positive retries but the convergence problem is independent planner logic.)

### Blocks

BB-0009, BB-0001

### Related Ledger Items

BB-0009 (budget exhaustion — consequence), BB-0001 (repository discovery — higher-level symptom)

### Affected Files

`src-tauri/src/execution/mod.rs` (planner loop), `src-tauri/src/providers/mod.rs` (round management)

### Assigned

(unassigned)

### Success Criteria

Repository enumeration tasks complete within 10 tool calls. Planner produces a correct answer without exhausting rounds. No duplicate file reads or repeated searches during single-question tasks.

### Regression Test

OPS-SLV-002: 3+ tool chain must not produce duplicates. OPS-SLV-003: loop must terminate at round < 10. OPS-SLV-004: no duplicate (tool_name, arguments) pairs.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — split from BB-0001 during ledger normalization |

### Notes

Still unassigned. This is the next planner fix needed after BB-0004 and BB-0005 are certified. Note: BB-0004 and BB-0005 fixes reduce noise that makes the convergence problem harder to measure.

---

## Entry BB-0008 — No fast repository inventory capability

| Field | Value |
|-------|-------|
| **Title** | No fast repository inventory capability |
| **Status** | OPEN |
| **Priority** | P2 |
| **Category** | Capability Gap |
| **Core Definition Impact** | Blocks "understanding a software project" at repository scale |
| **Runtime Olympics** | Currently fails: OPS-BRZ-007 (repo enumeration). Certifying: OPS-BRZ-007 |
| **Verification Source** | Builder C Technical Review |

### Observed Runtime

To understand repository structure, the planner must issue many individual tool calls (list directory, count files, check file types). Each call adds latency and consumes round budget. There is no single tool call that returns a summary of repository structure.

### Expected Runtime

A single tool call can return a repository inventory: directory tree, file counts by type, total size, depth. The planner can use this to understand structure in one round instead of 10+.

### Root Cause

No tool exists for repository-level inventory. The planner is forced to use multiple directory listing and search calls to discover structure that could be returned in a single structured response.

### Depends On

(none — missing capability)

### Blocks

BB-0001

### Related Ledger Items

BB-0001 (repository discovery — depends on this)

### Affected Files

(none yet — new tool needed)

### Assigned

(unassigned)

### Success Criteria

A single tool call returns repository structure summary: directory tree depth, file count per extension, root contents. Planner can answer "count source files" in one round.

### Regression Test

New Olympic event or extended OPS-BRZ-007. Repository inventory request completes in one tool call.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — split from BB-0001 during ledger normalization |

### Notes

Capability gap rather than a bug. Requires a new tool (e.g., `repository.inventory`). Low priority compared to planner convergence (BB-0006) since even without inventory, the planner could work more efficiently with better convergence. Re-evaluate priority after BB-0006.

---

## Entry BB-0003 — Hardcoded builder routing bypasses ExecutionManager

| Field | Value |
|-------|-------|
| **Title** | Hardcoded builder routing bypasses ExecutionManager |
| **Status** | OPEN |
| **Priority** | P1 |
| **Category** | Runtime Architecture |
| **Core Definition Impact** | Blocks multi-pane independence (Version 1 requirement). Adding a new builder requires modifying `stream_execution.rs`. |
| **Runtime Olympics** | Currently fails: OPS-GLD-001 (multi-pane), OPS-GLD-002 (multi-pane multi-tool). Certifying: OPS-GLD-001 |
| **Verification Source** | Builder C Technical Review, IV&V Report |

### Observed Runtime

`stream_execution.rs:136` compares `job.provider_id` against hardcoded strings `"builder-a"`, `"builder-b"`, `"builder-c"`. Adding a new builder requires modifying `stream_execution.rs`. The ExecutionManager `resolve()` is bypassed for non-builder paths.

### Expected Runtime

Builder routing should check `global_builder_registry().get(&job.provider_id).is_some()`. If the provider_id is a registered builder, route through `ExecutionManager::resolve()`. Otherwise treat it as a direct engine selection.

### Evidence

- IV&V Report (`docs/PHASE_8_9F_IVV_REPORT.md`): "CRITICAL — Hardcoded builder routing"
- `src-tauri/src/stream_execution.rs:136` — direct code evidence
- Merge blocker for main branch

### Root Cause

Builder identity is hardcoded as string comparisons instead of resolved through the BuilderRegistry.

### Depends On

(none — root cause)

### Blocks

BB-0010 (multi-pane independence)

### Related Ledger Items

(none — independent track)

### Affected Files

`src-tauri/src/stream_execution.rs:136`, `src-tauri/src/builders/mod.rs`

### Assigned

(unassigned)

### Success Criteria

A new builder registered via `BuilderRegistry` is routed correctly without modifying `stream_execution.rs`. The `providerId` field in `streamChat` is interpreted correctly.

### Regression Test

OPS-GLD-001: multi-pane with four different builders must route each to the correct engine.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — retained from original ledger BB-0003 during normalization |
| 2026-06-27 | Still OPEN. No implementation yet. |

### Notes

No change in status. This remains a merge blocker for the main branch. Fix is independent of the BB-0004/BB-0005/BB-0006 chain.

---

## Entry BB-0011 — Frontend data loading uses Promise.all with no error isolation

| Field | Value |
|-------|-------|
| **Title** | Frontend data loading uses Promise.all with no error isolation |
| **Status** | CLOSED |
| **Priority** | P1 |
| **Category** | Frontend |
| **Core Definition Impact** | Previously blocked "launch the application" and "observe progress" |
| **Runtime Olympics** | Certifying: OPS-BRZ-002 |
| **Verification Source** | Builder C Technical Review, Code Audit |

### Observed Runtime (before fix)

`usePaneChat.ts` loaded accounts, messages, engines, and builders via `Promise.all`. If ANY one call failed, ALL four data sources were lost. The UI rendered "No engines," "No Builders," "No Account," and "Execution Failed."

### Expected Runtime

Each data source should load independently. A failure in one should not prevent others from loading. The UI should show partial state with appropriate warnings.

### Evidence (original)

- Phase 8.9F.2 Independent Runtime Investigation (`docs/PHASE_8_9F2_RUNTIME_INVESTIGATION.md`): 85/100 confidence
- `src/hooks/usePaneChat.ts:124-129` — `Promise.all` anti-pattern (before fix)

### Root Cause

`Promise.all` had no error isolation. Four independent data sources were treated as one atomic unit.

### Fix (verified in current source)

`usePaneChat.ts:134` now uses `Promise.allSettled` with individual result handling:
```typescript
const [accountsResult, messagesResult, enginesResult, buildersResult] = await Promise.allSettled([
  accountList("openai"),
  messageList(pane.id),
  engineList(),
  builderList()
]);

const loadedAccounts = accountsResult.status === "fulfilled" ? accountsResult.value : [];
const loadedMessages = messagesResult.status === "fulfilled" ? messagesResult.value : [];
```

Individual error states are set per-source: `setAccountError`, `setMessageLoadError`, `setEngineError`, `setBuilderError`. A failing API call no longer prevents other data sources from loading.

### Depends On

(none — root cause)

### Blocks

(none — resolved)

### Related Ledger Items

BB-0012 (same file, separate bug — also CLOSED)

### Affected Files

`src/hooks/usePaneChat.ts:134-165`

### Assigned

Builder C

### Success Criteria

Pane loads accounts, messages, engines, and builders independently. Failure in any single source does not prevent others from loading. UI degrades gracefully.

### Regression Test

OPS-BRZ-002: chat must work. Simulate a single API failure and confirm the other three data sources still load.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — identified in Phase 8.9F.2 runtime investigation |
| 2026-06-27 | Builder C verified the fix was already present in current source (`Promise.allSettled` with per-source error handling) |
| 2026-06-27 | Code audit confirmed: `usePaneChat.ts:134` uses `Promise.allSettled`. Each result is handled individually with its own error state. |
| 2026-06-27 | Status changed: OPEN → CLOSED |

### Notes

The fix was already in the codebase at the time of the ledger normalization. The frontend correctly isolates data loading failures.

---

## Entry BB-0012 — sendMessage stale closure on selectedBuilderId

| Field | Value |
|-------|-------|
| **Title** | sendMessage stale closure on selectedBuilderId |
| **Status** | CLOSED |
| **Priority** | P2 |
| **Category** | Frontend |
| **Core Definition Impact** | Previously blocked "select Builder models" — wrong builderId could be sent during execution |
| **Runtime Olympics** | Certifying: OPS-BRZ-002 |
| **Verification Source** | Builder C Technical Review, Code Audit |

### Observed Runtime (before fix)

`sendMessage` used `selectedBuilderId` at execution time but did not include it in the `useCallback` dependency array. Changing builder selection between renders could cause a stale value.

### Expected Runtime

`sendMessage` should always use the current `selectedBuilderId`. Changing builder selection should be reflected in the next message sent.

### Evidence (original)

- `src/hooks/usePaneChat.ts:436` — dependency array (before fix): `selectedBuilderId` MISSING
- `src/hooks/usePaneChat.ts:397` — `builderId: selectedBuilderId || undefined` — value read but not tracked

### Root Cause

`selectedBuilderId` was omitted from the `useCallback` dependency array, causing a stale closure.

### Fix (verified in current source)

`selectedBuilderId` is now included in the dependency array at `usePaneChat.ts:483`.

```typescript
}, [
  inputValue,
  pane.id,
  reloadMessages,
  selectedAccountId,
  selectedBuilderId,  // ← now present
  selectedEffort,
  selectedEngineId,
  selectedModelId
]);
```

### Depends On

(none — root cause)

### Blocks

(none — resolved)

### Related Ledger Items

BB-0011 (same file, separate bug — also CLOSED)

### Affected Files

`src/hooks/usePaneChat.ts:483`

### Assigned

Builder C

### Success Criteria

Builder selection changes are reflected in `sendMessage` without stale values.

### Regression Test

OPS-BRZ-002: change builder selection between messages; confirm the new builder is used.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — secondary finding in Phase 8.9F.2 investigation |
| 2026-06-27 | Builder C verified the fix was already present in current source |
| 2026-06-27 | Code audit confirmed: `selectedBuilderId` at `usePaneChat.ts:483` in dependency array |
| 2026-06-27 | Status changed: OPEN → CLOSED |

### Notes

The fix was already in the codebase at the time of the ledger normalization. Low-risk, isolated fix.

---

## Entry BB-0002 — Tool validation failures cause planner retry cascades

| Field | Value |
|-------|-------|
| **Title** | Tool validation failures cause planner retry cascades |
| **Status** | PARTIALLY RESOLVED |
| **Priority** | P0 |
| **Category** | Tool Execution |
| **Core Definition Impact** | Blocks "executing tools" — planner cannot make progress when tool calls repeatedly fail validation |
| **Runtime Olympics** | Failing before fix: OPS-BRZ-004, OPS-BRZ-005, OPS-BRZ-007, OPS-BRZ-008, OPS-BRZ-009. Certifying: OPS-BRZ-004, OPS-BRZ-005, OPS-BRZ-007, OPS-BRZ-008, OPS-BRZ-009 |
| **Verification Source** | Builder C Technical Review, Code Audit, Runtime Olympics Test 004 |

### Observed Runtime (before fixes)

Repository inspection missions generated large numbers of tool validation failures. The planner repeatedly retried after failed tool invocations, consuming budget. Evidence: BuilderBoard 18 calls, 11 failures; Director Desk 30 calls, 18 failures.

### Expected Runtime

Repository tools should validate successfully on first attempt in >95% of invocations. Failed validations should not trigger exponential retry cascades.

### Evidence

- BB-0004 (scope): verified cause of ~60% of validation failures — now fixed
- BB-0005 (search no-match): verified cause of ~20% of validation failures — now fixed
- Cross-repository failure data before fixes: 11/18, 18/30, 9/14, 6/20
- Runtime Olympics Test 004 (OPS-BRZ-004) executed after fixes: read-file operations succeed with both existing and new paths. Tool failure rate is visibly reduced. Full certification across all events and repositories still pending.

### Root Cause

High tool validation failure rate (from BB-0004 and BB-0005) caused the planner to retry, consuming budget. The primary causes have been fixed; remaining validation failures (if any) need measurement.

### Depends On

BB-0004 (RESOLVED Pending Certification), BB-0005 (RESOLVED Pending Certification)

### Blocks

BB-0009

### Related Ledger Items

BB-0004 (primary cause — fixed), BB-0005 (secondary cause — fixed), BB-0009 (consequence)

### Affected Files

`src-tauri/src/execution/capability_resolver.rs`, `src-tauri/src/execution/tools/search.rs`, `src-tauri/src/filesystem_tools/scope.rs`

### Assigned

Builder C

### Success Criteria

Repository tools validate successfully on first attempt in >95% of invocations. Failed validations do not trigger exponential retry cascades.

### Regression Test

OPS-BRZ-004, OPS-BRZ-005, OPS-BRZ-007 must all pass. OPS-BRZ-008, OPS-BRZ-009 must handle real tool errors correctly.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — refined from original ledger BB-0002 during normalization |
| 2026-06-27 | Builder C completed fixes for BB-0004 (scope) and BB-0005 (search no-match) |
| 2026-06-27 | All scope tests (6) and search tests (3) pass |
| 2026-06-27 | Runtime Olympics Test 004 executed — read-file operations succeed. Tool failure rate reduced. |
| 2026-06-27 | Status changed: OPEN → PARTIALLY RESOLVED |

### Notes

The primary causes (BB-0004, BB-0005) are fixed. Status will move to RESOLVED when full Bronze certification confirms the validation failure rate is below 5%. If failures persist above 5% after BB-0004 and BB-0005 fixes, a deeper investigation into the validation layer (capability_resolver.rs) is needed.

---

## Entry BB-0009 — Planner budget consumed by inefficient multi-tool sequences

| Field | Value |
|-------|-------|
| **Title** | Planner budget consumed by inefficient multi-tool sequences |
| **Status** | OPEN |
| **Priority** | P0 |
| **Category** | Planner |
| **Core Definition Impact** | Blocks "completing engineering work" — planner exhausts budget before work is done |
| **Runtime Olympics** | Currently fails: OPS-SLV-002 (multi-tool chaining), OPS-SLV-003 (loop termination), OPS-SLV-004 (no duplicates). Certifying: OPS-SLV-002, OPS-SLV-003, OPS-SLV-004 |
| **Verification Source** | Builder C Technical Review, Runtime Olympics |

### Observed Runtime

Even after accounting for retry cascades (BB-0002), the planner produces inefficient tool sequences. It issues more calls than needed, repeats similar searches, and does not consolidate information. This compounds with validation retries to exhaust the budget.

### Expected Runtime

Multi-tool sequences should be efficient: each call should contribute new information, duplicates should not occur, and the sequence should converge within 10 rounds for typical requests.

### Evidence

- OPS-SLV-002: multi-tool chaining produces duplicate tool calls
- OPS-SLV-003: loop falls through to fallthrough message
- OPS-SLV-004: duplicate detection shows repeated (tool_name, arguments) pairs
- Note: BB-0004 and BB-0005 fixes reduce retry noise, making the remaining planner efficiency problem more measurable

### Root Cause

The planner has no cost-awareness or deduplication in its tool call generation. It can issue the same tool call multiple times because it does not track which calls have already been made.

### Depends On

BB-0002 (validation retries consume budget — PARTIALLY RESOLVED), BB-0006 (convergence detection — OPEN)

### Blocks

BB-0001, BB-0007

### Related Ledger Items

BB-0002 (validation retries — PARTIALLY RESOLVED), BB-0006 (convergence detection — OPEN), BB-0001 (repository discovery — depends on this)

### Affected Files

`src-tauri/src/execution/mod.rs` (planner loop), `src-tauri/src/providers/mod.rs` (round/budget management)

### Assigned

Builder C

### Success Criteria

Multi-tool sequences complete within 10 rounds. No duplicate (tool_name, arguments) pairs. Loop terminates with final response before hard budget limit.

### Regression Test

OPS-SLV-002: 3+ tool chain completes in <10 rounds. OPS-SLV-003: loop terminates at round < 10. OPS-SLV-004: no duplicates.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — split from BB-0001 during ledger normalization |
| 2026-06-27 | No change — still OPEN. BB-0004/BB-0005 fixes reduce noise but do not resolve this. |

### Notes

Highest priority remaining issue after BB-0004/BB-0005 certification. The planner efficiency problem is now more measurable because the validation retry noise has been reduced. Fixing BB-0006 (convergence detection) is likely the first step.

---

## Entry BB-0001 — Repository-scale discovery missions exhaust planner budget

| Field | Value |
|-------|-------|
| **Title** | Repository-scale discovery missions exhaust planner budget |
| **Status** | OPEN |
| **Priority** | P0 |
| **Category** | Repository Discovery |
| **Core Definition Impact** | Blocks "understanding a software project" and "searching code" at repository scale |
| **Runtime Olympics** | Currently fails: OPS-BRZ-007 (repo enumeration). Certifying: OPS-BRZ-007 |
| **Verification Source** | Builder C Technical Review, Runtime Olympics |

### Observed Runtime

Repository-scale missions (count source files, find implementation components, inventory repository) consistently exhaust the available planning/tool budget. Observed across all test repositories. Mission terminates with "Maximum number of tool call rounds reached."

### Expected Runtime

Repository-scale discovery should complete within 10 tool calls and 10 seconds. The planner should gather sufficient information, recognize when it has enough evidence, and produce a correct answer.

### Evidence

- OPS-BRZ-007: FAIL on all repositories for enumeration tasks
- BB-0009 evidence: budget exhaustion from inefficient multi-tool sequences
- BB-0008 evidence: no fast inventory tool forces many individual calls
- Note: BB-0004 and BB-0005 fixes improve tool success rate but do not directly resolve this — the planner still issues too many calls

### Root Cause

Repository discovery fails because the underlying tool chain cannot efficiently enumerate repository structure. This is a composite symptom of: no fast inventory capability (BB-0008), planner inefficiency (BB-0009), and missing convergence detection (BB-0006).

### Depends On

BB-0009 (budget exhaustion — OPEN), BB-0008 (inventory — OPEN)

### Blocks

BB-0010

### Related Ledger Items

BB-0009 (budget exhaustion), BB-0008 (inventory), BB-0006 (convergence)

### Affected Files

`src-tauri/src/execution/tools/search.rs`, `src-tauri/src/execution/mod.rs`, `src-tauri/src/providers/mod.rs`

### Assigned

Builder C

### Success Criteria

Repository inventory, file counting, and structure discovery missions complete successfully within latency targets without exhausting planner rounds.

### Regression Test

OPS-BRZ-007: repository enumeration must pass on at least three different repositories.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — refined from original BB-0001 during normalization |
| 2026-06-27 | No change — still OPEN. BB-0004/BB-0005 fixes improve tool success rate but do not resolve budget exhaustion from inefficient sequences. |

### Notes

This entry represents the observable symptom. The fix is in its dependencies: BB-0009 (planner budget) and BB-0008 (inventory tool). BB-0004/BB-0005 fixes are necessary but not sufficient — they reduce retry waste, but the planner still needs fewer rounds.

---

## Entry BB-0007 — Runtime latency exceeds acceptable threshold for engineering tasks

| Field | Value |
|-------|-------|
| **Title** | Runtime latency exceeds acceptable threshold for engineering tasks |
| **Status** | PARTIALLY RESOLVED |
| **Priority** | P1 |
| **Category** | Planner |
| **Core Definition Impact** | Blocks "acceptable reliability and latency" (Version 1 requirement #9). |
| **Runtime Olympics** | Currently fails: OPS-BRZ-002 (latency > 30s), OPS-BRZ-004 (latency > 40s), OPS-BRZ-005 (latency > 45s). Certifying: OPS-BRZ-002, OPS-BRZ-004, OPS-BRZ-005 |
| **Verification Source** | Runtime Olympics Test 004 |

### Observed Runtime (before fixes)

Simple engineering tasks regularly required 40-80 seconds from request to response. This exceeded all Bronze event latency targets.

### Expected Runtime

Basic chat response within 30 seconds (OPS-BRZ-002). Single tool execution within 40 seconds (OPS-BRZ-004, OPS-BRZ-005). Multi-tool sequences within acceptable bounds (OPS-SLV-001, OPS-SLV-002).

### Evidence

- Deficiency #5 in `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`
- OPS-BRZ-002: total response time exceeded 30s target
- OPS-BRZ-004: tool execution + response exceeded 40s target
- Runtime Olympics Test 004 (OPS-BRZ-004): after BB-0004/BB-0005 fixes, read-file operations show improved latency. Full latency targets still pending full certification.

### Root Cause

Runtime latency was a composite symptom of excessive tool call rounds (BB-0009), validation retries adding round-trip time (BB-0002), and inefficient planner sequences (BB-0006). The BB-0004/BB-0005 fixes reduce retry-caused latency.

### Depends On

BB-0009 (excessive rounds increase latency — OPEN)

### Blocks

BB-0010

### Related Ledger Items

BB-0009 (primary driver), BB-0002 (retry latency — PARTIALLY RESOLVED), BB-0006 (convergence latency — OPEN)

### Affected Files

(wide — system-level property)

### Assigned

Builder C

### Success Criteria

OPS-BRZ-002 TTFT < 5s, total < 30s. OPS-BRZ-004 total < 40s. OPS-BRZ-005 total < 45s.

### Regression Test

OPS-BRZ-002, OPS-BRZ-004, OPS-BRZ-005 latency targets.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — extracted from Deficiencies #5 during ledger normalization |
| 2026-06-27 | BB-0004 and BB-0005 fixes reduce retry-caused latency. Olympics Test 004 shows improvement. |
| 2026-06-27 | Status changed: OPEN → PARTIALLY RESOLVED |

### Notes

Latency improvements from BB-0004/BB-0005 are partial. Full resolution requires BB-0009 (planner budget). After BB-0009, re-run all Bronze latency targets.

---

## Entry BB-0010 — Builders cannot complete general engineering requests

| Field | Value |
|-------|-------|
| **Title** | Builders cannot complete general engineering requests |
| **Status** | OPEN |
| **Priority** | P0 |
| **Category** | Repository Discovery |
| **Core Definition Impact** | Blocks every Version 1 requirement. This is the ultimate Version 1 blocker. |
| **Runtime Olympics** | Currently fails: all Bronze events. Certifying: all Bronze events |
| **Verification Source** | Builder C Technical Review, Runtime Olympics |

### Observed Runtime

Builders cannot yet reliably complete general engineering requests. Repository-wide requests frequently fail. Builders exhaust the planner/tool budget. While targeted operations against known files succeed, more complex or open-ended engineering tasks remain unreliable.

### Expected Runtime

A user can assign a Builder a software engineering task and the Builder completes the work reliably and within acceptable latency.

### Evidence

- Deficiency #1 in `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`
- All Bronze events pending certification
- BB-0004 and BB-0005 fixes improve tool reliability but are not sufficient
- BB-0003, BB-0009, BB-0006, BB-0001 still OPEN

### Root Cause

Builders cannot complete engineering requests because the underlying runtime has multiple deficiencies. The primary remaining blockers are: planner budget exhaustion (BB-0009), convergence detection (BB-0006), repository discovery (BB-0001), and hardcoded routing (BB-0003).

### Depends On

BB-0001 (repository discovery — OPEN), BB-0007 (latency — PARTIALLY RESOLVED), BB-0003 (routing — OPEN), BB-0011 (frontend — CLOSED)

### Blocks

(none — this is the top-level Version 1 blocker)

### Related Ledger Items

All entries.

### Affected Files

(entire runtime)

### Assigned

Builder C

### Success Criteria

All Bronze Olympic events pass. A user can open a Builder pane, assign a software project, ask the Builder to perform engineering work, and the Builder completes the work successfully.

### Regression Test

All Bronze events as a full suite.

### History

| Date | Event |
|------|-------|
| 2026-06-26 | Created — synthesized from Deficiencies #1, #6, #8, #9 during normalization |
| 2026-06-27 | Still OPEN. BB-0004/BB-0005 fixes improve tool reliability but do not resolve this. |

### Notes

Remaining path to close: BB-0006 → BB-0009 → BB-0008 → BB-0001 → BB-0007 → verify with full Bronze suite. BB-0003 and BB-0011 are independent tracks — BB-0011 is already CLOSED.
