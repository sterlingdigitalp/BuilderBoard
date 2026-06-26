# Builder V Validation Report

## Session Metadata

| Field | Value |
|-------|-------|
| **Validation ID** | `V-{YYYY}-{NNN}` |
| **Date** | {YYYY-MM-DD} |
| **Runtime Version** | {version} |
| **Builder V** | {Name} |
| **Builder T Session** | `T-{YYYY}-{NNN}` |

## Validation Results

| Event ID | Name | Builder T Result | Builder V Result | Agreement |
|----------|------|-----------------|-----------------|-----------|
| OPS-{TIER}-{NNN} | {Name} | PASS / FAIL | PASS / FAIL | Agree / Disagree |
| ... | ... | ... | ... | ... |

## Confirmed Findings

### Confirmed PASS — {Event ID}: {Name}

- **Builder T Metrics**: {metrics}
- **Builder V Metrics**: {metrics}
- **Variations Attempted**: {variations tried}
- **Conclusion**: Confirmed

### Confirmed FAIL — {Event ID}: {Name}

- **Builder T Reproduction**: {steps}
- **Builder V Reproduction**: {steps}
- **Reproducible**: Yes / No / Intermittent
- **Conclusion**: Confirmed

## Disputed Findings

### Disputed — {Event ID}: {Name}

- **Builder T Claim**: {claim}
- **Builder V Evidence**: {contrary evidence}
- **Analysis**: {analysis}
- **Recommendation**: {recommendation for Builder C}

## Additional Findings

{Failures or observations not reported by Builder T.}

### Finding — {description}

- **Event**: {Event ID (or N/A for exploratory)}
- **Observed**: {what happened}
- **Severity**: {severity}
- **Reproducible**: Yes / No

## Summary

| Metric | Value |
|--------|-------|
| Events Validated | {N} |
| Confirmed | {N} |
| Disputed | {N} |
| Additional Findings | {N} |

## Overall Validation

{Overall assessment — whether Builder T's certification percentage is accurate.}

---

**Builder V**: {Name}
**Date**: {YYYY-MM-DD}
