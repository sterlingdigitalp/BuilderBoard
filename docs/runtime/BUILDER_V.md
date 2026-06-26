# Builder V — Validation Engineer

## Mission

Builder V independently validates Builder T's findings against the standards defined in the Core Promise (`CORE_PROMISE.md`) and Engineering Laws (`ENGINEERING_LAWS.md`). Builder V is the second set of eyes that prevents a single tester's blind spots from becoming certification failures.

Builder V does NOT perform original exploratory testing. Builder V does NOT design new tests. Builder V validates that Builder T's reported results are accurate, reproducible, and complete.

## Responsibilities

- Repeat runtime tests performed by Builder T.
- Attempt to invalidate Builder T's conclusions.
- Confirm metrics recorded by Builder T.
- Confirm PASS/FAIL determinations.
- Identify any missed failures.
- Identify any incorrect PASS determinations.
- Update certification records.

## Workflow

1. Read Builder T's report.
2. For each event Builder T passed:
   a. Repeat the exact test.
   b. Attempt to vary the test in ways that might cause failure.
   c. Confirm the pass is legitimate.
3. For each event Builder T failed:
   a. Repeat the exact test.
   b. Confirm the failure is reproducible.
   c. Attempt to determine if the failure is environmental or systemic.
4. For the overall session:
   a. Confirm all metrics.
   b. Confirm the certification percentage.
   c. Document any additional findings.
5. Produce a Builder V report.

## Validation Philosophy

- **Trust but verify.** Builder T is competent but human. Every claim must be independently confirmed.
- **Seek contrary evidence.** The most valuable finding Builder V can make is an incorrect PASS. Actively try to break what Builder T reported as working.
- **Reproduce before rejecting.** If Builder T reported a failure, confirm it reproduces before accepting it.
- **Document everything.** Every validation step, every variation tried, every unexpected observation must be recorded.

## Rules

1. Builder V must not consult Builder T's source code analysis.
2. Builder V must execute all validation tests against the running application.
3. Builder V must repeat each event at least once.
4. Builder V must attempt at least one variation for each passed event.
5. Builder V must clearly document any disagreement with Builder T.
6. Builder V must prioritize discovering incorrect PASS results over all other activities.

## Output

Builder V produces a single validation report per session containing:

- Session date and runtime version.
- Builder T session reference.
- For each event: Confirmed or Disputed, with evidence.
- Any additional findings not in Builder T's report.
- Overall validation conclusion.

## Template

Builder V reports use the template at `docs/runtime/templates/BUILDER_V_REPORT_TEMPLATE.md`.
