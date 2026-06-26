# Ledger Revision 2 — Status Change Summary

*Runtime Engineering Ledger update — 2026-06-27*

---

## Overview

Revision 2 updates the ledger to reflect Builder C's stabilization sprint: four fixes verified, one new field added, dependency graph validated.

---

## Status Changes

| Entry | Previous Status | New Status | Reason |
|-------|----------------|------------|--------|
| BB-0004 | OPEN | RESOLVED (Pending Runtime Certification) | Builder C implemented `resolve_create_path`. All 6 scope tests pass. OPS-BRZ-004 shows improvement. |
| BB-0005 | OPEN | RESOLVED (Pending Runtime Certification) | Builder C fixed no-match return path. Regression test `grep_missing_pattern_succeeds_with_empty_result` passes. OPS-BRZ-007 shows improvement. |
| BB-0011 | OPEN | CLOSED | Builder C verified fix already in source. Code audit confirms `Promise.allSettled` with per-source error handling at `usePaneChat.ts:134-165`. |
| BB-0012 | OPEN | CLOSED | Builder C verified fix already in source. Code audit confirms `selectedBuilderId` in dependency array at `usePaneChat.ts:483`. |
| BB-0002 | OPEN | PARTIALLY RESOLVED | Primary causes (BB-0004, BB-0005) are fixed. Validation failure rate visibly reduced per Olympics Test 004. Full certification still pending. |
| BB-0007 | OPEN | PARTIALLY RESOLVED | BB-0004/BB-0005 fixes reduce retry-caused latency. Olympics Test 004 shows improvement. Full latency targets pending full certification. |

---

## Unchanged Entries

| Entry | Status | Reason |
|-------|--------|--------|
| BB-0006 | OPEN | Planner convergence — no implementation yet |
| BB-0008 | OPEN | Inventory capability — no implementation yet |
| BB-0003 | OPEN | Hardcoded routing — no implementation yet |
| BB-0009 | OPEN | Planner budget — depends on BB-0002 and BB-0006 |
| BB-0001 | OPEN | Repository discovery — depends on BB-0009 and BB-0008 |
| BB-0010 | OPEN | Top-level blocker — still blocked by multiple dependencies |

---

## New Field Added: Verification Source

Every entry now has a `Verification Source` field indicating how we know the current state.

| Verification Source | Used By |
|--------------------|---------|
| Builder C Technical Review | All entries (Builder C reviewed the ledger state during stabilization) |
| Code Audit | BB-0004, BB-0005, BB-0011, BB-0012 (fixes verified by reading source) |
| Cargo Test | BB-0004 (6 scope tests), BB-0005 (3 search tests) |
| Runtime Olympics Test 004 | BB-0004, BB-0005, BB-0002, BB-0007 |
| IV&V Report | BB-0003 |
| Phase 8.9F.2 Investigation | BB-0011, BB-0012 |

---

## Test Count Verification

| Test Suite | Before | After | New Tests |
|------------|--------|-------|-----------|
| `cargo test --lib` | 162 | 168 | 6 (4 scope, 1 search, 1 unclassified) |
| Scope tests | 2 | 6 | 4 (`resolve_create_path_*`) |
| Search tests | 2 | 3 | 1 (`grep_missing_pattern_succeeds_with_empty_result`) |

---

## Dependency Graph (Validated — Unchanged)

```
BB-0004 (scope) ✓   BB-0005 (search) ✓    BB-0006 (convergence)
        \                /                        |
         BB-0002 (retries) — PARTIAL               |
              |                BB-0008 (inventory)  |
              |                     |               |
              BB-0009 (budget) ─────+───────────────+
             /        \
     BB-0001 (discovery)  BB-0007 (latency) — PARTIAL
            \            /
             BB-0010 (can't complete)
                    ↑
        BB-0003 (routing), BB-0011 (frontend) ✓, BB-0012 (stale closure) ✓
```

✓ = fix implemented and verified
PARTIAL = partially resolved (causes fixed, full certification pending)

No dependency relationships changed. BB-0004 and BB-0005 had no dependencies (root causes). BB-0002 still depends on BB-0004 and BB-0005. The dependency chain from BB-0002 → BB-0009 → BB-0001 → BB-0010 remains the same, now with BB-0004 and BB-0005 verified as fixed.

---

## Current Engineering State

### What is Fixed (CLOSED)
- BB-0011: Frontend `Promise.all` cascade → `Promise.allSettled`
- BB-0012: `sendMessage` stale closure → `selectedBuilderId` in dependency array

### What is Fixed but Needs Certification (RESOLVED Pending)
- BB-0004: Filesystem scope non-existent path rejection → `resolve_create_path` method
- BB-0005: Search no-match returns failure → success with empty result

### What is Partially Resolved
- BB-0002: Validation retry cascade — causes fixed, needs full cert
- BB-0007: Runtime latency — improved by retry reduction, needs planner fix

### What Still Blocks Version 1 (OPEN)
- BB-0006: Planner convergence detection (no implementation yet)
- BB-0008: Repository inventory capability (no implementation yet)
- BB-0003: Hardcoded builder routing (no implementation yet)
- BB-0009: Planner budget exhaustion (depends on BB-0002/Bb-0006)
- BB-0001: Repository discovery failure (depends on BB-0009/BB-0008)
- BB-0010: Top-level engineering completion blocker

### Next Engineering Priority

**BB-0006: Planner convergence detection.** This is now the highest-priority unstarted work because:
1. BB-0004 and BB-0005 are fixed, removing retry noise that masked planner issues
2. BB-0009 (budget exhaustion) directly depends on BB-0006
3. BB-0001 (repository discovery) depends on BB-0009
4. BB-0007 (latency) improves with fewer rounds

After BB-0006 → verify BB-0009 → then BB-0001 → then full Bronze certification.

BB-0003 (hardcoded routing) is an independent track that can be done in parallel.

### Olympics Still Requiring Execution

| Olympic Event | Status | Blocked By |
|--------------|--------|-----------|
| OPS-BRZ-004 (read) | Cert pending | BB-0004 cert |
| OPS-BRZ-005 (shell) | Cert pending | BB-0004 cert |
| OPS-BRZ-007 (search) | Cert pending | BB-0005 cert |
| OPS-BRZ-002 (chat) | — | No remaining blocker (BB-0011 CLOSED) |
| OPS-SLV-001 (2-tool chain) | Failing | BB-0006, BB-0009 |
| OPS-SLV-002 (3+ chain) | Failing | BB-0006, BB-0009 |
| OPS-SLV-003 (loop term) | Failing | BB-0006 |
| OPS-GLD-001 (multi-pane) | Failing | BB-0003 |
| OPS-BRZ-001 (launch) | — | No known blocker |

---

## Summary

| Metric | Revision 1 | Revision 2 | Change |
|--------|-----------|-----------|--------|
| Total entries | 9 | 9 | Same |
| CLOSED | 0 | 2 | +2 |
| RESOLVED (Pending Cert) | 0 | 2 | +2 |
| PARTIALLY RESOLVED | 0 | 2 | +2 |
| OPEN | 9 | 5 | −4 |
| Scope tests | 2 | 6 | +4 |
| Search tests | 2 | 3 | +1 |
| Total cargo tests | 162 | 168 | +6 |
| Fields per entry | 19 | 20 | +1 (Verification Source) |
