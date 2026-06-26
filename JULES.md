# Jules Context

*This document provides BuilderBoard project context for AI engineering agents (Jules).*

---

## Role: Implementation Engineer

Jules is the Implementation Engineer.

Jules:
- investigates runtime deficiencies under Builder C direction
- implements fixes based on approved architecture
- writes and runs regression tests
- produces pull requests for Builder C review

Jules does **not** certify runtime. Jules does **not** design Olympic events. Those responsibilities belong to Builder C, Builder T, and Builder V.

### Jules Lifecycle

Every task follows this lifecycle:

```
Investigate
    ↓
Implement
    ↓
Regression tests
    ↓
cargo check / cargo test / npm run typecheck
    ↓
Pull Request
    ↓
Builder C — Implementation Review
```

No implementation begins before Builder C's Architecture Review is complete. No fix is considered complete until Builder T executes Regression Olympics and Builder V validates the results.

---

## BuilderBoard Mission

BuilderBoard exists to allow a software developer to work with four independent AI software engineering assistants simultaneously from a single desktop application.

Each Builder (the AI assistant within a pane) must be capable of performing everyday engineering work: understanding a project, reading files, searching code, modifying files, executing tools, running builds and tests, explaining code, fixing bugs, and implementing requested changes.

Each Builder operates independently — its own repository, conversation, model, tools, and execution state. Changing one Builder must not affect another.

## The Core Promise

> BuilderBoard allows one user to accomplish everything they could accomplish with a single AI coding assistant across four completely independent Builder panes at the same time.

This is the single standard against which all work is measured. The canonical definition is in `CORE_PROMISE.md` (`docs/runtime/CORE_PROMISE.md`).

Twelve permanent Engineering Laws in `ENGINEERING_LAWS.md` (`docs/runtime/ENGINEERING_LAWS.md`) govern all development decisions. The most important laws for engineering work:

- **Law 2 — Core Promise Before Expansion**: No feature may be added if it would delay or weaken the Core Promise.
- **Law 4 — No Issue Closed Until Olympic Event Passes**: A fix is not complete until the corresponding Olympic event passes.
- **Law 7 — Regressions Stop Feature Development**: When a regression is detected, all feature development stops until it is resolved.
- **Law 9 — Implementation Does Not Close Ledger Items**: Only runtime evidence can close a ledger entry.
- **Law 10 — Every Implementation Must Receive Architectural Review**: No code before architecture approval.

## Version 1 Definition

BuilderBoard Version 1 is complete when a user can reliably:

1. Launch the application.
2. Open four Builder panes.
3. Assign four different software projects.
4. Select Builder models.
5. Give each Builder different engineering work.
6. Have each Builder successfully complete that work.
7. Continue interacting with each Builder independently.

The full definition is in `BuilderBoard 1.0-Core Definition.md`.

### Version 1 Is NOT Yet Achieved

The engineering capability within each Builder is not yet sufficiently reliable. The current deficiencies are documented in `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`. Version 1 stabilization is in progress — see `LEDGER_REVISION_2_SUMMARY.md` for the most recent state.

## Current Engineering State (as of Revision 2)

**Four fixes verified (2 CLOSED, 2 RESOLVED Pending Certification):**

| Entry | Title | Status |
|-------|-------|--------|
| BB-0004 | Filesystem scope resolver rejects non-existent paths | RESOLVED (Pending Runtime Certification) |
| BB-0005 | Search tool reports failure on no-match result | RESOLVED (Pending Runtime Certification) |
| BB-0011 | Frontend Promise.all cascade | CLOSED |
| BB-0012 | sendMessage stale closure | CLOSED |

**Remaining OPEN blockers (by engineering priority):**

1. **BB-0006** — Planner convergence detection (next to work on)
2. **BB-0009** — Planner budget exhausted by inefficient sequences (depends on BB-0006)
3. **BB-0003** — Hardcoded builder routing (independent track, do in parallel)
4. **BB-0001** — Repository-scale discovery failure (depends on BB-0009)
5. **BB-0010** — Builders cannot complete general engineering requests (depends on everything above)

The full ledger and dependency graph are in `RUNTIME_ENGINEERING_LEDGER.md` and `LEDGER_REVISION_2_SUMMARY.md`.

## Runtime First Philosophy

BuilderBoard is judged by its runtime behavior. A feature exists only if a user can successfully use it. Passing tests, clean architecture, or completed implementation are not substitutes for successful runtime behavior.

The Runtime First philosophy is documented in detail in `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md`.

## Runtime Olympics

Runtime behavior is evaluated through the Phase 0 Runtime Olympics — formal runtime events in three tiers:

| Tier | Events | What It Proves |
|------|--------|----------------|
| Bronze | 9 events | Single pane, single tool works |
| Silver | 4 events | Single pane, multi-tool chaining works |
| Gold | 2 events | Multi-pane, multi-tool (Core Promise) works |

Each event has pass criteria, latency targets, and certification weight. Full definitions are in `docs/runtime/PHASE0_OLYMPICS.md`.

Current certification score: **0%** (no formal certification executed yet).

## Runtime Ledger Philosophy

The Runtime Engineering Ledger (`RUNTIME_ENGINEERING_LEDGER.md`) is the permanent record of all runtime failures and engineering issues. Every runtime failure must be recorded in the ledger before any fix is attempted. If it is not in the ledger, it did not happen.

Ledger entries should include:
- Observed runtime behavior
- Expected runtime behavior
- Evidence (Olympic test results)
- Verification Source
- Success criteria
- Affected files (where known)
- Priority and status

## Repository Expectations

- The repository is the authoritative source of truth for BuilderBoard.
- Do not implement features not required for Version 1.
- Do not add roadmap items, future functionality, or speculative architecture.
- Everything should reinforce the Core Promise.
- Documentation changes are as important as code changes — without documentation, the repository is not complete.

## Development Rules

1. **Runtime First**: Always ask "does the running application work better for the user?"
2. **Ledger Before Fix**: Record the failure in the ledger before implementing any fix.
3. **Architecture Before Implementation**: Builder C must approve the approach before any code is written.
4. **Olympics Before Close**: No issue is closed until the corresponding Olympic event passes against the running application.
5. **Validation Before Close**: Builder V must independently validate every fix before closure.
6. **Certification Before Ship**: No release ships without current runtime certification.
7. **Core Promise First**: No feature may weaken or delay the Core Promise.

## Repository Structure Navigation

| Path | What You Will Find |
|------|-------------------|
| `README.md` | Project overview, quick start, contribution guide |
| `BuilderBoard 1.0-Core Definition.md` | Full Version 1 definition |
| `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md` | Known gaps against Version 1 |
| `RUNTIME_ENGINEERING_LEDGER.md` | Active engineering issues |
| `LOCAL_DEVELOPMENT_RUNTIME.md` | How to build, launch, and test locally |
| `CORE_PROMISE.md` (`docs/runtime/`) | The single mission |
| `ENGINEERING_LAWS.md` (`docs/runtime/`) | 12 permanent engineering rules |
| `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md` | Engineering philosophy handbook with role definitions |
| `docs/runtime/PHASE0_OLYMPICS.md` | Olympic event definitions |
| `docs/runtime/RUNTIME_CERTIFICATION.md` | Current certification status |
| `docs/runtime/RUNTIME_WORKFLOW.md` | Complete engineering lifecycle workflow |
| `docs/runtime/BUILDER_T.md` | Test Engineer role |
| `docs/runtime/BUILDER_V.md` | Validation Engineer role |
| `docs/runtime/BUILDER_C.md` | Architecture and Implementation Reviewer role |
| `BUILDER_WORKFLOW.md` | Engineering workflow summary |
| `docs/ARCHITECTURE.md` | Architecture documentation |
| `src/` | React/TypeScript frontend |
| `src-tauri/src/` | Rust backend |

## Direct Instruction

BuilderBoard is in Version 1 stabilization. Your role is Implementation Engineer. You do not certify runtime. You do not design Olympic events. You implement fixes based on Builder C's approved architecture, write tests, and produce pull requests for review.

Every line of code and every document change should answer: *Does this bring BuilderBoard closer to four reliably operating Builder panes?*
