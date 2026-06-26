# Builder Runtime Workflow

## The Complete Runtime Lifecycle

This document describes the canonical BuilderBoard testing and certification pipeline.

---

## Lifecycle Diagram

```
                    ┌─────────────────────────────────────────────────────┐
                    │              USER MISSION / FEATURE REQUEST          │
                    │  "I want BuilderBoard to do X for a real user."     │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              ROADMAP GATE                          │
                    │  Is runtime certified at the level this feature     │
                    │  requires? (see RUNTIME_ENGINEERING_GUIDE.md)      │
                    ├─────────────────────────────────────────────────────┤
                    │  ✓ Certified → Proceed                             │
                    │  ✗ Not certified → Pause. Fix runtime first.      │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              IMPLEMENTATION PHASE                   │
                    │  Engineers build / modify / fix runtime code.       │
                    │  No testing occurs during this phase.               │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              BUILDER T — EXECUTION                  │
                    │                                                    │
                    │  1. Read Olympic event definition                  │
                    │  2. Launch BuilderBoard                            │
                    │  3. Execute event against running application       │
                    │  4. Record metrics                                 │
                    │  5. Determine PASS/FAIL                            │
                    │  6. Document observations                          │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              RUNTIME LEDGER                         │
                    │                                                    │
                    │  Builder T records:                                │
                    │  - Event ID & name                                 │
                    │  - Date & runtime version                          │
                    │  - PASS/FAIL                                       │
                    │  - All metrics                                     │
                    │  - Reproduction steps (if FAIL)                    │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              BUILDER V — VALIDATION                 │
                    │                                                    │
                    │  1. Read Builder T's report                        │
                    │  2. Repeat each event                              │
                    │  3. Attempt variations to break PASS results        │
                    │  4. Confirm or dispute each PASS/FAIL              │
                    │  5. Document findings                              │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              RESOLUTION (if needed)                 │
                    │                                                    │
                    │  If Builder T and Builder V disagree:              │
                    │  - Builder C reviews evidence                      │
                    │  - Builder C may request retesting                 │
                    │  - Builder C makes final determination             │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              BUILDER C — CERTIFICATION              │
                    │                                                    │
                    │  1. Review Builder T report                        │
                    │  2. Review Builder V report                        │
                    │  3. Review ledger                                  │
                    │  4. Determine certification level                  │
                    │  5. Issue certification                            │
                    │  6. Update RUNTIME_CERTIFICATION.md                │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              RELEASE CHECKLIST                     │
                    │  RUNTIME_FIRST_CHECKLIST.md — 6 questions           │
                    ├─────────────────────────────────────────────────────┤
                    │  ✓ All Yes → Ship                                  │
                    │  ✗ Any No  → Fix → Recertify                      │
                    └──────────────────────┬──────────────────────────────┘
                                           │
                                           ▼
                    ┌─────────────────────────────────────────────────────┐
                    │              CERTIFICATION PUBLISHED                │
                    │                                                    │
                    │  - RUNTIME_CERTIFICATION.md updated                │
                    │  - Certification snapshot filed                    │
                    │  - Runtime score published to dashboard            │
                    │  - Development continues at certified level        │
                    └─────────────────────────────────────────────────────┘
```

---

## Detailed Steps

### Step 0: Roadmap Gate

**Who**: Builder C (or equivalent authority)

**What**: Verify the runtime is certified at the level required by the proposed feature.

**Process**:

1. Developer presents a feature proposal with the required certification level.
2. Builder C checks the current certification level in RUNTIME_CERTIFICATION.md.
3. If the current level meets or exceeds the required level, the gate passes.
4. If the current level is below the required level, the feature is blocked.
5. Blocked features must wait for recertification or be reduced in scope.

**Reference**: See Roadmap Gate section in RUNTIME_ENGINEERING_GUIDE.md.

---

### Step 1: Implementation Phase

**Who**: Engineers

**What**: Build or modify runtime code.

**Constraint**: No testing during implementation. Testing begins only when Builder T launches the application.

---

### Step 2: Builder T Execution

**Who**: Builder T

**What**: Execute each Olympic event against the running application.

**Process**:

1. Read the Olympic event definition from `docs/runtime/PHASE0_OLYMPICS.md`.
2. Launch the BuilderBoard application.
3. For each event:
   a. Perform the user action specified in the Mission.
   b. Observe the runtime behavior.
   c. Measure latency and other metrics.
   d. Compare against Pass Criteria.
   e. Record PASS or FAIL.
4. Record all results in the ledger.
5. Produce a Builder T report.

---

### Step 3: Ledger Entry

**Who**: Builder T

**What**: Record the results in the Runtime Ledger.

**Process**:

1. Create a new ledger entry file in `docs/runtime/ledger/` using the Ledger Entry template.
2. Include all required fields.
3. Sign and date the entry.

---

### Step 4: Builder V Validation

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
4. Produce a Builder V report.

---

### Step 5: Resolution

**Who**: Builder C (if needed)

**What**: Resolve disagreements between Builder T and Builder V.

**Process**:

1. Read both reports.
2. Evaluate the evidence.
3. Request retesting if necessary.
4. Make a final determination.
5. Document the resolution.

---

### Step 6: Builder C Certification

**Who**: Builder C

**What**: Issue formal certification.

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

### Step 7: Release Checklist

**Who**: Builder C, Builder T, Builder V (all sign)

**What**: Verify that the runtime passes the Release Checklist.

**Process**:

1. Open `docs/runtime/RUNTIME_FIRST_CHECKLIST.md`.
2. For each of the 6 questions:
   a. Gather the required evidence.
   b. Answer Yes or No.
   c. Record the evidence location.
3. If all answers are Yes, sign the checklist and proceed to Step 8.
4. If any answer is No, the release is blocked. Return to Step 1 (Implementation) to fix the issue, then recertify.

**Reference**: `docs/runtime/RUNTIME_FIRST_CHECKLIST.md`

---

### Step 8: Certification Published

**What**: The certification is live.

**Consequences**:

- The runtime is certified at the stated level.
- The certification snapshot is filed in `docs/runtime/certification/`.
- The dashboard (if built per `RUNTIME_DASHBOARD_SPEC.md`) is updated.
- All subsequent development must maintain this level.
- Any regression below this level blocks new features.
- The next certification cycle begins when significant changes are made to the runtime.

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

## Escalation Path

```
Builder T discovers blocking issue
    ↓
Builder T records in ledger
    ↓
Builder V confirms issue
    ↓
Issue escalated to engineering team
    ↓
Issue fixed
    ↓
Builder T retests affected events
    ↓
Builder V validates
    ↓
Builder C recertifies
```

---

## Related Documents

| Document | Purpose |
|----------|---------|
| `CORE_PROMISE.md` | The single reason BuilderBoard exists |
| `ENGINEERING_LAWS.md` | Seven permanent engineering principles |
| `PHASE0_OLYMPICS.md` | Runtime Olympics event definitions |
| `RUNTIME_ENGINEERING_GUIDE.md` | Complete engineering philosophy handbook |
| `RUNTIME_CERTIFICATION.md` | Current certification status |
| `RUNTIME_FIRST_CHECKLIST.md` | Release checklist |
| `RUNTIME_DASHBOARD_SPEC.md` | Dashboard specification |
| `AUTOMATION_PLAN.md` | Future automation architecture |
| `BUILDER_T.md` | Runtime Test Engineer role |
| `BUILDER_V.md` | Validation Engineer role |
| `BUILDER_C.md` | Runtime Certifier role |
| `templates/` | Reusable templates |
