# Jules Context

*This document provides BuilderBoard project context for AI engineering agents (Jules).*

---

## BuilderBoard Mission

BuilderBoard exists to allow a software developer to work with four independent AI software engineering assistants simultaneously from a single desktop application.

Each Builder (the AI assistant within a pane) must be capable of performing everyday engineering work: understanding a project, reading files, searching code, modifying files, executing tools, running builds and tests, explaining code, fixing bugs, and implementing requested changes.

Each Builder operates independently — its own repository, conversation, model, tools, and execution state. Changing one Builder must not affect another.

## The Core Promise

> BuilderBoard allows one user to accomplish everything they could accomplish with a single AI coding assistant across four completely independent Builder panes at the same time.

This is the single standard against which all work is measured. The canonical definition is in `CORE_PROMISE.md` (`docs/runtime/CORE_PROMISE.md`).

Seven permanent Engineering Laws in `ENGINEERING_LAWS.md` (`docs/runtime/ENGINEERING_LAWS.md`) govern all development decisions. The most important laws for engineering work:

- **Law 2 — Core Promise Before Expansion**: No feature may be added if it would delay or weaken the Core Promise.
- **Law 4 — No Issue Closed Until Olympic Event Passes**: A fix is not complete until the corresponding Olympic event passes.
- **Law 7 — Regressions Stop Feature Development**: When a regression is detected, all feature development stops until it is resolved.

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

The engineering capability within each Builder is not yet sufficiently reliable. The current deficiencies are documented in `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`. The most critical blockers:

1. Builders cannot yet reliably complete general engineering requests (planner/tool budget exhaustion).
2. Repository discovery is unreliable.
3. Tool execution is not yet sufficiently reliable.
4. Planner efficiency is insufficient.
5. Runtime latency is too high.

## Current Engineering Priorities

The highest priority work is resolving the deficiencies listed in `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`. The active tracking ledger is in `RUNTIME_ENGINEERING_LEDGER.md`.

Current ledger entries (BB-0001, BB-0002) identify two critical issues:

- **BB-0001**: Repository-scale discovery missions exhaust the planner before producing a result.
- **BB-0002**: Repository tool validation failures cause planner exhaustion.

## Runtime First Philosophy

BuilderBoard is judged by its runtime behavior. A feature exists only if a user can successfully use it. Passing tests, clean architecture, or completed implementation are not substitutes for successful runtime behavior.

The Runtime First philosophy is documented in detail in `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md`.

## Runtime Olympics

Runtime behavior is evaluated through the Phase 0 Runtime Olympics — 14 formal runtime events in three tiers:

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
3. **Olympics Before Close**: No issue is closed until the corresponding Olympic event passes.
4. **Certification Before Ship**: No release ships without current runtime certification.
5. **Core Promise First**: No feature may weaken or delay the Core Promise.

## Repository Structure Navigation

| Path | What You Will Find |
|------|-------------------|
| `README.md` | Project overview, quick start, contribution guide |
| `BuilderBoard 1.0-Core Definition.md` | Full Version 1 definition |
| `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md` | Known gaps against Version 1 |
| `RUNTIME_ENGINEERING_LEDGER.md` | Active engineering issues |
| `LOCAL_DEVELOPMENT_RUNTIME.md` | How to build, launch, and test locally |
| `CORE_PROMISE.md` (`docs/runtime/`) | The single mission |
| `ENGINEERING_LAWS.md` (`docs/runtime/`) | 7 permanent engineering rules |
| `docs/runtime/RUNTIME_ENGINEERING_GUIDE.md` | Engineering philosophy handbook |
| `docs/runtime/PHASE0_OLYMPICS.md` | Olympic event definitions |
| `docs/runtime/RUNTIME_CERTIFICATION.md` | Current certification status |
| `docs/runtime/RUNTIME_WORKFLOW.md` | Runtime lifecycle workflow |
| `docs/runtime/BUILDER_T.md` | Test Engineer role |
| `docs/runtime/BUILDER_V.md` | Validation Engineer role |
| `docs/runtime/BUILDER_C.md` | Certifier role |
| `docs/ARCHITECTURE.md` | Architecture documentation |
| `src/` | React/TypeScript frontend |
| `src-tauri/src/` | Rust backend |

## Direct Instruction

BuilderBoard is in Version 1 stabilization. Do not implement unrelated features, roadmap items, or future functionality. Prioritize resolving the runtime deficiencies that prevent the Core Promise.

Every line of code and every document change should answer: *Does this bring BuilderBoard closer to four reliably operating Builder panes?*
