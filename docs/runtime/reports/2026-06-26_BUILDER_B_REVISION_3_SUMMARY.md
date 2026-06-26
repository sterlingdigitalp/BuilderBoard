# BuilderBoard Runtime Framework — Revision 3

**Date:** 2026-06-26
**Builder:** B — Framework Engineer

---

## Summary

Revision 3 updates the entire Runtime Framework to reflect the engineering organization that has emerged through real practice across Builder T, Builder V, Builder C, and Jules. The framework has grown from 7 to 12 Engineering Laws, gained a formal role system, and established a canonical engineering lifecycle.

---

## What Changed

### Modified Documents (12)

| Document | Changes |
|----------|---------|
| `docs/runtime/ENGINEERING_LAWS.md` | Added 5 new laws (8-12): Implementation Truth vs Runtime Truth, Implementation Does Not Close Ledger Items, Every Implementation Must Receive Architectural Review, Every Runtime Fix Must Receive Runtime Validation, Every Ledger Item Must Identify Verification Source |
| `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md` | Added Implementation Truth vs Runtime Truth section. Added Discovery vs Regression Olympics. Replaced old role summary with full role definitions (Builder T, Builder V, Builder C, Jules). Updated ledger section with canonical status progression (OPEN→IMPLEMENTED→RESOLVED→VALIDATED→CLOSED). Updated workflow to new canonical lifecycle. Updated certification section with Builder C vs Builder V separation. Added Verification Source section. Updated law count to 12. |
| `docs/runtime/RUNTIME_WORKFLOW.md` | Replaced entire workflow with the new canonical lifecycle (Discovery Olympics → Ledger → Architecture Review → Implementation → Implementation Review → Regression Olympics → Validation → Ledger Update → Certification → Release Checklist). All 11 steps detailed. |
| `docs/runtime/RUNTIME_CERTIFICATION.md` | Added "Who Certifies What" table clarifying Builder C (architecture + implementation) vs Builder V (runtime behavior) separation. Updated certification process to include Builder V validation step. Removed obsolete line items. |
| `docs/runtime/PHASE0_OLYMPICS.md` | Added Discovery vs Regression Olympics section with comparison table. Updated "seven Engineering Laws" → "twelve Engineering Laws". |
| `docs/runtime/BUILDER_T.md` | Expanded role: added Discovery Olympics mode, added "challenge engineering assumptions" and "may invalidate existing ledger hypotheses" to responsibilities. Confirmed Builder T does not implement. |
| `docs/runtime/BUILDER_V.md` | Expanded role: added RESOLVED→CLOSED gatekeeper authority, ledger authority section (APPROVE/REJECT/PARTIAL), escalated from "validation engineer" to "Runtime Validation Engineer and final runtime gatekeeper". |
| `docs/runtime/BUILDER_C.md` | Complete rewrite: now has two distinct review stages (Architecture Review + Implementation Review). Clarified Builder C does NOT certify runtime — Builder V does. Added review workflows, rules, and outputs for both stages. |
| `JULES.md` | Added role header: Implementation Engineer. Added Jules lifecycle (Investigate→Implement→Tests→PR→Review). Added "Jules does not certify runtime." Added 3 new development rules (Architecture Before Implementation, Validation Before Close). Updated law references from 7→12. |
| `BUILDER_WORKFLOW.md` | Replaced old workflow with the new canonical workflow including Builder C Architecture Review and Implementation Review stages, Jules implementation stage. Added Implementation Truth vs Runtime Truth principle. Updated role handoff table. Updated state transitions. |
| `RUNTIME_ENGINEERING_LEDGER.md` | Added Canonical Status Progression diagram. Added full Status Definitions table with required evidence per state. Added Verification Source section with all 7 valid sources. Updated Status field options to include IMPLEMENTED and VALIDATED. |
| `docs/README.md` | Updated ENGINEERING_LAWS.md description to "Twelve permanent engineering principles". Updated BUILDER_C.md description. Added JULES.md reference. |
| `README.md` (root) | Updated "seven" → "twelve" for Engineering Laws reference. |
| `docs/runtime/README.md` | Updated "Seven" → "Twelve" for Engineering Laws reference. |

### Unchanged Documents

| Document | Reason |
|----------|--------|
| `docs/runtime/CORE_PROMISE.md` | Core Promise is stable and needs no revision. |
| `docs/runtime/RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification is forward-looking and independent of process changes. |
| `docs/runtime/RUNTIME_FIRST_CHECKLIST.md` | Release checklist is process-independent. |
| `docs/runtime/AUTOMATION_PLAN.md` | Automation is future work and independent of current process. |
| `docs/runtime/templates/` | Templates are format-level and independent of role structure. |

---

## New Engineering Principles

### Implementation Truth vs Runtime Truth (Law 8)

Implementation does not prove success. Runtime does. Every change must be verified against the running application before it is considered resolved. Code audit, compilation, and unit tests establish Implementation Truth — only Olympic event execution establishes Runtime Truth.

### Implementation Does Not Close Ledger Items (Law 9)

No ledger entry may be marked CLOSED based on implementation evidence alone. Every status transition from IMPLEMENTED→RESOLVED→VALIDATED→CLOSED requires runtime evidence.

### Every Implementation Must Receive Architectural Review (Law 10)

No code before architecture. Builder C must review and approve the approach before any implementation begins.

### Every Runtime Fix Must Receive Runtime Validation (Law 11)

Builder V must independently validate every fix. No fix is complete without independent runtime confirmation.

### Every Ledger Item Must Identify Verification Source (Law 12)

Every claim about runtime behavior must be traceable to a specific observation method: Runtime Olympics, Builder T, Builder V, Builder C Technical Review, Jules Investigation, Runtime Trace, or User Observation.

---

## Updated Role Definitions

| Role | Title | Responsibilities | Does Not |
|------|-------|-----------------|----------|
| **Builder T** | Runtime Test Engineer | Design Olympics, discover failures, execute regression tests, measure runtime | Implement fixes |
| **Builder V** | Runtime Validation Engineer | Independently validate fixes, control RESOLVED→CLOSED transition, approve/reject closure | Implement fixes, design Olympics |
| **Builder C** | Architecture & Implementation Reviewer | Architecture Review, Implementation Review, issue certification | Certify runtime, execute Olympics |
| **Jules** | Implementation Engineer | Investigate, implement, write tests, create PRs | Certify runtime, design Olympics |

---

## Canonical Workflow

```
Runtime Olympics (Discovery)
    ↓
Runtime Engineering Ledger (OPEN)
    ↓
Roadmap Gate
    ↓
Builder C — Architecture Review
    ↓
Jules — Implementation
    ↓
Builder C — Implementation Review (→ IMPLEMENTED)
    ↓
Builder T — Runtime Olympics (Regression) (→ RESOLVED)
    ↓
Builder V — Runtime Validation (→ VALIDATED)
    ↓
Runtime Ledger Update (→ CLOSED)
    ↓
Certification (if tier complete)
    ↓
Release Checklist → Ship
```

---

## Ledger State Lifecycle

```
OPEN
  ↓ (Architecture Review by Builder C)
IMPLEMENTED
  ↓ (Implementation Review by Builder C)
RESOLVED (Pending Runtime Certification)
  ↓ (Regression Olympics by Builder T)
VALIDATED
  ↓ (Builder V confirms runtime evidence)
CLOSED
```

---

## New Engineer Onboarding Path

A new engineer should read in order:

1. `README.md` — project overview and Core Promise
2. `docs/runtime/CORE_PROMISE.md` — the single mission
3. `docs/runtime/ENGINEERING_LAWS.md` — 12 permanent rules
4. `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md` — complete handbook with role definitions, workflow, and ledger lifecycle
5. `BUILDER_WORKFLOW.md` — concrete workflow diagram with role handoffs
6. `JULES.md` — Implementation Engineer context (if implementing)
7. `docs/runtime/PHASE0_OLYMPICS.md` — how runtime is evaluated
8. `docs/runtime/RUNTIME_CERTIFICATION.md` — current certification state

All documents now consistently describe the same four-role organization, the same canonical workflow, and the same governing principles.
