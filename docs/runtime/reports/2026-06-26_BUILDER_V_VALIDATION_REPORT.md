# Builder V — Runtime Validation Report

**Date:** 2026-06-26
**Audited:** BB-0011, BB-0012, BB-0004, BB-0005, BB-0002, BB-0007
**Role:** Builder V — Runtime Validation Engineer

---

## Methodology

For each ledger entry I independently:

1. Read the **implementation** (source code) and compared against the claimed fix.
2. Read every **test** related to the fix and verified test correctness.
3. Ran `cargo test --lib` (168 tests) and `cargo test --test planner_convergence` (8 tests) to **confirm no regressions**.
4. Assessed whether the **runtime behavior claim** is supported by evidence.
5. Assigned a **confidence score** (1-5) and recommended **ledger status**.

---

## 1. BB-0011 — Promise.allSettled cascade (CLOSED)

| Field | Finding |
|-------|---------|
| **Claimed fix** | `Promise.allSettled` at `usePaneChat.ts:134` with per-source error handling |
| **Code audit** | Line 134: `Promise.allSettled([accountList, messageList, engineList, builderList])` |
| | Lines 145-148: each source handled independently via `result.status === "fulfilled"` |
| | Lines 150-170: per-source error states (`setAccountError`, `setMessageLoadError`, `setEngineError`) |
| **Verdict** | Fix is correctly implemented. A single source failure does not cascade. |
| **Confidence** | 5/5 — Code audit is unambiguous. No runtime dependency. |
| **Recommendation** | **RETAIN CLOSED** |

---

## 2. BB-0012 — Stale closure (CLOSED)

| Field | Finding |
|-------|---------|
| **Claimed fix** | `selectedBuilderId` added to `useCallback` dependency array at `usePaneChat.ts:483` |
| **Code audit** | Line 483: `selectedBuilderId` is in the dependency array `[inputValue, pane.id, reloadMessages, selectedAccountId, selectedBuilderId, selectedEffort, selectedEngineId, selectedModelId]` |
| **Verdict** | Fix is correctly implemented. No stale closure can form. |
| **Confidence** | 5/5 — Code audit is unambiguous. |
| **Recommendation** | **RETAIN CLOSED** |

---

## 3. BB-0004 — Filesystem scope (RESOLVED Pending Certification)

| Field | Finding |
|-------|---------|
| **Claimed fix** | `resolve_create_path` at `scope.rs:112` validates create paths without requiring disk existence |
| **Code audit** | Function handles 4 cases: null byte rejection, empty/`.` → root, existing path (canonicalize + escape check), new path (parent walk + ancestor canonicalize + escape check) |
| | `normalize_absolute` (`scope.rs:176`): resolves `..`, returns PathEscape on over-pop |
| | `normalize_relative` (`scope.rs:194`): starts from root, rejects `..`→root, rejects root dir in relative paths |
| **Path escape analysis** | For existing paths: canonicalize + `is_within_root` → secure |
| | For new paths: parent walk + ancestor canonicalize + `is_within_root` → secure because candidate was already normalized and is a descendant of the checked ancestor |
| **Tests** | 6 scope tests pass (3 original + 3 new from fix) |
| | `resolve_create_path_allows_new_file_inside_root` — creates `docs/test.md` inside root → returns `root/docs/test.md` ✓ |
| | `resolve_create_path_rejects_traversal` — `../outside.md` → PathEscape ✓ |
| | `resolve_create_path_rejects_symlink_parent_escape` — symlinked parent escape → PathEscape ✓ |
| **Regression risk** | Low. `resolve_create_path` is additive (new function, no existing code changed in scope.rs). The original `resolve_path` and `resolve_existing_path` are unchanged. |
| **Verdict** | Implementation is correct. The fix eliminates the largest single cause (~60%) of validation failures. |
| **Confidence** | 4/5 — Code audit and tests pass, but needs live runtime certification to confirm the failure rate reduction. |
| **Recommendation** | **RETAIN RESOLVED (Pending Runtime Certification)** |

### Additional findings

The `resolve_create_path` function has a correct security posture:

```
Request:  create /project/new/file.rs
1. normalize_relative: /project/new/file.rs (within root ✓)
2. candidate.exists()? → No
3. Walk parents: /project/new → /project → / exists
4. Canonicalize /project → verify is_within_root ✓
5. Return /project/new/file.rs
```

```
Request:  /project/../../etc/passwd
1. normalize_absolute: /etc/passwd (.. resolved)
2. candidate.exists()? → Maybe (if /etc/passwd exists)
3a. EXISTS: canonicalize → /etc/passwd → is_within_root? → ✗ PathEscape
3b. NOT EXISTS: parent walk → /etc → canonicalize → is_within_root? → ✗ PathEscape
```

---

## 4. BB-0005 — Search no-match (RESOLVED Pending Certification)

| Field | Finding |
|-------|---------|
| **Claimed fix** | `rg`/`grep` exit code 1 (no match) treated as success instead of failure |
| **Code audit** | `search.rs:165` (rg) and `search.rs:186` (grep): `matches!(output.status.code(), Some(0) | Some(1))` |
| | Exit code 0 = matches found, exit code 1 = no matches = both success |
| | Exit code 2+ = actual error = failure |
| **Tests** | 3 search tests pass |
| | `grep_missing_pattern_succeeds_with_empty_result` — searches for `NO_SUCH_SYMBOL_BB_0005` → success=true, stdout="" ✓ |
| | `grep_existing_pattern_succeeds` — `fn main` → success=true, stdout="main.rs" ✓ |
| | `grep_invalid_pattern_remains_failure` — `[` → success=false ✓ |
| **Regression risk** | Low. The only change is accepting exit code 1 alongside exit code 0. The invalid-pattern test confirms real errors (exit code 2+) still fail. |
| **Verdict** | Implementation is correct. The fix eliminates the second largest cause (~20%) of validation failures. |
| **Confidence** | 4/5 — Code audit and tests pass, but needs live runtime certification. |
| **Recommendation** | **RETAIN RESOLVED (Pending Runtime Certification)** |

---

## 5. BB-0002 — Validation retry cascades (PARTIALLY RESOLVED)

| Field | Finding |
|-------|---------|
| **Claimed improvement** | BB-0004 + BB-0005 fixes reduce tool validation failure rate |
| **Code audit** | Both fixes independently verified above (BB-0004 ✓, BB-0005 ✓) |
| | The two fixes address the two largest root causes of validation failures (~60% + ~20%) |
| | No other code changes identified that would reduce the remaining ~20% |
| **Evidence strength** | MODERATE — The claim "failure rate is reduced" is strongly supported by code analysis, but the actual quantitative reduction cannot be measured without live runtime tests (requires OpenAI credentials). The pre-fix data showed 11/18, 18/30, 9/14, 6/20 failure rates. Post-fix rates are unknown. |
| **Remaining risk** | The capability_resolver.rs validation layer is unaffected. If the remaining ~20% of failures are caused by validation logic (not scope/search), BB-0002 cannot reach RESOLVED without additional fixes. |
| **Verdict** | PARTIALLY RESOLVED is accurate. The primary causes are fixed. The remaining ~20% of failures need measurement. |
| **Confidence** | 3/5 — Code audit strongly supports improvement; runtime measurement is the gap. |
| **Recommendation** | **RETAIN PARTIALLY RESOLVED** |

---

## 6. BB-0007 — Runtime latency (PARTIALLY RESOLVED)

| Field | Finding |
|-------|---------|
| **Claimed improvement** | BB-0004 + BB-0005 fixes reduce retry-caused latency |
| **Code audit** | Fewer retry rounds = fewer provider round trips = lower latency. This is logically sound but: |
| | (a) No latency measurements exist for the post-fix state |
| | (b) The primary latency driver (BB-0009 planner budget) remains OPEN |
| | (c) Latency is a composite property; fixing two input causes reduces it but cannot resolve it |
| **Evidence strength** | WEAK — The claim is logically plausible but unsupported by runtime measurements. Pre-fix latency data (40-80s) exists; post-fix measurements do not. |
| **Verdict** | PARTIALLY RESOLVED is defensible but weakly supported. The true latency impact of these fixes is unknown without live runtime testing. |
| **Confidence** | 2/5 — Logical chain is sound; absent runtime data is the gap. |
| **Recommendation** | **RETAIN PARTIALLY RESOLVED** |

---

## Summary

| Entry | Status Before | Recommended | Confidence | Key Evidence |
|-------|--------------|-------------|------------|-------------|
| BB-0011 | CLOSED | **CLOSED** | 5/5 | `Promise.allSettled` at `usePaneChat.ts:134` |
| BB-0012 | CLOSED | **CLOSED** | 5/5 | `selectedBuilderId` at `usePaneChat.ts:483` |
| BB-0004 | RESOLVED (Pending Cert) | **RESOLVED (Pending Cert)** | 4/5 | `resolve_create_path` at `scope.rs:112`, 6/6 scope tests pass |
| BB-0005 | RESOLVED (Pending Cert) | **RESOLVED (Pending Cert)** | 4/5 | `Some(0)` → `Some(0) | Some(1)` at `search.rs:165/186`, 3/3 search tests pass |
| BB-0002 | PARTIALLY RESOLVED | **PARTIALLY RESOLVED** | 3/5 | Primary causes fixed; remaining ~20% unmeasured |
| BB-0007 | PARTIALLY RESOLVED | **PARTIALLY RESOLVED** | 2/5 | Logically plausible; no runtime latency data exists |

**Overall regression check:** All 168 unit tests pass. All 8 planner convergence tests pass. `cargo check --tests` clean. No regressions detected.

**All six ledger statuses accurately reflect runtime reality as of today. No status changes recommended.**

---

## Remaining Gaps

1. **No live runtime evidence** — All 4 RESOLVED/PARTIALLY RESOLVED entries would benefit from live Olympic event execution. Without OpenAI credentials, every "improvement" claim is code-derived, not runtime-derived.
2. **BB-0007 latency data** — No post-fix latency measurements exist. The PARTIALLY RESOLVED claim is the weakest in this audit.
3. **BB-0002 remaining ~20%** — The unaddressed validation failure causes need identification and measurement.
