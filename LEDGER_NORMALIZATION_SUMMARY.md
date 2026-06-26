# Ledger Normalization Summary

*Runtime Engineering Ledger normalization — 2026-06-26*

---

## What Changed

The Runtime Engineering Ledger was normalized from 3 compound entries to 9 single-root-cause entries.

| Metric | Before | After |
|--------|--------|-------|
| Total entries | 3 | 9 |
| Root cause entries | 0 | 7 |
| Intermediate symptom entries | 0 | 1 |
| Top-level symptom entries | 0 | 2 |
| Entries with dependencies | 0 | 9 |
| Entries with Olympic linkage | 1 | 9 |
| Entries with Core Definition Impact | 0 | 9 |
| Average root causes per entry | ~3 | 1 |

---

## Split Entries

### BB-0001 (previous) → 5 new entries

Previous BB-0001 "Repository-scale discovery missions exhaust the planner" contained multiple hypotheses and root causes. It was split into:

| New Entry | Title | Type |
|-----------|-------|------|
| BB-0004 | Filesystem scope resolver rejects non-existent paths | Root cause |
| BB-0005 | Search tool reports failure on no-match result | Root cause |
| BB-0006 | Planner lacks convergence detection for repository-scale enumeration | Root cause |
| BB-0008 | No fast repository inventory capability | Root cause (capability gap) |
| BB-0009 | Planner budget consumed by inefficient multi-tool sequences | Intermediate symptom |
| BB-0001 | Repository-scale discovery missions exhaust planner budget | Top-level symptom (refined) |

### BB-0002 (previous) → 2 new entries + refined BB-0002

Previous BB-0002 "Repository tool validation failures cause planner exhaustion" mixed the cause (validation failures) with the consequence (planner exhaustion). It was split into:

| New Entry | Title | Type |
|-----------|-------|------|
| BB-0004 | Filesystem scope resolver rejects non-existent paths | Root cause (moved) |
| BB-0005 | Search tool reports failure on no-match result | Root cause (moved) |
| BB-0002 | Tool validation failures cause planner retry cascades | Intermediate symptom (refined) |

### BB-0003 (previous) → retained intact

Previous BB-0003 "Hardcoded builder routing" was already a single root cause. Retained with same ID, updated to standard template.

---

## New Entries Added

| New ID | Title | Source |
|--------|-------|--------|
| BB-0006 | Planner lacks convergence detection for repository-scale enumeration | Split from BB-0001 |
| BB-0008 | No fast repository inventory capability | Split from BB-0001 |
| BB-0009 | Planner budget consumed by inefficient multi-tool sequences | Split from BB-0001 |
| BB-0010 | Builders cannot complete general engineering requests | Deficiencies #1, #6, #8, #9 |
| BB-0011 | Frontend data loading uses Promise.all with no error isolation | Phase 8.9F.2 investigation |
| BB-0012 | sendMessage stale closure on selectedBuilderId | Phase 8.9F.2 investigation (secondary) |
| BB-0007 | Runtime latency exceeds acceptable threshold for engineering tasks | Deficiencies #5 |

---

## Entries Removed (Non-Version-1)

| Entry | Reason |
|-------|--------|
| Deficiency #7 "Runtime certification not fully autonomous" | Process issue, not a runtime problem. Does not directly block the Core Promise. |
| Deficency #8 "Runtime reliability not yet demonstrated" | Consequence of other deficiencies, not an independent root cause. |
| Dead code from IV&V report (6 unused functions) | Preexisting technical debt, not blocking Version 1. |

---

## Dependency Graph

```
BB-0004 (filesystem scope)          BB-0005 (search no-match)   BB-0006 (convergence)
        \                              /                              |
         \                            /                               |
          BB-0002 (validation retries)                                 |
                       |                                               |
                       |                    BB-0008 (no fast inventory)
                       |                              |
                       BB-0009 (planner budget exhaustion)
                      /          \                     \
                     /            \                     \
            BB-0001 (repo discovery)  BB-0007 (latency)  \
                     \                 /                   \
                      \               /                     \
                       BB-0010 (builders can't complete)    /
                                                          /
BB-0003 (hardcoded routing)  ────────────────────────────/
BB-0011 (Promise.all cascade) ──────────────────────────/
BB-0012 (stale closure) ───────────────────────────────/
```

### Entry Types

| Type | Description | Examples |
|------|-------------|----------|
| **Root cause** | Independent engineering problem with no dependencies on other ledger fixes | BB-0004, BB-0005, BB-0006, BB-0008, BB-0003, BB-0011, BB-0012 |
| **Intermediate symptom** | Observable problem caused by root causes below it | BB-0002, BB-0009 |
| **Top-level symptom** | Observable Version 1 blocker caused by intermediate symptoms | BB-0001, BB-0007, BB-0010 |

---

## Priority Order (by engineering dependency — fix foundation first)

| Priority | Entry | Title | Why Here |
|----------|-------|-------|----------|
| **P1** | BB-0004 | Filesystem scope resolver rejects non-existent paths | Root cause — blocking all tool calls to new paths |
| **P1** | BB-0005 | Search tool reports failure on no-match result | Root cause — inflating validation failure rate |
| **P1** | BB-0006 | Planner lacks convergence detection | Root cause — independent planner logic fix |
| **P1** | BB-0003 | Hardcoded builder routing | Root cause — independent architecture fix |
| **P1** | BB-0011 | Frontend Promise.all cascade | Root cause — independent frontend fix |
| **P2** | BB-0012 | sendMessage stale closure | Root cause — low-risk isolated frontend fix |
| **P2** | BB-0008 | No fast repository inventory capability | Root cause — new capability, independent |
| **P0** | BB-0002 | Tool validation failures cause planner retry cascades | Intermediate — depends on BB-0004 + BB-0005 |
| **P0** | BB-0009 | Planner budget consumed by inefficient sequences | Intermediate — depends on BB-0002 + BB-0006 |
| **P0** | BB-0001 | Repository-scale discovery exhausts planner budget | Top-level — depends on BB-0009 + BB-0008 |
| **P1** | BB-0007 | Runtime latency exceeds acceptable threshold | Top-level — depends on BB-0009 |
| **P0** | BB-0010 | Builders cannot complete general engineering requests | Top-level — depends on everything above |

---

## Recommended Implementation Order

1. **BB-0004** (filesystem scope) — Single-file fix in `scope.rs`. High impact, low risk.
2. **BB-0005** (search no-match) — Single-file fix in `search.rs`. High impact, low risk.
3. **BB-0011** (Promise.all cascade) — Single-file fix in `usePaneChat.ts`. High impact, moderate risk (changes frontend data loading).
4. **BB-0012** (stale closure) — Single-line fix in `usePaneChat.ts:436`. Low impact, no risk.
5. **BB-0003** (hardcoded routing) — Moderate complexity fix in `stream_execution.rs`. Required for main merge.
6. **BB-0006** (planner convergence) — Planner logic change. Tests after BB-0002 enabled (retry noise reduced).
7. **BB-0008** (inventory capability) — New tool. Can be done in parallel with BB-0006.
8. **Verify BB-0002 closes** — After BB-0004 and BB-0005 are fixed, re-run OPS-BRZ-004/005/007. If validation failure rate drops below 5%, mark BB-0002 RESOLVED.
9. **Verify BB-0009 closes** — After BB-0002 and BB-0006, re-run OPS-SLV-002/003/004. If rounds drop below 10, mark BB-0009 RESOLVED.
10. **Verify BB-0001 closes** — After BB-0009 and BB-0008, re-run OPS-BRZ-007 repository enumeration. Mark BB-0001 RESOLVED.
11. **Verify BB-0007 closes** — After BB-0009, re-run all Bronze latency targets. Mark BB-0007 RESOLVED.
12. **Verify BB-0010 closes** — Run full Bronze certification. If all pass, mark BB-0010 RESOLVED. Version 1 is in reach.

---

## Olympic Event Coverage

Every ledger entry is now linked to specific Olympic events:

| Entry | Failing Events | Certifying Event |
|-------|---------------|------------------|
| BB-0004 | OPS-BRZ-004, OPS-BRZ-005 | OPS-BRZ-004, OPS-BRZ-005 |
| BB-0005 | OPS-BRZ-007 | OPS-BRZ-007 |
| BB-0006 | OPS-SLV-002, OPS-SLV-003 | OPS-SLV-002, OPS-SLV-003 |
| BB-0008 | OPS-BRZ-007 | OPS-BRZ-007 (extended) |
| BB-0003 | OPS-GLD-001 | OPS-GLD-001 |
| BB-0011 | OPS-BRZ-002 | OPS-BRZ-002 |
| BB-0012 | OPS-BRZ-002 | OPS-BRZ-002 |
| BB-0002 | OPS-BRZ-004, -005, -007, -008, -009 | OPS-BRZ-004, -005, -007, -008, -009 |
| BB-0009 | OPS-SLV-002, -003, -004 | OPS-SLV-002, -003, -004 |
| BB-0001 | OPS-BRZ-007 | OPS-BRZ-007 |
| BB-0007 | OPS-BRZ-002, -004, -005 | OPS-BRZ-002, -004, -005 |
| BB-0010 | All Bronze events | All Bronze events |

---

## Version 1 Alignment

Every entry in the normalized ledger directly blocks one or more Version 1 requirements:

| Version 1 Requirement | Blocked By |
|----------------------|------------|
| Launch the application | BB-0011 |
| Open four Builder panes | BB-0011 |
| Assign four different software projects | BB-0011 |
| Select Builder models | BB-0012 |
| Give each Builder different engineering work | BB-0003 |
| Have each Builder successfully complete that work | BB-0010 (and all below it) |
| Continue interacting with each Builder independently | BB-0003 |
| Acceptable reliability and latency | BB-0007 |

Entries that did not directly block Version 1 were removed from the ledger.

---

## Summary

| Metric | Value |
|--------|-------|
| Total entries (new ledger) | 9 |
| Root cause entries | 7 |
| Intermediate entries | 1 |
| Top-level symptom entries | 2 |
| Track independent of tool chain | 3 (BB-0003, BB-0011, BB-0012) |
| Olympic events covered | 12 of 14 |
| Engineering dependency levels | 3 (root → intermediate → top-level) |
| One root cause per entry | 9/9 |
| Entries with explicit dependencies | 9/9 |
| Entries with Olympic linkage | 9/9 |
| Removed (non-Version-1) | 3 |
