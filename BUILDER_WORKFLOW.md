# Builder Engineering Workflow

*The intended workflow for every engineering change in BuilderBoard.*

---

## Overview

Every change — whether a bug fix, performance improvement, or feature implementation — follows the same workflow. This workflow ensures that runtime behavior is always the primary measure of quality and that no change ships without certification.

---

## The Workflow

```
Runtime Observation (anyone)
    │
    ▼
┌────────────────────────────────────────────────┐
│ 1. ENGINEERING HYPOTHESIS (LEDGER)             │
│                                                │
│ Record the observation as a hypothesis.        │
│ Include:                                       │
│ - Observed behavior / expected behavior         │
│ - Root cause hypothesis                        │
│ - Olympic event linkage                        │
│ - Affected files                               │
│ - Status: OPEN                                 │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 2. ROADMAP GATE                                │
│                                                │
│ Builder C checks current certification level.  │
│ Is the runtime certified at the level this     │
│ fix requires?                                  │
│                                                │
│ ├─ YES → Proceed to investigation              │
│ └─ NO  → Fix runtime first.                    │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 3. JULES INVESTIGATION + AUDITS                │
│                                                │
│ Investigate root cause. Review existing        │
│ audits for relevant evidence. Produce new      │
│ audit if none exists.                          │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 4. BUILDER C — ARCHITECTURE REVIEW             │
│                                                │
│ Validates root cause hypothesis against        │
│ investigation and audit evidence.              │
│ Approves implementation approach.              │
│ Identifies Olympic events for certification.   │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 5. JULES — IMPLEMENTATION                      │
│                                                │
│ Writes and commits the fix.                    │
│ Runs regression tests.                         │
│ Creates Pull Request for review.               │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 6. BUILDER C — IMPLEMENTATION REVIEW           │
│                                                │
│ Confirms fix matches architecture.             │
│ Unit tests pass. Compiler passes.              │
│ Status: IMPLEMENTED                            │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 7. BUILDER T — RUNTIME EXPERIMENT              │
│                                                │
│ Design and execute experiment to test the      │
│ hypothesis. Measure observed runtime behavior. │
│ Determine: does evidence support or            │
│ contradict the hypothesis?                     │
│ Status: RESOLVED (Pending Cert)                │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 8. BUILDER V — EVIDENCE VALIDATION             │
│                                                │
│ Independently validate Builder T's evidence.   │
│ Confirm or dispute: does evidence support or   │
│ reject the hypothesis?                         │
│ Approve or reject closure based on evidence.   │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 9. LEDGER REFINEMENT + CERTIFICATION           │
│                                                │
│ Hypothesis refined if evidence contradicted    │
│ original understanding. If tier complete,      │
│ Builder C issues certification at tier level.  │
│ Status: CLOSED (per entry)                     │
└───────────────────┬────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ 10. RELEASE CHECKLIST                           │
│                                                │
│ Run RUNTIME_FIRST_CHECKLIST.md.                │
│ Builder C, T, V all sign.                      │
│ ├─ All YES → Ship                              │
│ └─ Any NO  → Fix → Recertify                  │
└────────────────────────────────────────────────┘
```

---

## Key Principles

### Implementation Truth vs Runtime Truth

Implementation does not prove success. Runtime does.

A fix that compiles, passes unit tests, and looks correct in code review is **implemented** — not **resolved**. Resolution requires runtime evidence: the fix must be observed working in the running application under realistic conditions.

### Every Runtime Observation Becomes an Engineering Hypothesis

Before any code is written, the observation must be recorded in the ledger as an engineering hypothesis. This ensures:
- Traceability from observation to evidence.
- No fix is attempted without a clear hypothesis about root cause.
- Historical data accumulates for trend analysis.
- Hypotheses can be invalidated by runtime evidence without triggering unnecessary rework.

### Runtime Evidence Before Close

No issue is closed until runtime evidence supports the engineering hypothesis. Compilation and unit test success are insufficient. Builder T produces evidence through experimentation. Builder V validates that evidence.

Authenticated Olympic evidence must come from the packaged, locally signed runtime:

```text
/Applications/BuilderBoard Dev.app
```

Evidence from `npm run dev`, `cargo tauri dev`, or `target/debug/builderboard` is invalid for authenticated provider workflows because those binaries use unstable macOS Keychain identities.

### Certification Before Ship

No release ships without current runtime certification at the appropriate level. The release checklist (`RUNTIME_FIRST_CHECKLIST.md`) must pass.

---

## Role Handoff

```
Step                      Performed By         Delivers To
───────────────────────────────────────────────────────────
1. Hypothesis (Ledger)    Anyone                Builder C
2. Roadmap Gate           Builder C             — (decision)
3. Investigation + Audits Builder C / Jules     Builder C
4. Architecture Review    Builder C             Jules
5. Implementation         Jules                 Builder C
6. Implementation Review  Builder C             Builder T
7. Runtime Experiment     Builder T            Builder V
8. Evidence Validation    Builder V             Builder C
9. Ledger Refinement +    Builder C             Repository
   Certification
10. Release Checklist      All Builders         Repository
```

---

## Escalation

If Builder T and Builder V disagree on experimental evidence:

1. Builder C reviews both sets of evidence.
2. Builder C may request retesting.
3. Builder C makes a final determination.
4. The resolution is documented in the ledger.

If a change causes a regression (an experimental result or Olympic event that previously passed now fails):

1. The regression is recorded in the ledger.
2. All feature work stops (Engineering Law 7).
3. The regression is fixed before any further feature development.

---

## Ledger State Transitions

```
OPEN (hypothesis recorded)
  ↓ Investigation + Architecture Review by Builder C
IMPLEMENTED
  ↓ Implementation Review by Builder C
RESOLVED (Pending Runtime Certification)
  ↓ Runtime Experiment by Builder T — evidence supports/contradicts hypothesis
VALIDATED
  ↓ Builder V validates evidence — supports or rejects hypothesis
  ↓ If hypothesis contradicted by evidence → return to IMPLEMENTATION
CLOSED
```

Implementation alone never closes a ledger entry. Every transition from RESOLVED onward requires runtime evidence. If Builder T's experiment or Builder V's validation produces evidence that contradicts the original hypothesis, the hypothesis must be refined and the implementation revised — this is not a failure, it is the expected engineering process (Law 15).
