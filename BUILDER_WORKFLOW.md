# Builder Engineering Workflow

*The intended workflow for every engineering change in BuilderBoard.*

---

## Overview

Every change — whether a bug fix, performance improvement, or feature implementation — follows the same workflow. This workflow ensures that runtime behavior is always the primary measure of quality and that no change ships without certification.

---

## The Workflow

```
Runtime Olympics (Discovery)
    │
    ▼
┌─────────────────────────────────────┐
│ 1. RUNTIME ENGINEERING LEDGER        │
│                                     │
│ Record the failure. Include:        │
│ - Observed/expected behavior         │
│ - Root cause analysis               │
│ - Olympic event linkage              │
│ - Affected files                    │
│ - Priority and status: OPEN         │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 2. ROADMAP GATE                     │
│                                     │
│ Builder C checks current            │
│ certification level. Is the runtime │
│ certified at the level this fix     │
│ requires?                           │
│                                     │
│ ├─ YES → Proceed to architecture    │
│ └─ NO  → Fix runtime first.        │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 3. BUILDER C — ARCHITECTURE REVIEW  │
│                                     │
│ Validates root cause.               │
│ Approves implementation approach.   │
│ Identifies Olympic events for cert. │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 4. JULES — IMPLEMENTATION           │
│                                     │
│ Writes and commits the fix.         │
│ Runs regression tests.              │
│ Creates Pull Request for review.    │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 5. BUILDER C — IMPLEMENTATION REVIEW│
│                                     │
│ Confirms fix matches architecture.  │
│ Unit tests pass. Compiler passes.   │
│ Status: IMPLEMENTED                 │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 6. BUILDER T — RUNTIME OLYMPICS     │
│                                     │
│ Execute Olympic events linked to    │
│ this fix against running app.       │
│ Measure latency, correctness, etc.  │
│ Record PASS/FAIL.                   │
│ Status: RESOLVED (Pending Cert)     │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 7. BUILDER V — RUNTIME VALIDATION   │
│                                     │
│ Independently repeat each event.    │
│ Confirm or dispute PASS/FAIL.       │
│ Approve or reject closure.          │
│ Status: VALIDATED (if confirmed)    │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 8. CERTIFICATION (if tier complete) │
│                                     │
│ Builder C reviews all evidence.     │
│ Issues certification at tier level. │
│ Status: CLOSED (per entry)          │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 9. RELEASE CHECKLIST                 │
│                                     │
│ Run RUNTIME_FIRST_CHECKLIST.md.     │
│ Builder C, T, V all sign.           │
│ ├─ All YES → Ship                   │
│ └─ Any NO  → Fix → Recertify       │
└─────────────────────────────────────┘
```

---

## Key Principles

### Implementation Truth vs Runtime Truth

Implementation does not prove success. Runtime does.

A fix that compiles, passes unit tests, and looks correct in code review is **implemented** — not **resolved**. Resolution requires runtime evidence: the fix must be observed working in the running application under realistic conditions.

### Runtime Failure Always Goes to the Ledger First

Before any code is written, the failure must be recorded in the ledger. This ensures:
- Traceability from failure to fix.
- No fix is attempted without understanding the root cause.
- Historical data accumulates for trend analysis.

### Olympic Events Before Close

No issue is closed until the corresponding Olympic event passes against the running application. Compilation and unit test success are insufficient.

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
Step                  Performed By         Delivers To
──────────────────────────────────────────────────────
1. Ledger             Anyone                Builder C
2. Roadmap Gate       Builder C             — (decision)
3. Architecture Rev.  Builder C             Jules
4. Implementation     Jules                 Builder C
5. Implementation Rev Builder C             Builder T
6. Regression Olympics Builder T            Builder V
7. Validation         Builder V             Builder C
8. Certification      Builder C             Repository
9. Release Checklist  All Builders          Repository
```

---

## Escalation

If Builder T and Builder V disagree on an Olympic result:

1. Builder C reviews both sets of evidence.
2. Builder C may request retesting.
3. Builder C makes a final determination.
4. The resolution is documented in the ledger.

If a change causes a regression (an Olympic event that previously passed now fails):

1. The regression is recorded in the ledger.
2. All feature work stops (Engineering Law 7).
3. The regression is fixed before any further feature development.

---

## Ledger State Transitions

```
OPEN
  ↓ Architecture Review by Builder C
IMPLEMENTED
  ↓ Implementation Review by Builder C
RESOLVED (Pending Runtime Certification)
  ↓ Regression Olympics by Builder T
VALIDATED
  ↓ Builder V confirms runtime evidence
CLOSED
```

Implementation alone never closes a ledger entry. Every transition from RESOLVED onward requires runtime evidence.
