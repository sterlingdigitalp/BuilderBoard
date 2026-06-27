# Builder Runtime Workflow

## The Complete Runtime Lifecycle

This document describes the canonical BuilderBoard engineering and certification lifecycle.

---

## Canonical Engineering Lifecycle

```
                    ┌─────────────────────────────────────────────────────┐
                    │         RUNTIME OBSERVATION                         │
                    │  Anyone observes runtime behavior — failure,        │
                    │  anomaly, unexpected result.                        │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         ENGINEERING HYPOTHESIS (LEDGER)              │
                    │  Hypothesis recorded: observed behavior, expected   │
                    │  behavior, root cause hypothesis, Olympic linkage.  │
                    │  Status: OPEN                                       │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         ROADMAP GATE                                │
                    │  Is runtime certified at the level this fix         │
                    │  requires?                                         │
                    ├─────────────────────────────────────────────────────┤
                    │  ✓ Certified → Proceed                             │
                    │  ✗ Not certified → Fix runtime first.              │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         JULES INVESTIGATION + AUDITS                │
                    │  Investigate root cause. Review existing audits     │
                    │  for relevant evidence. Produce new audit if none   │
                    │  exists.                                            │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         BUILDER C — ARCHITECTURE REVIEW             │
                    │  Validates root cause hypothesis against            │
                    │  investigation and audit evidence.                  │
                    │  Approves approach. Identifies Olympic events.      │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         JULES — IMPLEMENTATION                      │
                    │  Writes and commits the fix.                        │
                    │  Runs regression tests.                             │
                    │  Creates Pull Request.                              │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         BUILDER C — IMPLEMENTATION REVIEW           │
                    │  Confirms fix matches architecture.                 │
                    │  Unit tests pass. Compiler passes.                  │
                    │  Status: IMPLEMENTED                                │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         BUILDER T — RUNTIME EXPERIMENT              │
                    │  Design and execute experiment to test the          │
                    │  hypothesis. Measure observed runtime behavior.     │
                    │  Determine: does evidence support or contradict     │
                    │  the hypothesis?                                    │
                    │  Status: RESOLVED (Pending Runtime Certification)   │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         BUILDER V — EVIDENCE VALIDATION              │
                    │  Independently validate Builder T's evidence.       │
                    │  Confirm or dispute: does evidence support or       │
                    │  reject the hypothesis?                             │
                    │  Approve or reject closure based on evidence.       │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         RUNTIME LEDGER REFINEMENT                   │
                    │  Hypothesis refined if evidence contradicted         │
                    │  original understanding.                            │
                    │  Status transition recorded.                        │
                    │  If hypothesis invalidated → return to implement.   │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         CERTIFICATION (if tier complete)             │
                    │  Builder C reviews all passed events and            │
                    │  supporting evidence.                               │
                    │  Issues certification at Bronze/Silver/Gold.        │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         RELEASE CHECKLIST                           │
                    │  RUNTIME_FIRST_CHECKLIST.md — 6 questions           │
                    │  All sign (Builder C, Builder T, Builder V).        │
                    ├─────────────────────────────────────────────────────┤
                    │  ✓ All Yes → Ship                                  │
                    │  ✗ Any No  → Fix → Recertify                      │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         CERTIFICATION PUBLISHED                     │
                    │  RUNTIME_CERTIFICATION.md updated.                  │
                    │  Certification snapshot filed.                      │
                    │  Development continues at certified level.          │
                    └─────────────────────────────────────────────────────┘
```

---

## Detailed Steps

### Step 0: Runtime Observation

**Who**: Anyone

**What**: Observe runtime behavior — a failure, anomaly, or unexpected result.

**Process**:

1. Use BuilderBoard in any capacity.
2. Observe runtime behavior — latency, correctness, convergence, errors, unexpected results.
3. Record the observation.
4. If the observation indicates a runtime deficiency, proceed to Step 1 to create an engineering hypothesis in the ledger.

---

### Step 0a: Engineering Hypothesis (Ledger Entry)

**Who**: Builder T (or anyone discovering a failure)

**What**: Record the runtime observation as an engineering hypothesis in the Runtime Engineering Ledger.

**Process**:

1. Create a new entry using the Ledger Entry template.
2. Frame the entry as a hypothesis: "The root cause of this observed behavior is X."
3. Include: observed behavior, expected behavior, root cause hypothesis, Olympic event linkage, affected files, Verification Source.
4. Set status to OPEN.

---

### Step 1: Ledger Entry Created

**Who**: Builder T (or anyone discovering a failure)

**What**: Record the failure in the Runtime Engineering Ledger.

**Process**:

1. Create a new entry using the Ledger Entry template.
2. Include: root cause analysis, Olympic event linkage, affected files, Verification Source.
3. Set status to OPEN.

---

### Step 2: Roadmap Gate

**Who**: Builder C

**What**: Verify the runtime is certified at the level required by the proposed fix.

**Process**:

1. Check current certification level in RUNTIME_CERTIFICATION.md.
2. If the current level meets or exceeds the required level, the gate passes.
3. If the current level is below the required level, runtime must be fixed first.

**Reference**: See Roadmap Gate section in RUNTIME_ENGINEERING_GUIDE.md.

---

### Step 3: Audits — Engineering Evidence

**Who**: Builder C / Jules

**What**: Review existing engineering investigations relevant to the ledger entry.

**Process**:

1. Open `AUDITS/README.md` and identify relevant audits by ledger item or topic.
2. Read the relevant audit documents.
3. If existing audits cover the topic, use their findings to inform root cause analysis.
4. If no existing audit covers the topic, Jules conducts an investigation and produces a new audit document.
5. Cross-reference any audit findings in the ledger entry.
6. Pass the accumulated evidence to Builder C for Architecture Review.

**Purpose**: Prevents redundant investigations and ensures engineering decisions are based on accumulated evidence.

---

### Step 4: Builder C — Architecture Review

**Who**: Builder C

**What**: Validate the investigation and approve the implementation approach.

**Process**:

1. Read the ledger entry and root cause analysis.
2. Evaluate the proposed approach.
3. Confirm the approach is consistent with the Core Promise.
4. Identify the Olympic events required for certification.
5. Approve or reject the approach.

If rejected, return to Step 1 for further investigation.

---

### Step 4: Jules — Implementation

**Who**: Jules (Implementation Engineer)

**What**: Write and commit the fix.

**Process**:

1. Read the approved architecture.
2. Implement the fix.
3. Write regression tests.
4. Run `cargo test --lib` (or equivalent).
5. Run `cargo check` and `npm run typecheck`.
6. Create a Pull Request.

**Constraint**: Implementation occurs after architecture review. No implementation before approval.

---

### Step 5: Builder C — Implementation Review

**Who**: Builder C

**What**: Review the implementation against the approved architecture.

**Process**:

1. Read the Pull Request.
2. Confirm the implementation matches the approved architecture.
3. Verify unit tests pass and no regressions exist.
4. Set status to IMPLEMENTED.

---

### Step 6: Builder T — Regression Olympics

**Who**: Builder T

**What**: Execute Olympic events against the running application.

**Process**:

1. Read the Olympic event definitions linked to this ledger entry.
2. Launch BuilderBoard.
3. For each event:
    a. Perform the user action specified in the Mission.
    b. Observe the runtime behavior.
    c. Measure latency and other metrics.
    d. Compare against Pass Criteria.
    e. Record PASS or FAIL.
4. Record all results in the ledger.
5. Set status to RESOLVED (Pending Runtime Certification).
6. Produce a Builder T test report.

---

### Step 7: Builder V — Runtime Validation

**Who**: Builder V

**What**: Independently validate Builder T's results.

**Process**:

1. Read Builder T's report and ledger entries.
2. For each event:
    a. Repeat the exact test procedure.
    b. Confirm the same results.
    c. Attempt at least one variation that could cause failure.
    d. Document CONFIRMED or DISPUTED.
3. If any result is disputed, document the evidence and escalate to Builder C.
4. Produce a Builder V validation report.
5. Approve or reject closure.

If confirmed, set status to VALIDATED.
If disputed, return to Step 6 or Step 4 depending on the nature of the dispute.

---

### Step 8: Ledger Update

**Who**: Builder V (with Builder C review if needed)

**What**: Record the final status transition.

**Process**:

1. Document Builder V's recommendation.
2. If approved, set status to CLOSED.
3. If rejected, document the reason and the required next steps.

---

### Step 9: Certification

**Who**: Builder C

**What**: Issue formal certification when all events at a tier pass.

**Process**:

1. Review Builder T report.
2. Review Builder V report.
3. Review the ledger.
4. Calculate the certification score.
5. Determine the certification level (Bronze/Silver/Gold/None).
6. Issue the certification document.
7. Update RUNTIME_CERTIFICATION.md with the new status.
8. File the certification in `docs/runtime/certification/`.

---

### Step 10: Release Checklist

**Who**: Builder C, Builder T, Builder V (all sign)

**What**: Verify that the runtime passes the Release Checklist.

**Process**:

1. Open `docs/runtime/RUNTIME_FIRST_CHECKLIST.md`.
2. For each of the 6 questions:
    a. Gather the required evidence.
    b. Answer Yes or No.
    c. Record the evidence location.
3. If all answers are Yes, sign the checklist and proceed to Step 11.
4. If any answer is No, the release is blocked. Return to Step 4 to fix the issue, then recertify.

**Reference**: `docs/runtime/RUNTIME_FIRST_CHECKLIST.md`

---

### Step 11: Certification Published

**What**: The certification is live.

**Consequences**:

- The runtime is certified at the stated level.
- The certification snapshot is filed in `docs/runtime/certification/`.
- The dashboard (if built per `RUNTIME_DASHBOARD_SPEC.md`) is updated.
- All subsequent development must maintain this level.
- Any regression below this level blocks new features.
- The next certification cycle begins when significant changes are made to the runtime.

---

## Escalation Path

```
Runtime Olympics (Discovery) reveals blocking issue
    ↓
Ledger entry created (OPEN)
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

## Triggers for Recertification

Recertification is required when:

1. A new runtime version is released.
2. A regression is detected in the field.
3. The certification has expired (no fixed expiration; recertification is event-driven).
4. A major architectural change is made to the runtime.
5. Builder C determines that recertification is necessary.

## Continuous Improvement Loop

The certification framework itself evolves through a continuous improvement loop:

```
Certification issued
    ↓
Experience with current events reveals gaps or ambiguities
    ↓
New events proposed or existing events refined
    ↓
Builder C reviews and approves changes
    ↓
Updated Olympics document published
    ↓
Next certification cycle reflects improved framework
```

### How to Improve

- **Add events**: When a new capability needs runtime verification, add an Olympic event.
- **Refine pass criteria**: If a pass criterion is ambiguous or unmeasurable, clarify it.
- **Adjust weights**: If an event's importance changes relative to others, adjust its weight.
- **Tighten latency targets**: As the runtime improves, targets should become more demanding.

All changes to the Olympics must be reviewed by Builder C and documented in the ledger.

---

## Related Documents

| Document | Purpose |
|----------|---------|
| `CORE_PROMISE.md` | The single reason BuilderBoard exists |
| `ENGINEERING_LAWS.md` | Fifteen permanent engineering principles |
| `PHASE0_OLYMPICS.md` | Runtime Olympics event definitions |
| `RUNTIME_ENGINEERING_GUIDE.md` | Complete engineering philosophy handbook and role definitions |
| `RUNTIME_CERTIFICATION.md` | Current certification status |
| `RUNTIME_FIRST_CHECKLIST.md` | Release checklist |
| `RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification |
| `AUTOMATION_PLAN.md` | Future automation architecture |
| `templates/` | Reusable templates |
