# BuilderBoard Runtime Engineering Guide

*The definitive handbook for Runtime First engineering.*

---

## Mission

BuilderBoard's mission is to allow a single user to accomplish everything possible with one AI software engineering assistant simultaneously across four independent Builder panes.

This guide describes how every contributor — engineer, tester, certifier — operates within the Runtime First philosophy to achieve that mission.

---

## Core Promise

> BuilderBoard exists to allow a single user to accomplish everything possible with one AI software engineering assistant simultaneously across four independent Builder panes. Until this works reliably, BuilderBoard is not considered complete.

Everything in this guide serves that promise.

The Core Promise is permanently defined in `CORE_PROMISE.md`. It is supported by twelve Engineering Laws defined in `ENGINEERING_LAWS.md` which govern all development decisions.

---

## What Runtime First Means

### Primary Measure

Runtime behavior is the primary measure of software quality.

When evaluating whether a change is good, the first question is always: *Does the running application work better for the user?*

Secondary questions (test coverage, code quality, architecture) are subordinate and may be deferred.

### Secondary Measures

These are important but never override runtime behavior:

- **Unit tests**: A test that passes but does not reflect real runtime behavior is misleading.
- **Code quality**: Clean code that does not work is not clean — it is technical debt.
- **Architecture**: An elegant architecture that does not deliver reliable runtime behavior is a design failure.
- **Documentation**: Documentation that describes non-functional behavior is misinformation.

### Implementation Truth vs Runtime Truth

Implementation does not prove success. Runtime does.

A fix that compiles, passes unit tests, and looks correct in code review is **implemented** — not **resolved**. The distinction is fundamental:

| Truth | How Established | What It Proves |
|-------|----------------|----------------|
| **Implementation Truth** | Code audit, compilation, unit tests | The fix is present in the codebase |
| **Runtime Truth** | Olympic event execution against live runtime | The fix works for a real user |

Implementation Truth must never be confused with Runtime Truth. This principle is codified as Engineering Law 8.

### Implementation Fungibility

If the runtime works correctly, implementation details can be changed freely.

If the runtime breaks, the implementation must be fixed regardless of how clean or well-tested it is.

This means:

- Refactoring is safe when certification confirms the runtime still works.
- Refactoring is unsafe when certification has not been run.
- All significant changes must be followed by recertification.

---

## Phase 0

Phase 0 is the Core Runtime Olympics. It defines the minimum functionality BuilderBoard must possess before any additional product functionality is considered complete.

Phase 0 is defined in `docs/runtime/PHASE0_OLYMPICS.md`.

Phase 0 contains three tiers:

- **Bronze**: Single pane, single tool. The application must be launchable and capable of executing individual tool calls.
- **Silver**: Single pane, multi-tool. The application must chain multiple tool calls and terminate correctly.
- **Gold**: Multi-pane, multi-tool. The full Core Promise must be demonstrated.

No feature is considered delivered until the corresponding Olympic events pass.

---

## Runtime Olympics

The Runtime Olympics are the definitive set of tests that determine whether BuilderBoard works.

Each Olympic event is a single, independently executable test of runtime behavior.

Events are organized by tier (Bronze, Silver, Gold) and scored by weight.

Complete event definitions are in `docs/runtime/PHASE0_OLYMPICS.md`.

### Two Purposes

Runtime Olympics serve two distinct purposes:

1. **Discovery Olympics** — explore runtime behavior to find new failures. These are open-ended investigations that may uncover issues not yet in the ledger. Builder T leads discovery.

2. **Regression Olympics** — re-execute specific events to verify that a fix has been correctly applied and no regressions were introduced. These are deterministic re-tests linked to specific ledger entries. Builder T leads regression testing; Builder V independently validates results.

Every ledger entry must specify which Olympic events certify it and whether those events are discovery or regression in nature.

---

## Ledger

The Runtime Ledger is the permanent record of all runtime deficiencies, fixes, and certifications.

### Canonical Status Progression

Every ledger entry follows this lifecycle:

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

| Status | Meaning | Required Evidence |
|--------|---------|-------------------|
| **OPEN** | Issue is acknowledged but no fix has been implemented. Investigation may be in progress. | Ledger entry with root cause analysis, Olympic linkage, and affected files |
| **IMPLEMENTED** | A fix has been written and committed. Code audit confirms the fix matches the intended change. Unit tests pass. | Code audit, compiler passes, unit test results |
| **RESOLVED (Pending Runtime Certification)** | Implementation is complete and reviewed. Runtime Olympics have been designed or identified. Awaiting execution of Olympic events against the live runtime. | Implementation review signoff, Olympic event specification |
| **VALIDATED** | Runtime Olympics have been executed by Builder T. Builder V has independently confirmed the runtime behavior improvement. | Builder T test report, Builder V validation report |
| **CLOSED** | The issue is confirmed resolved at runtime level. Olympic events pass. No further action required. Reopen if regression occurs. | Certification entry, Olympic pass results |

**Implementation alone never closes a ledger entry.** Only runtime evidence can close an entry. This is Engineering Law 9.

### Entry Structure

Each ledger entry records:

- Event ID and name.
- Date and runtime version.
- Builder T who executed the test.
- Builder V who validated the test.
- PASS/FAIL result.
- Metrics collected.
- Verification Source (how the runtime behavior was observed).
- Any observations or anomalies.

The ledger accumulates over time. It provides:

- Traceability for every certification.
- Historical data for trend analysis.
- Evidence for regression detection.

Current ledger entries are stored in `docs/runtime/ledger/`.

---

## Certification

Certification is the process of formally declaring that BuilderBoard meets the Core Runtime Olympics requirements.

### Who Certifies What

Builder C and Builder V have distinct, non-overlapping certification responsibilities:

| Role | Certifies | Does NOT Certify |
|------|-----------|-----------------|
| **Builder C** | Architecture soundness. Implementation correctness. Olympic event design. | Runtime behavior. Live application performance. |
| **Builder V** | Runtime behavior. Olympic event results. Ledger status accuracy. | Implementation correctness. Code quality. Unit test coverage. |

**Certification requires both.** Builder C certifies that the right thing was built. Builder V certifies that it works in the running application. Neither can substitute for the other.

### Certification Tiers

Certification occurs in three tiers corresponding to the Olympic tiers:

- **Bronze Certification**: All Bronze events pass.
- **Silver Certification**: All Bronze + Silver events pass.
- **Gold Certification**: All Bronze + Silver + Gold events pass.

### Current Status

Current certification status is maintained in `docs/runtime/RUNTIME_CERTIFICATION.md`.

Historical certifications are stored in `docs/runtime/certification/`.

### Certification Snapshot

Each certification is a snapshot in time. It certifies that the runtime worked correctly at that specific version under those specific conditions.

---

## Regression

A regression is any Olympic event that passed in a prior certification but fails in the current one.

When a regression is detected:

1. The event is marked FAIL in the current ledger entry.
2. A regression report is created using the template at `docs/runtime/templates/REGRESSION_REPORT_TEMPLATE.md`.
3. The regression is added to the Known Runtime Blockers in RUNTIME_CERTIFICATION.md.
4. The regression must be fixed before the runtime can be recertified at the same level.

Regressions are the highest priority work item. No new features may be added while a regression exists at the current certification level.

---

## Continuous Improvement

The Runtime Certification framework is itself subject to improvement.

### Adding Events

New Olympic events can be added at any time. Each new event must:

1. Follow the event template.
2. Be assigned a unique Event ID.
3. Be assigned a certification weight.
4. Be reviewed by Builder C.
5. Specify whether it is a Discovery event or Regression event.

Adding events raises the certification bar. This is encouraged.

### Modifying Events

Existing events should not be modified unless:

- The pass criteria are ambiguous or unmeasurable.
- The latency targets are unrealistic.
- The event no longer reflects intended user behavior.

Changes to pass criteria that make an event easier to pass must be reviewed by Builder C and documented in the ledger.

### Retiring Events

Events may be retired if:

- The feature they test has been intentionally removed.
- The user workflow they represent is no longer supported.

Retired events are moved to a `retired` section in the Olympics document. They are not deleted.

---

## Roadmap Gate

Every feature or phase of development must pass through the Roadmap Gate before implementation begins.

### The Gate Rule

> **No feature may be implemented unless the runtime is currently certified at the level that feature requires.**

Concretely:
- If the runtime is not Bronze certified, no feature work may proceed until Bronze certification passes.
- If Bronze is certified, only Bronze-level features may be developed.
- To build Silver-level features, the runtime must first achieve Silver certification.
- To build Gold-level features (Core Promise), the runtime must first achieve Silver certification.

### Why This Exists

Without the Roadmap Gate, feature development and runtime stability compete for attention — and features always win in the short term. The result is an unstable runtime that never reaches certification. The Roadmap Gate forces the discipline to finish what was started before starting something new.

### How It Works

1. Developer proposes a feature.
2. Builder C checks current certification level.
3. If certification is below the feature's required level, the feature is blocked until the runtime is recertified.
4. If certification meets the required level, the feature proceeds to implementation.
5. After implementation, the feature must pass its corresponding Olympic events before being considered complete.

### Certification Requirement by Feature Type

| Feature Type | Minimum Certification Required | Rationale |
|-------------|-------------------------------|-----------|
| Bug fix / regression fix | Current level | Must maintain existing certification |
| Single-pane tool enhancement | Bronze | Tool execution must already work |
| Multi-tool chaining | Bronze | Tool execution is prerequisite |
| Multi-pane feature | Silver | Multi-tool must work in single pane first |
| Core Promise feature | Silver | Gold certification is the deliverable |

### Escalation

If a feature is blocked by the Roadmap Gate, the developer may:

1. Request recertification at the required level.
2. Reduce the feature scope to match the current certification level.
3. Defer the feature until the runtime achieves the required level.

No developer may bypass the Roadmap Gate. If a feature is implemented while the runtime is below the required certification level, the implementation will not be certified and may not ship.

---

## Role Definitions

### Builder T — Runtime Test Engineer

Builder T is the Runtime Test Engineer.

Builder T:
- designs and maintains Runtime Olympics
- executes Olympic events against the running application
- discovers new runtime failures through experimentation
- challenges engineering assumptions with runtime evidence
- measures runtime behavior — latency, correctness, convergence
- may invalidate existing ledger hypotheses with new evidence
- produces Builder T test reports

Builder T does **not** implement fixes. Builder T's job is to reveal reality, not to change it.

### Builder V — Runtime Validation Engineer

Builder V is the Runtime Validation Engineer.

Builder V:
- independently validates every fix produced by Builder C and Jules
- determines whether runtime evidence supports ledger closure
- controls the RESOLVED → CLOSED transition
- approves or rejects every ledger status change
- produces Builder V validation reports

Builder V does **not** implement fixes. Builder V is the final runtime gatekeeper. No ledger item may close without Builder V signoff.

### Builder C — Architecture and Implementation Reviewer

Builder C has two distinct review stages:

1. **Architecture Review** — reviews investigations, validates the approach, and approves the implementation plan before any code is written.
2. **Implementation Review** — reviews the completed implementation, confirms it matches the architecture, and verifies unit tests pass.

Builder C:
- validates investigations and root cause analysis
- validates implementations against architecture
- recommends implementation approaches
- recommends runtime testing priorities
- reviews Olympic event design

Builder C does **not** certify runtime. Builder C certifies that the implementation is architecturally sound.

### Jules — Implementation Engineer

Jules is the Implementation Engineer.

Jules follows this lifecycle for each task:

```
Investigate
    ↓
Implement
    ↓
Regression tests
    ↓
Pull Request
    ↓
Builder C review
```

Jules:
- investigates runtime deficiencies under Builder C direction
- implements fixes based on approved architecture
- writes and runs regression tests
- produces pull requests for Builder C review

Jules does **not** certify runtime. Jules does **not** design Olympic events.

---

## Workflow Summary

### Canonical Engineering Lifecycle

```
Runtime Olympics (Discovery)
    ↓
Runtime Engineering Ledger (entry created)
    ↓
Builder C — Architecture Review
    ↓
Jules — Implementation
    ↓
Builder C — Implementation Review
    ↓
Builder T — Runtime Olympics (Regression)
    ↓
Builder V — Runtime Validation
    ↓
Runtime Ledger Update (status transition)
    ↓
Certification (if all events at tier pass)
```

### Release Path

```
Certification achieved at required level
    ↓
Release Checklist (RUNTIME_FIRST_CHECKLIST.md)
    ├── All Yes → Ship
    └── Any No  → Fix → Recertify
```

### Escalation Path

```
Builder T discovers blocking issue
    ↓
Builder T records in ledger
    ↓
Builder V confirms issue
    ↓
Issue escalated to Builder C
    ↓
Builder C — Architecture Review
    ↓
Jules — Implementation
    ↓
Builder C — Implementation Review
    ↓
Builder T — Regression Olympics
    ↓
Builder V — Validation
    ↓
Builder C — Certification (if tier complete)
```

---

## Verification Source

Every claim about runtime behavior must identify how it was verified.

Verification sources, in order of reliability:

| Source | Reliability | When Used |
|--------|-------------|-----------|
| **Runtime Olympics** | Highest | Formal certification events |
| **Builder T** | High | Discovery testing, experimentation |
| **Builder V** | High | Independent validation audits |
| **Builder C Technical Review** | Medium | Architecture and implementation reviews |
| **Jules Investigation** | Medium | AI agent investigation |
| **Runtime Trace** | Medium | Automated trace analysis |
| **User Observation** | Low | Anecdotal reports |

This requirement is codified as Engineering Law 12. Every ledger entry must include a Verification Source field.

---

## Related Documents

| Document | Purpose |
|----------|---------|
| `CORE_PROMISE.md` | The single reason BuilderBoard exists |
| `ENGINEERING_LAWS.md` | Twelve permanent engineering principles |
| `PHASE0_OLYMPICS.md` | Runtime Olympics event definitions |
| `RUNTIME_WORKFLOW.md` | Complete runtime lifecycle workflow |
| `RUNTIME_CERTIFICATION.md` | Current certification status |
| `RUNTIME_FIRST_CHECKLIST.md` | Release checklist (mandatory before shipping) |
| `RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification for certification visibility |
| `AUTOMATION_PLAN.md` | Future automation architecture |
| `templates/` | Reusable templates for events, ledger, reports, certifications |

---

## Final Principle

**The application must work for a real user before it is considered complete. Everything else is secondary.**
