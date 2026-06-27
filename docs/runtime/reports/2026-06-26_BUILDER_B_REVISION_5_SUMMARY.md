# BuilderBoard Runtime Framework — Revision 5

**Date:** 2026-06-26
**Builder:** B — Framework Engineer

---

## Summary

Revision 5 integrates Builder T's first Hypothesis Validation cycle into the Runtime Framework. The key insight from runtime evidence is that engineering hypotheses — not failures — are the fundamental unit of the ledger. Builder T is redefined from "Runtime Test Engineer" to "Runtime Experimentalist," and Builder V from "Runtime Validation Engineer" to "Runtime Evidence Validator." The engineering lifecycle now begins with a runtime observation that becomes a formal hypothesis, goes through experimentation, and may result in hypothesis refinement if evidence contradicts the original understanding.

---

## What Changed

### Modified Documents (10)

| Document | Changes |
|----------|---------|
| `docs/runtime/ENGINEERING_LAWS.md` | Extended Law 13 to explicitly state "Runtime experiments are the highest form of evidence." Extended Law 15 to document the hypothesis→experiment→refinement cycle. |
| `docs/runtime/BUILDER_T.md` | Redefined from "Runtime Test Engineer" to "Runtime Experimentalist." Updated all mission, responsibilities, workflow, rules, and output sections to reflect experimental design, hypothesis validation, and evidence-based conclusions. |
| `docs/runtime/BUILDER_V.md` | Redefined from "Runtime Validation Engineer" to "Runtime Evidence Validator." Validates evidence, not implementations. Assesses whether evidence supports or rejects engineering hypotheses. Controls RESOLVED→CLOSED based on evidence quality. |
| `docs/runtime/RUNTIME_WORKFLOW.md` | Lifecycle now starts with Runtime Observation + Engineering Hypothesis (Ledger Entry). Added hypothesis validation experiment step. Renamed "Regression Olympics" → "Runtime Experiment." Added Ledger Refinement step for hypothesis correction based on evidence. |
| `BUILDER_WORKFLOW.md` | Workflow diagram updated: "Runtime Olympics (Discovery)" → "Runtime Observation," "Engineering Ledger" → "Engineering Hypothesis (Ledger)," "Audits" → "Jules Investigation + Audits," "Builder T — Runtime Olympics" → "Builder T — Runtime Experiment," "Builder V — Runtime Validation" → "Builder V — Evidence Validation." Added Ledger Refinement step. Updated role handoff table, escalation, key principles, and ledger state transitions. |
| `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md` | Updated from "Three Forms of Evidence" to "Four Forms" (adding Runtime Experiment). Updated from "Four Types of Engineering Knowledge" to "Five Types" (adding Evidence Validation). Updated role definitions for Builder T (Experimentalist) and Builder V (Evidence Validator). Updated workflow summary, escalation path, ledger canonical status progression. Updated BB-0006 example to reflect three hypothesis revisions. |
| `docs/runtime/PHASE0_OLYMPICS.md` | Clarified three Olympic modes: Hypothesis Validation Experiments (test specific engineering hypothesis), Discovery Experiments (open-ended exploration), and Certification Olympics (formal tier qualification). Updated comparison table. |
| `AUDITS/README.md` | Added "Runtime Evidence (Builder T Experimental Reports)" category with references to Builder T Hypothesis Validation report and Convergence report. Updated philosophy section to include experiment and validation in the evidence stack. Added Runtime Evidence cross-references to ledger item and Olympic event indexes. |
| `RUNTIME_ENGINEERING_LEDGER.md` (BB-0006 entry) | Renamed from "Planner lacks convergence detection for repository-scale enumeration" to "Planner lacks error recovery and tool call adaptation." Updated observed/expected runtime based on Hypothesis Validation trace evidence. Added v3 hypothesis revision with direct runtime evidence of 10 identical failing calls. Updated all cross-references throughout the ledger. |
| `JULES.md` | Updated BB-0006 priority description to reflect renamed entry. |

### Unchanged Documents

| Document | Reason |
|----------|--------|
| `CORE_PROMISE.md` | Core Promise is stable. |
| `docs/runtime/RUNTIME_CERTIFICATION.md` | No certification has been executed yet (0% certified). |
| `docs/runtime/RUNTIME_FIRST_CHECKLIST.md` | Release checklist is process-independent. |
| `docs/runtime/RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification is forward-looking. |
| `docs/runtime/AUTOMATION_PLAN.md` | Automation is future work. |
| `docs/runtime/templates/` | Templates are format-level and independent of role structure. |

---

## Key Conceptual Changes

### Engineering Hypothesis as the Fundamental Unit

Previously, the ledger tracked "failures." Now it tracks **engineering hypotheses** — recorded observations with a proposed root cause explanation. Each hypothesis is tested by Builder T through controlled experimentation, producing evidence that either supports or contradicts it.

### Builder T: Test Engineer → Experimentalist

Builder T no longer merely executes predefined tests against known functionality. Builder T:
- Designs experiments with clear hypotheses and expected outcomes
- Executes experiments against the running application
- Interprets results: does evidence support or contradict the hypothesis?
- Documents the evidence and its implications for the ledger

### Builder V: Validation Engineer → Evidence Validator

Builder V no longer validates that "the implementation works." Builder V:
- Evaluates whether Builder T's experimental evidence is reproducible, sufficient, and correctly interpreted
- Assesses evidence quality, not code correctness
- Determines whether evidence supports or rejects the engineering hypothesis
- May recommend hypothesis refinement if evidence contradicts current understanding

### Hypothesis Refinement Is Expected, Not a Failure

When runtime evidence contradicts a hypothesis, the correct response is to refine the hypothesis — not to ignore the evidence or consider it a failed experiment. This is codified in Engineering Law 15. BB-0006's three revisions in a single day exemplify this: each was driven by runtime evidence, not speculation.

### Four Forms of Evidence

| Form | Purpose | Validity |
|------|---------|----------|
| **Investigation** | Plausible explanation for observed behavior | Necessary prerequisite |
| **Implementation** | Fix present in codebase | Necessary prerequisite |
| **Runtime Experiment** | Evidence that supports or contradicts hypothesis | Highest — produced by Builder T |
| **Evidence Validation** | Evidence is reproducible and correctly interpreted | Highest — confirmed by Builder V |

### Evidence Stack Hierarchy

```
Architecture Documents (intended design)
    ↓ informs
Runtime Engineering Ledger (engineering hypotheses)
    ↓ cross-references
AUDITS — Engineering Evidence Library (investigations)
    ↓ validates
Builder T — Runtime Experiments (hypothesis testing)
    ↓ validates
Builder V — Evidence Validation (reproducibility + interpretation)
    ↓ confirms
Ledger Refinement → Certification → Ship
```

---

## Evidence Driving Revision 5

### Builder T Hypothesis Validation Report

Builder T's first Hypothesis Validation cycle (documented in `AUDITS/2026-06-26_BUILDER_T_HYPOTHESIS_VALIDATION.md`) produced:

- **16 experiments designed** across 4 themes
- **4 experiments completed** (from runtime traces)
- **3 themes analyzed**: scope validation, search failure, planner error recovery
- **12 experiments pending** live runtime execution

### Critical Finding: BB-0006 Root Cause Correction

Execution 2 of the Hypothesis Validation trace revealed:
- 10 identical `filesystem.write` calls with the same validation error
- Zero adaptation across all 10 rounds
- No alternative tool attempted
- No graceful degradation

This directly contradicts the convergence detection hypothesis and the prompt completion hypothesis. The planner *converges* correctly but *recovers* from errors incorrectly. The entry was renamed and the hypothesis revised to v3.

---

## Updated Workflow

```
Runtime Observation (anyone)
    ↓
Engineering Hypothesis (Runtime Ledger entry — OPEN)
    ↓
Roadmap Gate
    ↓
Jules Investigation + AUDITS — Engineering Evidence
    ↓
Builder C — Architecture Review
    ↓
Jules — Implementation
    ↓
Builder C — Implementation Review (→ IMPLEMENTED)
    ↓
Builder T — Runtime Experiment (→ RESOLVED)
    ↓
Builder V — Evidence Validation (→ VALIDATED)
    ↓
Runtime Ledger Refinement (hypothesis may be refined)
    ↓
Certification (if tier complete)
    ↓
Release Checklist → Ship
```

---

## Updated Ledger State Lifecycle

```
OPEN (hypothesis recorded)
  ↓ (Investigation + Architecture Review by Builder C)
IMPLEMENTED
  ↓ (Implementation Review by Builder C)
RESOLVED (Pending Runtime Certification)
  ↓ (Runtime Experiment by Builder T — evidence supports/contradicts)
VALIDATED
  ↓ (Builder V validates evidence — supports or rejects)
  ↓ (if contradicted → return to IMPLEMENTATION)
CLOSED
```

---

## How Revision 5 Changes Engineering Practice

1. **Every ledger entry is a hypothesis.** Before writing it, ask: "What is the engineering hypothesis about root cause? How will we test it?"

2. **Builder T designs experiments before executing them.** Each experiment has a clear hypothesis, expected outcome, and pass/fail criteria. Open-ended exploration is Discovery mode; targeted hypothesis testing is the default.

3. **Builder V validates evidence, not code.** Builder V asks: "Is this evidence reproducible? Is it sufficient to support the hypothesis? Has it been correctly interpreted?"

4. **Hypothesis refinement is part of the lifecycle.** If evidence contradicts a hypothesis, the ledger entry is updated — not the fix reverted. This is Engineering Law 15 in practice.

5. **Runtime evidence is the highest form of evidence.** A well-designed experiment that produces contradictory evidence overrides all architectural assumptions, code reviews, and implementation evidence (Law 13).

---

## New Engineer Onboarding Path

1. `README.md` — project overview and Core Promise
2. `docs/runtime/CORE_PROMISE.md` — the single mission
3. `docs/runtime/ENGINEERING_LAWS.md` — 15 permanent rules
4. `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md` — complete handbook with role definitions, workflow, and ledger lifecycle
5. `BUILDER_WORKFLOW.md` — concrete workflow diagram with role handoffs
6. `docs/runtime/BUILDER_T.md` — Runtime Experimentalist role
7. `docs/runtime/BUILDER_V.md` — Runtime Evidence Validator role
8. `JULES.md` — Implementation Engineer context
9. `docs/runtime/PHASE0_OLYMPICS.md` — how runtime is evaluated
10. `AUDITS/2026-06-26_BUILDER_T_HYPOTHESIS_VALIDATION.md` — first Hypothesis Validation in practice
