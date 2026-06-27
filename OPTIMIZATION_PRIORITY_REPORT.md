# BuilderBoard Version 1: Optimization Prioritization Report

**Date:** 2026-06-27
**Target:** BuilderBoard Version 1
**Audience:** Builder C (Architecture & Implementation)

---

## Executive Summary

This report synthesizes all known optimization opportunities across the BuilderBoard repository. It analyzes architectural findings (Phase 8.9 series) and runtime findings (Runtime Engineering Ledger, IV&V Reports) to recommend an engineering roadmap for Builder C.

The primary goal is to prioritize optimizations that directly resolve Version 1 blockers (as defined in the Core Definition and Current Deficiencies) and to defer optimizations that, while valuable, do not block the Core Promise.

---

## Theme 1: Planner Efficiency & Convergence

**Description:** The execution engine's LLM loop (the "planner") consumes excessive rounds, fails to terminate when sufficient information is gathered, and hits hard budget limits (10 rounds).
**Why it matters:** This directly causes high latency (40-80s) and prevents Builders from completing complex, multi-tool engineering tasks. It is the root cause of multiple Version 1 failures.
**Supporting Evidence:**
- **Ledger:** BB-0006 (Planner convergence detection), BB-0009 (Planner budget exhaustion).
- **Runtime Evidence:** `2026-06-26_BUILDER_T_CONVERGENCE_REPORT.md` proves that the issue is not the loop logic itself, but the prompt failing to instruct the LLM to stop calling tools.
- **Deficiencies:** Item #4 (Planner efficiency is insufficient) and Item #5 (Runtime latency is too high).
**Runtime Risk:** High (impacts all tool executions).
**Implementation Complexity:** Low (prompt engineering change in `stream_execution.rs`).
**Confidence:** 95/100 (Runtime tests confirm the fix direction).
**Recommended Priority:** **P0**
**Recommendation:** **Implement** (Update system prompt to include a convergence directive).

---

## Theme 2: Repository Discovery & Inventory Optimization

**Description:** Builders struggle to understand repository-scale structure efficiently, relying on repeated, inefficient search cycles rather than a fast, comprehensive inventory tool.
**Why it matters:** Blocks "understanding a software project" and "searching code" at repository scale, which are core requirements for a software engineering assistant.
**Supporting Evidence:**
- **Ledger:** BB-0001 (Repository-scale discovery missions exhaust planner budget), BB-0008 (Repository inventory capability).
- **Deficiencies:** Item #2 (Repository discovery is unreliable).
- **Runtime Evidence:** Yes (Olympic test OPS-BRZ-007 fails).
**Runtime Risk:** Medium.
**Implementation Complexity:** Medium (requires building a new fast-inventory tool in Rust).
**Confidence:** 90/100.
**Recommended Priority:** **P0**
**Recommendation:** **Implement** (Develop a dedicated `repository.inventory` or `directory.tree` tool to replace multi-step search discovery).

---

## Theme 3: Database & State Optimization

**Description:** The `StreamWriteBuffer` holds database locks across multiple queries, and the frontend uses `Promise.all` which creates fragile data loading cascades.
**Why it matters:** While DB locks are an architectural bottleneck, the `Promise.all` cascade actively breaks the UI on load.
**Supporting Evidence:**
- **Architectural Evidence:** `PHASE_8_9A_EXECUTION_ARCHITECTURE.md` calls out DB batching as HIGH priority.
- **Runtime Evidence:** `PHASE_8_9F2_RUNTIME_INVESTIGATION.md` identified `Promise.all` as the root cause of data loading failures, which was later fixed (BB-0011).
**Runtime Risk:** Low (DB locks), Medium (Frontend state).
**Implementation Complexity:** Low (batch DB writes).
**Confidence:** 85/100.
**Recommended Priority:** **P2**
**Recommendation:** **Measure** (Monitor DB contention; implement batch writes if contention spikes. The frontend `Promise.all` issue is already CLOSED).

---

## Theme 4: Execution Engine Architecture & UX Parity

**Description:** Upgrading the execution loop to support advanced agentic features (multi-turn error recovery, token budget tracking, context sliding windows) and adding missing UX elements (status bars, diff previews, tool call cards).
**Why it matters:** Improves LLM reliability and user trust, but represents significant architectural rework.
**Supporting Evidence:**
- **Architectural Evidence:** `PHASE_8_9A_EXECUTION_ARCHITECTURE.md` (Inner loop rebuild, sliding window), `PHASE_8_9C_ENGINE_UX_ARCHITECTURE.md` (Missing UX components).
- **Runtime Evidence:** None. These are architectural gaps against market competitors, not runtime failures against the Core Definition.
**Runtime Risk:** High (Requires rebuilding the core execution orchestrator and React components).
**Implementation Complexity:** High (Weeks of effort).
**Confidence:** 90/100.
**Recommended Priority:** **P3**
**Recommendation:** **Defer** (These are Phase 9C/9D features. Version 1 does not strictly require advanced UX or complex context sliding to fulfill the Core Promise of four independent panes).

---

## Theme 5: Hardcoded Routing & Builder Extensibility

**Description:** The routing logic in `stream_execution.rs` relies on hardcoded string matching (`"builder-a"`, `"builder-b"`) instead of dynamic registry lookup, bypassing the `ExecutionManager`.
**Why it matters:** Prevents dynamic builder creation and represents a brittle point of failure if new providers are added.
**Supporting Evidence:**
- **IV&V Report:** `PHASE_8_9F_IVV_REPORT.md` flags this as a CRITICAL defect blocking merge.
- **Ledger:** BB-0003 (Hardcoded builder routing).
- **Runtime Evidence:** Partially architectural, but flagged as a blocker for multi-pane Olympics (OPS-GLD-001).
**Runtime Risk:** Medium.
**Implementation Complexity:** Low (Replace hardcoded strings with `global_builder_registry().get()`).
**Confidence:** 100/100.
**Recommended Priority:** **P1**
**Recommendation:** **Implement** (Fix the routing defect to unblock Builder extensibility and Gold tier Olympics).

---

## Summary of Recommendations

1. **P0 - Implement:** Fix Planner Convergence (BB-0006) via prompt engineering.
2. **P0 - Implement:** Build Repository Inventory Tool (BB-0008) to fix discovery failures.
3. **P1 - Implement:** Remove hardcoded routing (BB-0003) to enable builder extensibility.
4. **P2 - Measure:** DB write batching (Phase 8A finding).
5. **P3 - Defer:** Advanced UX components and execution loop rebuilds (Phase 8C/8A findings) until Version 1 Core Promise is met.
