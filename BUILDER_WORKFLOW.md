# Builder Engineering Workflow

*The intended workflow for every engineering change in BuilderBoard.*

---

## Overview

Every change — whether a bug fix, performance improvement, or feature implementation — follows the same workflow. This workflow ensures that runtime behavior is always the primary measure of quality and that no change ships without certification.

## The Workflow

```
Runtime Failure or Feature Request
    │
    ▼
┌─────────────────────────────────────┐
│ 1. RUNTIME LEDGER                    │
│                                     │
│ Record the issue or request in the   │
│ Runtime Engineering Ledger. Include: │
│ - Observed/expected behavior         │
│ - Olympic event linkage              │
│ - Affected files (where known)       │
│ - Priority and status                │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 2. ROADMAP GATE                     │
│                                     │
│ Check current certification level.  │
│ Is the runtime certified at the     │
│ level this change requires?         │
│                                     │
│ ├─ YES → Proceed to architecture    │
│ └─ NO  → Fix runtime first.        │
│          Feature work paused.       │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 3. ARCHITECTURE                      │
│                                     │
│ Design the change. Consider:        │
│ - Does this preserve the Core       │
│   Promise?                          │
│ - Does this make runtime behavior   │
│   better for the user?              │
│ - What Olympic events will verify   │
│   this change?                      │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 4. IMPLEMENTATION                    │
│                                     │
│ Write code. Follow Runtime First:   │
│ - Implementation is fungible        │
│ - Working runtime > clean code      │
│ - No feature weakens the Core       │
│   Promise                           │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 5. RUNTIME OLYMPICS                  │
│                                     │
│ Builder T executes Olympic events   │
│ against the running application.    │
│                                     │
│ - Launch the packaged runtime       │
│ - Execute affected events           │
│ - Execute full suite if possible    │
│ - Record metrics in ledger          │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 6. VALIDATION                        │
│                                     │
│ Builder V validates Builder T's     │
│ results independently:              │
│ - Repeat each event                 │
│ - Attempt variations to break       │
│ - Confirm or dispute PASS/FAIL      │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 7. CERTIFICATION                     │
│                                     │
│ Builder C reviews evidence and      │
│ issues certification:               │
│ - Review T and V reports            │
│ - Calculate certification score     │
│ - Determine certification level     │
│ - Update RUNTIME_CERTIFICATION.md   │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 8. RELEASE CHECKLIST                 │
│                                     │
│ Run RUNTIME_FIRST_CHECKLIST.md:     │
│ ├─ All YES → Ship                   │
│ └─ Any NO  → Fix → Recertify       │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ 9. CLOSE LEDGER                      │
│                                     │
│ Update the ledger entry:            │
│ - Mark as RESOLVED or CLOSED        │
│ - Reference certification snapshot  │
│ - Document any lessons learned      │
└─────────────────────────────────────┘
```

## Key Principles

### Runtime Failure Always Goes to the Ledger First

Before any code is written, the failure must be recorded in the ledger. This ensures:
- Traceability from failure to fix.
- No fix is attempted without understanding the root cause.
- Historical data accumulates for trend analysis.

### Olympic Events Before Close

No issue is closed until the corresponding Olympic event passes against the running application. Compilation and unit test success are insufficient.

### Certification Before Ship

No release ships without current runtime certification at the appropriate level. The release checklist (`RUNTIME_FIRST_CHECKLIST.md`) must pass.

## Role Handoff

```
Step                  Performed By         Delivers To
──────────────────────────────────────────────────────
1. Ledger             Engineer             Builder C
2. Roadmap Gate       Builder C            Engineer
3. Architecture       Engineer             Engineer
4. Implementation     Engineer             Builder T
5. Olympics           Builder T            Builder V
6. Validation         Builder V            Builder C
7. Certification      Builder C            Repository
8. Release Checklist  All Builders         Repository
9. Close Ledger       Builder C            Ledger
```

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
