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

The Core Promise is permanently defined in `CORE_PROMISE.md`. It is supported by fifteen Engineering Laws defined in `ENGINEERING_LAWS.md` which govern all development decisions.

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

### Engineering Evidence Library (AUDITS)

The `AUDITS/` directory is BuilderBoard's permanent Engineering Knowledge Base. It contains
engineering investigations, runtime analysis, performance studies, architectural investigations,
and UX evaluations produced by Jules and Builder T.

**These are not product documentation. These are engineering records.**

Every audit in the library represents evidence that informs decisions, validates hypotheses,
and accumulates institutional knowledge. Before beginning significant engineering work, review
the relevant audits in `AUDITS/README.md`. Before launching a new investigation, check if an
audit already covers the topic — if it does, extend or validate that work rather than duplicating it.

### Four Forms of Evidence

BuilderBoard recognizes four independent forms of evidence, each with a distinct purpose and validity:

| Form | How Established | What It Proves | Who Produces |
|------|----------------|----------------|-------------|
| **Investigation** | Root cause analysis, code audit, Jules investigation report | A plausible explanation for the observed failure | Jules, Builder C |
| **Implementation** | Code written, tests pass, compiler succeeds, Builder C review passes | The fix is present in the codebase | Jules |
| **Runtime Experiment** | Builder T designs and executes an experiment against live runtime | Evidence that supports or contradicts an engineering hypothesis | Builder T |
| **Evidence Validation** | Builder V independently validates Builder T's experimental evidence | The evidence is reproducible, correctly interpreted, and supports or rejects the hypothesis | Builder V |

Only Runtime Experiment produces evidence about the hypothesis. Only Evidence Validation confirms that the evidence is sound. Investigation and Implementation are necessary prerequisites — neither is sufficient to close a ledger entry.

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

### Three Olympic Modes

Runtime Olympics serve three distinct purposes:

1. **Discovery Olympics** — explore runtime behavior to find new failures. These are open-ended investigations that may uncover issues not yet in the ledger. Builder T leads discovery. Results may create new ledger entries or refine existing hypotheses.

2. **Regression Olympics** — re-execute specific events to verify that a fix has been correctly applied and no regressions were introduced. These are deterministic re-tests linked to specific ledger entries. Builder T leads regression testing; Builder V independently validates results.

3. **Certification Olympics** — formal execution of the full event suite at a given tier to determine whether the runtime qualifies for Bronze, Silver, or Gold certification. These are comprehensive sweeps executed when all entries at a tier are closed. Builder C issues certification based on results.

| Dimension | Discovery | Regression | Certification |
|-----------|-----------|------------|---------------|
| **Purpose** | Find new failures | Verify existing fixes | Qualify for tier |
| **Lead** | Builder T | Builder T | Builder C |
| **Validation** | Not required | Builder V | Builder C issues |
| **Pass criteria** | Exploratory | Deterministic | Full suite pass |
| **Ledger linkage** | Creates new entry | References existing entry | Tier completion |
| **Frequency** | Continuous | After every implementation | When all entries at tier close |

Every ledger entry must specify which Olympic events certify it and which mode they use.

---

## Ledger

The Runtime Ledger is the permanent record of all runtime deficiencies, fixes, and certifications.

### Canonical Status Progression

Every ledger entry follows this lifecycle:

```
OPEN (hypothesis recorded)
  ↓  (investigation, architecture review)
IMPLEMENTED
  ↓  (implementation review, unit tests pass)
RESOLVED (Pending Runtime Certification)
  ↓  (Runtime Experiment executed — evidence supports or contradicts hypothesis)
VALIDATED
  ↓  (Builder V validates evidence — supports or rejects hypothesis)
  ↓  (if hypothesis contradicted by evidence → return to IMPLEMENTATION)
CLOSED
```

| Status | Meaning | Required Evidence |
|--------|---------|-------------------|
| **OPEN** | An engineering hypothesis has been recorded. The observed behavior and expected behavior are documented. Investigation may be in progress. | Ledger entry with hypothesis, root cause analysis, Olympic linkage, and affected files |
| **IMPLEMENTED** | A fix has been written and committed. Code audit confirms the fix matches the intended change. Unit tests pass. | Code audit, compiler passes, unit test results |
| **RESOLVED (Pending Runtime Certification)** | Implementation is complete and reviewed. Builder T has designed an experiment to test the hypothesis. Awaiting execution of experiment against the live runtime. | Implementation review signoff, experimental design specification |
| **VALIDATED** | Builder T's experiment has been executed. Builder V has validated that the evidence is reproducible and correctly interpreted. The evidence supports or rejects the hypothesis. | Builder T experimental report, Builder V validation report |
| **CLOSED** | The evidence supports the hypothesis and the issue is confirmed resolved at runtime level. No further action required. Reopen if regression occurs. | Certification entry, experimental pass results |

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

## Ledger Hypotheses Are Current Understanding

Every Runtime Engineering Ledger entry is a hypothesis about reality — not a permanent truth. Ledger entries may evolve when runtime evidence disproves earlier assumptions.

### Example: BB-0006

BB-0006 was originally entered as "Planner lacks convergence detection for repository-scale enumeration." The hypothesis was that the planner loop needed a code change to detect when it had gathered sufficient information.

The hypothesis evolved through three revisions in a single day as runtime evidence accumulated:

1. **v1**: "Planner lacks convergence detection" — architectural assumption.
2. **v2**: "Prompt lacks completion directive" — OPS-CON-001 suggested loop logic works, prompt may be the issue.
3. **v3**: "Planner lacks error recovery and tool call adaptation" — Builder T Hypothesis Validation executed Execution 2 against the life runtime trace, revealing 10 identical `filesystem.write` calls with zero adaptation. The planner *converges* correctly but *recovers* from errors incorrectly.

This is a valid ledger hypothesis correction sequence. Each revision was driven by runtime evidence, not speculation. The hypothesis will continue to evolve as more runtime evidence is gathered.

### Principle

Runtime evidence overrides architectural assumptions (Law 13). When a ledger hypothesis is contradicted by runtime evidence, the hypothesis must be updated — not the evidence ignored. Builders are encouraged to invalidate incorrect hypotheses through experimentation (Law 15).

---

## Olympic Gap Analysis

The Core Definition requires Builders to perform specific engineering tasks. Each task requires at least one Olympic event. The following table maps the Core Definition requirements to existing and needed Olympic coverage:

| Core Definition Requirement | Existing Events | Gap |
|---------------------------|----------------|-----|
| Understanding a project | OPS-BRZ-006 (structure read) | Partial — no coverage for understanding intent |
| Reading files | OPS-BRZ-004 (single read) | Covered |
| Searching code | OPS-BRZ-007 (grep/glob) | Covered |
| **Modifying files** | — | **Gap: no write/create/modify event** |
| Executing tools | OPS-BRZ-004/005/007 | Covered |
| **Running builds** | — | **Gap: no build-invocation event** |
| **Running tests** | — | **Gap: no test-invocation event** |
| Explaining code | OPS-SLV-002 (multi-tool) | Partial — covered implicitly through multi-tool chains |
| **Fixing bugs** | — | **Gap: no targeted bug-fix event** |
| **Implementing changes** | — | **Gap: no end-to-end implementation event** |
| Multi-turn conversations | OPS-BRZ-002 (basic chat) | Partial — only single-turn tested |
| **Different repositories** | — | **Gap: no multi-project event** |
| **Different model selection** | — | **Gap: no model-switching event** |
| Engineering completion | OPS-SLV-003 (loop term) | Partial — termination only, not "did it complete the work" |
| **Runtime recovery** | — | **Gap: no crash/hang recovery event** |
| UI responsiveness | — | Gap: subjective, hard to automate |

**These gaps are scheduled for the next Olympic expansion cycle.** New events will be designed and added to `PHASE0_OLYMPICS.md` as they are reviewed by Builder C.

---

## Authenticated Runtime Workflow

### Development vs Certification

BuilderBoard has two distinct runtime modes:

| Mode | Command | Purpose | Authentication |
|------|---------|---------|---------------|
| **Development** | `npm run dev` | UI development, component testing, Hot Module Replacement | Local keychain (may be unstable) |
| **Certification** | `npm run runtime:build -- --launch` | Authenticated runtime testing for Olympic events | Packaged app with stable Keychain identity |

Builder T and Builder V must use the **packaged runtime** (`/Applications/BuilderBoard Dev.app`) for all Olympic event execution that involves authenticated provider workflows.

### Keychain Behavior

The first launch of the packaged runtime may prompt for Keychain access once. Subsequent launches should not. Repeated Keychain prompts on every launch indicate a runtime regression that must be recorded in the ledger.

### Why the Packaged Runtime is Required

The development server (`npm run dev` / `cargo tauri dev`) uses an unstable macOS Keychain identity that cannot reliably store and retrieve OAuth tokens or API keys. Evidence collected from unauthenticated sessions is invalid for certification purposes.

**Builder T must use:** `/Applications/BuilderBoard Dev.app` (the locally signed packaged runtime)

**Builder T must not use:** `npm run dev`, `cargo tauri dev`, or `target/debug/builderboard`

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

### Builder T — Runtime Experimentalist

Builder T is the Runtime Experimentalist.

Builder T:
- designs experiments to test engineering hypotheses
- executes experiments against the running application
- determines whether runtime evidence supports or contradicts the hypothesis
- discovers new runtime failures through discovery experiments
- challenges engineering assumptions with runtime evidence
- measures runtime behavior — latency, correctness, convergence
- proposes Runtime Ledger corrections when evidence contradicts current understanding
- may invalidate existing ledger hypotheses with new evidence
- produces Builder T experimental reports

Builder T does **not** implement fixes. Builder T's job is to produce evidence about engineering hypotheses through controlled experimentation.

### Builder V — Runtime Evidence Validator

Builder V is the Runtime Evidence Validator.

Builder V:
- validates evidence, not implementations
- determines whether Builder T's experimental evidence is sufficient, reproducible, and correctly interpreted
- assesses whether evidence supports or rejects the engineering hypothesis
- independently repeats Builder T's experiments to validate reproducibility
- controls the RESOLVED → CLOSED transition based on evidence quality
- approves or rejects every ledger status change
- produces Builder V validation reports

Builder V does **not** implement fixes. Builder V is the final runtime gatekeeper. No ledger item may close without Builder V signoff on evidence quality.

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
Runtime Observation (anyone)
    ↓
Engineering Hypothesis (Runtime Ledger entry)
    ↓
Roadmap Gate
    ↓
Jules Investigation + AUDITS — Engineering Evidence
    ↓
Builder C — Architecture Review
    ↓
Jules — Implementation
    ↓
Builder C — Implementation Review
    ↓
Builder T — Runtime Experiment (test hypothesis)
    ↓
Builder V — Evidence Validation (validate evidence)
    ↓
Runtime Ledger Refinement (hypothesis may be refined based on evidence)
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
Builder T discovers blocking issue or contradictory evidence
    ↓
Builder T records hypothesis in ledger
    ↓
Builder V validates evidence of issue
    ↓
Jules Investigation + AUDITS — Engineering Evidence review
    ↓
Issue escalated to Builder C
    ↓
Builder C — Architecture Review (with audit and experimental context)
    ↓
Jules — Implementation
    ↓
Builder C — Implementation Review
    ↓
Builder T — Runtime Experiment (test refined hypothesis)
    ↓
Builder V — Evidence Validation
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
| `ENGINEERING_LAWS.md` | Fifteen permanent engineering principles |
| `AUDITS/README.md` | Engineering Evidence Library index |
| `PHASE0_OLYMPICS.md` | Runtime Olympics event definitions |
| `RUNTIME_WORKFLOW.md` | Complete runtime lifecycle workflow |
| `RUNTIME_CERTIFICATION.md` | Current certification status |
| `RUNTIME_FIRST_CHECKLIST.md` | Release checklist (mandatory before shipping) |
| `RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification for certification visibility |
| `AUTOMATION_PLAN.md` | Future automation architecture |
| `templates/` | Reusable templates for events, ledger, reports, certifications |

---

## Evidence-Based Engineering

BuilderBoard distinguishes five types of engineering knowledge:

| Type | Location | Purpose |
|------|----------|---------|
| **Architecture Documents** | `docs/ARCHITECTURE.md`, `docs/runtime/CORE_PROMISE.md` | Describe intended design |
| **Runtime Engineering Ledger** | `RUNTIME_ENGINEERING_LEDGER.md` | Tracks engineering hypotheses and their status |
| **Engineering Evidence (AUDITS)** | `AUDITS/` | Contains investigations and evidence |
| **Runtime Experiments** | Builder T experimental reports | Produces evidence about whether hypotheses are supported or contradicted |
| **Evidence Validation** | Builder V validation reports | Confirms that experimental evidence is reproducible and correctly interpreted |

Engineering decisions should increasingly be based on evidence rather than assumptions.
When a ledger hypothesis is contradicted by experimental evidence, the hypothesis must be updated —
not the evidence ignored.

This principle is reinforced by:
- **Law 13** — Runtime Evidence Overrides Assumptions
- **Law 14** — Ledger Represents Current Understanding, Not Permanent Truth
- **Law 15** — Builders Encouraged to Invalidate Incorrect Hypotheses

---

## Final Principle

**The application must work for a real user before it is considered complete. Everything else is secondary.**
