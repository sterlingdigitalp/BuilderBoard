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

---

## Amendment Process

These laws are permanent. They may be amended only by:

1. A documented proposal explaining why the amendment is necessary.
2. Unanimous agreement among Builder T, Builder V, and Builder C.
3. An updated copy of this document with the amendment recorded in the ledger.

Amendments that weaken the laws are strongly discouraged.
