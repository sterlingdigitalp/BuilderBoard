# Runtime Dashboard Specification

*Specification for the ultimate Runtime Certification dashboard.*

---

## Purpose

The Runtime Dashboard provides a single visual source of truth for BuilderBoard's runtime health. It is not a development tool — it is a certification tool. Anyone (Builder T, V, C, or observer) should be able to look at the dashboard and immediately answer: *Does the runtime work?*

---

## Display Requirements

### 1. Current Certification Status

```
┌──────────────────────────────────────────┐
│  CURRENT CERTIFICATION                   │
│                                          │
│  ⚫ Not Certified                        │
│  🔵 Bronze — 70% minimum                │
│  🟡 Silver — 95% minimum                │
│  🟢 Gold — 115% minimum                 │
│                                          │
│  Current score: XX%                      │
│  Last certified: YYYY-MM-DD             │
│  Runtime version: vX.Y.Z                │
└──────────────────────────────────────────┘
```

- Show the highest achieved certification level as the primary badge.
- Show the numeric score below.
- Show whether this level is current or stale (if runtime version changed since certification).

### 2. Per-Event Grid

```
┌─────────────┬──────────────────────┬──────┬──────────┬────────┐
│ Event ID    │ Name                 │ Pass │ Latency  │ Weight │
├─────────────┼──────────────────────┼──────┼──────────┼────────┤
│ OPS-BRZ-001 │ Application Launch   │  ✓   │ 4.2s     │  5%    │
│ OPS-BRZ-002 │ Basic Chat          │  ✓   │ 12.1s    │ 10%    │
│ OPS-BRZ-003 │ Tool Discovery      │  ✓   │ N/A      │  5%    │
│ ...         │ ...                  │ ...  │ ...      │ ...    │
└─────────────┴──────────────────────┴──────┴──────────┴────────┘
```

- One row per Olympic event.
- Color-coded PASS/FAIL.
- Show most recent result per event.
- Show latency compared to target.
- Grouped by tier (Bronze, Silver, Gold).

### 3. Open Ledger Items

```
┌──────────────────────────────────────┬──────────┬──────────────┐
│ Item                                 │ Type     │ Status       │
├──────────────────────────────────────┼──────────┼──────────────┤
│ Bronze events need execution        │ Testing  │ Pending      │
│ Builder T onboarding                │ Training │ In Progress  │
│ ...                                  │ ...      │ ...          │
└──────────────────────────────────────┴──────────┴──────────────┘
```

- Show all open ledger items.
- Color-coded status (Pending, In Progress, Blocked, Complete).

### 4. Known Runtime Blockers

```
┌──────────────────────────────────────────┬────────────┬───────────────┐
│ Blocker                                  │ Status     │ Impact        │
├──────────────────────────────────────────┼────────────┼───────────────┤
│ Phase 8.9F hardcoded builder routing     │ Open       │ Blocks main   │
│ ...                                      │ ...        │ ...           │
└──────────────────────────────────────────┴────────────┴───────────────┘
```

- Show all blockers with severity indicators.
- Show which certification tier each blocker affects.

### 5. Historical Trend

- Line chart of certification score over time (by date).
- Annotations for major version bumps.
- Show regression dips clearly.

### 6. Latency Trends

- Per-event latency over last 5+ runs.
- Show moving average vs. target threshold.
- Flag any event where latency trending toward threshold violation.

### 7. Recent Regressions

- Show events that passed in previous certification but fail now.
- Sorted by most recent first.
- Link to regression report for each.

### 8. Runtime Grade

```
RUNTIME GRADE: B (SILVER)

  Bronze:  100% ✓
  Silver:   80% ✓ (partial)
  Gold:     25% ✗
```

- Letter grade (A-F) based on highest consistently-met tier.
- Grade definitions:
  - **A**: Gold certification current
  - **B**: Silver certification current
  - **C**: Bronze certification current
  - **D**: Below Bronze — partially working
  - **F**: Certification failed or not attempted

---

## Implementation Notes

### Source of Truth

The dashboard reads from committed files:

- `docs/runtime/RUNTIME_CERTIFICATION.md` — current status
- `docs/runtime/ledger/` — historical execution data
- `docs/runtime/certification/` — historical certification snapshots

No database. No runtime state. The dashboard is a static or build-time generated view of committed data.

### Update Frequency

- After each certification cycle (event-driven).
- After each ledger entry is added.
- Not real-time — represents the last completed certification.

### Technology

- Should be a static page (HTML + CSS + JS) or a Tauri pane.
- No server required.
- Should be included in the application itself as an internal diagnostic view, accessible via a `--dashboard` flag or a hidden route.

---

## Future Considerations

- **Alerting**: Email or Slack notification when certification drops.
- **Drill-down**: Clicking an event row shows execution details.
- **Comparison**: Side-by-side view of two certification runs.
- **Export**: PDF export of certification report.
