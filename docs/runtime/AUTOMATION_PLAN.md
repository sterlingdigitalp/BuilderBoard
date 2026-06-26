# Future Automation Plan

*Architecture only — no implementation.*

---

## Purpose

This document describes how BuilderBoard will eventually automate the Runtime Certification process. Automation removes human variability, accelerates certification cycles, and enables continuous verification.

The automation architecture is designed to be incrementally adoptable. Each component can be built independently.

---

## Vision

A fully automated runtime certification pipeline:

1. Code change is committed.
2. CI/CD pipeline builds the application.
3. Pipeline launches the runtime in a test environment.
4. Pipeline executes all Olympic events sequentially.
5. Pipeline captures metrics automatically.
6. Pipeline generates ledger entries.
7. Pipeline updates certification status.
8. Pipeline produces a certification dashboard.

Human oversight (Builder T, Builder V, Builder C) is reserved for:

- Defining new Olympic events.
- Reviewing automation results for subtle failures.
- Making final certification decisions.
- Investigating regressions.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        AUTOMATION ORCHESTRATOR                       │
│                                                                     │
│  Reads Olympic event definitions → Executes events → Records →      │
│  Generates reports → Notifies stakeholders                          │
└─────────────────────────────────────────────────────────────────────┘
        │                    │                      │
        ▼                    ▼                      ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────────────┐
│ LAUNCHER      │   │ EVENT RUNNER  │   │ METRICS COLLECTOR     │
│               │   │               │   │                       │
│ Builds and    │   │ Executes one  │   │ Captures:             │
│ launches the  │   │ Olympic event │   │ - Latency metrics     │
│ application   │   │ against the   │   │ - Tool call counts    │
│ in a clean    │   │ running app   │   │ - Loop rounds         │
│ environment   │   │               │   │ - Error counts        │
└───────────────┘   └───────────────┘   └───────────────────────┘
                                                    │
                                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        LEDGER GENERATOR                              │
│                                                                     │
│  Converts raw metrics into structured ledger entries                │
│  in the canonical format                                            │
└─────────────────────────────────────────────────────────────────────┘
                                                    │
                                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        CERTIFICATION UPDATER                         │
│                                                                     │
│  Recalculates certification score based on ledger entries           │
│  Updates RUNTIME_CERTIFICATION.md                                   │
│  Flags regressions                                                  │
└─────────────────────────────────────────────────────────────────────┘
                                                    │
                                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        DASHBOARD                                     │
│                                                                     │
│  Displays:                                                          │
│  - Current certification status                                     │
│  - Individual event PASS/FAIL                                       │
│  - Historical trend data                                            │
│  - Regression alerts                                                │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. Launcher

**Responsibility**: Build the application and launch it in a controlled environment.

**Input**: Git commit hash or branch name.

**Output**: Running application instance accessible for testing.

**Challenges**:
- Headless launch for CI environments.
- Managing multiple concurrent instances for multi-pane testing.
- Clean state between test runs.

**Design notes**:
- Should support Docker containers for environment isolation.
- Should support macOS (development) and Linux (CI) targets.
- Should capture launch metrics (cold_start_ms, warm_start_ms).

---

### 2. Event Runner

**Responsibility**: Execute a single Olympic event against the running application.

**Input**: Event definition (from Olympics document).

**Output**: Raw metrics and PASS/FAIL determination.

**How it works**:
- Reads the event's Mission to determine what user action to simulate.
- Connects to the running application (via Tauri commands or direct IPC).
- Performs the user action (sends a message, opens a pane, etc.).
- Observes the runtime behavior.
- Captures all required metrics.
- Compares against pass criteria.
- Returns PASS or FAIL.

**Challenges**:
- Simulating natural user input (typing messages, clicking buttons).
- Observing non-deterministic LLM responses.
- Measuring latency accurately.
- Determining whether a response is "correct" (requires semantic evaluation).

**Design notes**:
- Should use the Tauri command API directly rather than UI automation for reliability.
- For Bronze events (single tool), response correctness can be checked by scanning for key terms.
- For Silver/Gold events, response correctness may require LLM-as-judge evaluation.
- Each event runner should be an independent script that can be run in isolation.

---

### 3. Metrics Collector

**Responsibility**: Capture all runtime metrics during event execution.

**Input**: Event execution stream.

**Output**: Structured metrics data.

**Metrics captured**:
- `ttft_ms`: Time to first token.
- `total_response_ms`: Full response time.
- `tool_rounds`: Number of tool call rounds.
- `tool_execution_ms`: Per-tool execution time.
- `tools_chained`: Number of tools in a chain.
- `tool_success`: Whether each tool succeeded.
- `all_succeeded`: Whether all tools in a chain succeeded.
- `duplicates_detected`: Whether identical tool calls were repeated.
- `loop_terminated`: Whether the loop terminated normally.
- `fallthrough_triggered`: Whether the fallthrough message was used.
- `exit_code`: Shell command exit codes.

**Design notes**:
- Should hook into the `trace_runtime_phase` logging system already present in the codebase.
- Should capture both application-level metrics and system-level metrics (CPU, memory).
- Should tag all metrics with event ID, runtime version, and timestamp.

---

### 4. Ledger Generator

**Responsibility**: Convert raw metrics into structured ledger entries.

**Input**: Metrics data from Metrics Collector.

**Output**: Ledger entry file in the canonical format.

**Design notes**:
- Should produce markdown files matching the Ledger Entry template.
- Should append to the ledger archive.
- Should support both automated and manual entries.

---

### 5. Certification Updater

**Responsibility**: Recalculate certification status based on ledger entries.

**Input**: Ledger entries.

**Output**: Updated RUNTIME_CERTIFICATION.md.

**How it works**:
- Reads all ledger entries for the current runtime version.
- Groups by Olympic tier.
- Calculates percentage of passed events.
- Determines certification level.
- Updates the certification document.
- Flags any regressions from previous version.

---

### 6. Dashboard

**Responsibility**: Display current certification status and history. (Full specification in `RUNTIME_DASHBOARD_SPEC.md`.)

**Input**: Current RUNTIME_CERTIFICATION.md and ledger entries.

**Output**: Visual dashboard.

**Design notes**:
- Should be a simple static page generated from the markdown files.
- Should show:
  - Current certification level with date.
  - Per-event PASS/FAIL grid.
  - Historical trend chart.
  - Regression alerts.
- Should not require a database — operates on committed files.

---

## Implementation Priority

| Component | Priority | Rationale |
|-----------|----------|-----------|
| Metrics Collector | P0 | Cannot automate without metrics |
| Event Runner | P0 | Core automation component |
| Launcher | P1 | Can manually launch initially |
| Ledger Generator | P1 | Can manually write entries initially |
| Certification Updater | P2 | Can manually update document |
| Dashboard | P3 | Nice-to-have visualization |

---

## Integration with CI/CD

The automation pipeline integrates with CI/CD as follows:

1. **On pull request**: Build + launch + run Bronze events only (fast feedback).
2. **On merge to main**: Build + launch + run all events (full certification).
3. **On release tag**: Full certification + generate dashboard + publish.

---

## Non-Goals

The following are explicitly out of scope for automation:

- **Semantic evaluation of LLM responses**: Determining whether a response is "helpful" or "correct" in a general sense. Automation checks pass criteria, not subjective quality.
- **UI testing**: The automation connects via the Tauri command API, not through the frontend UI.
- **Load testing**: Concurrent multi-pane testing is covered by Gold events but exhaustive load testing is separate.
- **Security testing**: Runtime certification does not replace security review.
