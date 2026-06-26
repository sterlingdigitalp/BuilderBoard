# Builder T — Planner Convergence Test Report

**Date:** 2026-06-26
**Target:** BB-0006 (Planner lacks convergence detection)
**Role:** Builder T — Runtime Test Engineer
**Status:** Test Design Complete — 8/8 tests passing

---

## Summary

Designed and executed the first targeted convergence test suite for BB-0006.
The tests validate the planner loop's ability to detect when it has gathered
sufficient information and should stop calling tools — vs. blindly hitting the
hard `max_tool_rounds` limit.

---

## Test Results

| Test | Rounds | Converged | Status |
|------|--------|-----------|--------|
| Immediate convergence (1 round) | 1 | ✓ | PASS |
| Multi-round convergence (2 rounds) | 2 | ✓ | PASS |
| Multi-round convergence (5 tool calls + final) | 6 | ✓ | PASS |
| Multi-round convergence (9 + 1 = boundary) | 10 | ✓ | PASS |
| Single tool call then text | 2 | ✓ | PASS |
| Multiple unique tools chain (search → read → list → text) | 4 | ✓ | PASS |
| Non-convergence (always tool calls) | 10 | ✗ | PASS |
| Empty response guard (test integrity) | - | - | PASS |

**All 8 tests pass.**

---

## Key Findings

### 1. Convergence algorithm is simple and correct

The convergence check is a single conditional: `if tool_calls.is_empty() { done }`.
This is correct by construction — when the LLM decides it has enough information,
it stops calling tools.

### 2. The real issue is *delayed* convergence, not *absent* convergence

BB-0006's title says "planner lacks convergence detection."  This is misleading.
The planner *does* detect convergence (it checks `tool_calls.is_empty()` every
round).  The real problem is that the LLM *chooses* to keep calling tools beyond
what a human would consider sufficient, because:

- The tool-calling prompt encourages exhaustive tool use.
- There is no "I have enough information, here is the answer" signal injected
  into the system prompt.
- Each round *resets* the LLM context window — the model doesn't see its own
  prior text as a complete answer, only as partial progress.

### 3. max_rounds = 10 is both safety net and failure mode

The hard limit prevents infinite loops.  But hitting it produces a generic
fallthrough message instead of whatever the LLM had computed.  Tests confirm
that non-convergent planners cleanly reach round 10.

---

## Recommended Fix Direction

Based on test findings, the convergence issue can be addressed through
**prompt engineering alone** (no planner loop changes needed):

1. **Add a system prompt directive:**
   `"Once you have gathered enough information to answer the user's request,
    stop calling tools and provide the final answer."`

2. **Inject an explicit "decision round" signal:**
   After N rounds of tool usage, inject a system message:
   `"You have used {N} tool(s). Do you have enough information to answer?
    If yes, provide the final answer. If no, continue with specific remaining
    questions."`

3. **Implement semantic convergence hint:**
   Before each round, the planner could inject a context-length / budget usage
   hint so the LLM can make an informed decision about whether to continue.

These changes belong in `stream_execution.rs` (the system prompt/context
construction for each round) — not in the planner loop itself.

---

## Tests Created

| File | Description |
|------|-------------|
| `src-tauri/tests/planner_convergence.rs` | 8 pure-logic convergence tests |
| `docs/runtime/tests/OPS-CON-001_PLANNER_CONVERGENCE.md` | Olympic test specification |

---

## Next Steps

1. **Promote to Bronze certification:** Move OPS-CON-001-A through OPS-CON-001-E
   into the Bronze certification suite in `PHASE0_OLYMPICS.md`.
2. **Implement convergence prompt fix:** Add system prompt directive in
   `stream_execution.rs` (Builder C / Jules).
3. **Re-run live Olympics:** After prompt fix, re-run OPS-SLV-001 through
   OPS-SLV-004 against live runtime to verify reduced round counts.
4. **Re-evaluate BB-0006 priority:** If prompt fix eliminates >90% of excess
   rounds, consider reducing priority or closing as RESOLVED.
