# Builder T Report

## Session Metadata

| Field | Value |
|-------|-------|
| **Session ID** | `T-{YYYY}-{NNN}` |
| **Date** | {YYYY-MM-DD} |
| **Runtime Version** | {version} |
| **Builder T** | {Name} |
| **Testing Scope** | Bronze / Silver / Gold / Partial |

## Event Results

| Event ID | Name | Result | Notes |
|----------|------|--------|-------|
| OPS-{TIER}-{NNN} | {Name} | PASS / FAIL | {brief note} |
| ... | ... | ... | ... |

## Summary

| Metric | Value |
|--------|-------|
| Total Events | {N} |
| Passed | {N} |
| Failed | {N} |
| Percentage | {N}% |

## Detailed Findings

### PASS — {Event ID}: {Name}

- **Metrics**: {key metrics}
- **Observations**: {what was observed}

### FAIL — {Event ID}: {Name}

- **Reproduction Steps**: {steps}
- **Expected**: {expected behavior}
- **Actual**: {observed behavior}
- **Severity**: {severity}

## Blocking Issues

| Issue | Blocks | Severity |
|-------|--------|----------|
| {issue} | {event IDs} | {severity} |

## Notes

{Any additional context, environmental factors, or observations.}

---

**Builder T**: {Name}
**Date**: {YYYY-MM-DD}
