# Engineering Laws

*Permanent principles that govern all BuilderBoard development.*

---

## Law 1 — Runtime is the Product

The running application is the only thing that matters. Code, tests, architecture, and documentation exist to serve the runtime — not the other way around. If the runtime does not work for a real user, nothing else matters.

## Law 2 — Core Promise Before Expansion

No feature, tool, or capability may be added if it would delay or weaken the Core Promise:

> Four independent Builder panes, each capable of independent engineering work with tool execution.

Until the Core Promise is met at Gold certification, every line of code must answer: *Does this bring us closer to four reliably operating panes?*

## Law 3 — Every Runtime Failure Becomes a Ledger Entry

Every runtime failure — crash, hang, incorrect response, tool error, loop exhaust — must be recorded in the Runtime Ledger before any fix is attempted. If it is not in the ledger, it did not happen.

## Law 4 — No Issue Closed Until Olympic Event Passes

A bug fix or feature implementation is not complete until the corresponding Olympic event (or a new one if one does not exist) passes against the running application. Compilation and unit test success are insufficient.

## Law 5 — Ships Only After Runtime Certification

No release may ship without a current Runtime Certification at the appropriate level. Certification is not optional, not skippable for "emergency fixes," and not replaceable by code review.

## Law 6 — Every Feature Must Preserve the Core Promise

Every new feature must be verified against the Core Promise before it can be merged. A feature that works in isolation but interferes with multi-pane operation is not a feature — it is a regression.

## Law 7 — Regressions Stop Feature Development

When a regression is detected — an Olympic event that previously passed now fails — all feature development stops until the regression is resolved. No exceptions. No "quick feature" before fixing the regression.

## Law 8 — Implementation Truth vs Runtime Truth

Implementation does not prove success. Runtime does.

A fix that compiles, passes unit tests, and looks correct in code review is **implemented** — not **resolved**. Resolution requires runtime evidence: the fix must be observed working in the running application under realistic conditions.

This distinction is fundamental:

| Truth | How It Is Established | What It Proves |
|-------|----------------------|----------------|
| **Implementation Truth** | Code audit, compilation, unit tests | The fix is present in the codebase |
| **Runtime Truth** | Olympic event execution against live runtime | The fix works for a real user |

Implementation Truth must never be confused with Runtime Truth.

## Law 9 — Implementation Does Not Close Ledger Items

No Runtime Engineering Ledger item may be marked CLOSED based on implementation evidence alone.

Every status transition from IMPLEMENTED → RESOLVED → VALIDATED → CLOSED requires runtime evidence. The only exception is when an item is determined to be non-reproducible after investigation — in which case it is marked CLOSED with a documented investigation report.

Canonical status progression:

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

## Law 10 — Every Implementation Must Receive Architectural Review

No implementation may proceed without an architectural review by Builder C. The review must confirm:

- The approach is consistent with the Core Promise.
- The implementation plan is sound.
- The Olympic events required for certification are identified.

Architectural review occurs before implementation begins.

## Law 11 — Every Runtime Fix Must Receive Runtime Validation

No runtime fix may be considered complete until it has been independently validated by Builder V against the running application.

Builder V validation occurs after Builder T executes the Runtime Olympics. Builder V must confirm or dispute every PASS/FAIL result.

## Law 12 — Every Ledger Item Must Identify Verification Source

Each Runtime Engineering Ledger entry must identify how the observed runtime behavior was verified.

Valid verification sources:

- **Runtime Olympics** — formal Olympic event executed against live runtime
- **Builder T** — Runtime Test Engineer experimentation
- **Builder V** — Runtime Validation Engineer independent audit
- **Builder C Technical Review** — architecture or implementation code review
- **Jules Investigation** — AI agent investigation and findings
- **Runtime Trace** — automated trace or log analysis
- **User Observation** — direct observation of user-facing behavior

This requirement exists to prevent unverified claims from entering the ledger. Every claim must be traceable to a specific observation method.

---

## Amendment Process

These laws are permanent. They may be amended only by:

1. A documented proposal explaining why the amendment is necessary.
2. Unanimous agreement among Builder T, Builder V, and Builder C.
3. An updated copy of this document with the amendment recorded in the ledger.

Amendments that weaken the laws are strongly discouraged.
