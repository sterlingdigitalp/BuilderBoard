# Builder Runtime Workflow

## The Complete Runtime Lifecycle

This document describes the canonical BuilderBoard engineering and certification lifecycle.

---

## Canonical Engineering Lifecycle

```
                    ┌─────────────────────────────────────────────────────┐
                    │         RUNTIME OLYMPICS (DISCOVERY)                │
                    │  Builder T explores runtime behavior, finds         │
                    │  failures, challenges assumptions.                  │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         RUNTIME ENGINEERING LEDGER                  │
                    │  Failure recorded: root cause, Olympic linkage,     │
                    │  affected files, Verification Source.               │
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
                    │         BUILDER C — ARCHITECTURE REVIEW             │
                    │  Validates root cause. Approves approach.           │
                    │  Identifies Olympic events for certification.       │
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
                    │         BUILDER T — RUNTIME OLYMPICS (REGRESSION)   │
                    │  Execute Olympic events linked to this fix.         │
                    │  Measure latency, correctness, convergence.         │
                    │  Record PASS/FAIL.                                  │
                    │  Status: RESOLVED (Pending Runtime Certification)   │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         BUILDER V — RUNTIME VALIDATION              │
                    │  Independently repeat each Olympic event.           │
                    │  Confirm or dispute Builder T's results.            │
                    │  Approve or reject closure.                         │
                    │  Status: VALIDATED (if confirmed)                   │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         RUNTIME LEDGER UPDATE                       │
                    │  Status transition recorded.                        │
                    │  Builder V's recommendation documented.             │
                    └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                    ┌─────────────────────────────────────────────────────┐
                    │         CERTIFICATION (if tier complete)             │
                    │  Builder C reviews all passed events.               │
                    │  Issues certification at Bronze/Silver/Gold.        │
                    │  Status: CLOSED (for each resolved entry)           │
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

### Step 0: Discovery Olympics

**Who**: Builder T

**What**: Explore runtime behavior to discover new failures.

**Process**:

1. Launch BuilderBoard.
2. Execute real engineering workflows.
3. Observe runtime behavior — latency, correctness, convergence, errors.
4. Record any failures or anomalies.
5. If a new failure is found, create a ledger entry.

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

### Step 3: Builder C — Architecture Review

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
| `ENGINEERING_LAWS.md` | Twelve permanent engineering principles |
| `PHASE0_OLYMPICS.md` | Runtime Olympics event definitions |
| `RUNTIME_ENGINEERING_GUIDE.md` | Complete engineering philosophy handbook and role definitions |
| `RUNTIME_CERTIFICATION.md` | Current certification status |
| `RUNTIME_FIRST_CHECKLIST.md` | Release checklist |
| `RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification |
| `AUTOMATION_PLAN.md` | Future automation architecture |
| `templates/` | Reusable templates |
