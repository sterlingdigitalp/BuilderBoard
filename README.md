# BuilderBoard

Four independent AI software engineering assistants in one desktop application.

---

## What Is BuilderBoard?

BuilderBoard is a desktop application that lets one software developer work with four independent AI software engineers simultaneously from a single window.

Each Builder operates independently — its own repository, conversation, model, tools, and execution state. Changing one Builder does not affect another.

## The Core Promise

> **BuilderBoard allows one user to accomplish everything they could accomplish with a single AI coding assistant across four completely independent Builder panes at the same time.**

This promise is the single measure of the product. Everything else is subordinate.

A permanent definition of this promise is maintained in [`CORE_PROMISE.md`](docs/runtime/CORE_PROMISE.md) and governed by twelve [`ENGINEERING_LAWS.md`](docs/runtime/ENGINEERING_LAWS.md).

## Version 1

BuilderBoard Version 1 is complete when a user can reliably:

- Launch the application.
- Open four Builder panes.
- Assign four different software projects.
- Select Builder models.
- Give each Builder different engineering work.
- Have each Builder successfully complete that work.
- Continue interacting with each Builder independently.

The full Version 1 definition is in [`BuilderBoard 1.0-Core Definition.md`](./BuilderBoard%201.0-Core%20Definition.md).

### Current Status: Version 1 Not Yet Achieved

The Core Promise is not yet satisfied. The engineering capability within each Builder is not yet sufficiently reliable. Known deficiencies are documented in [`BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`](./BuilderBoard%201.0-Current%20Deficiencies%20Against%20Core%20Definition.md).

## Runtime Certification

BuilderBoard uses a Runtime First engineering philosophy: the running application is the only thing that matters. Code, tests, architecture, and documentation exist to serve the runtime.

Runtime behavior is evaluated through the **Phase 0 Olympics** — a set of 14 formal runtime events organized into Bronze (single pane, single tool), Silver (single pane, multi-tool), and Gold (multi-pane, multi-tool) tiers.

The complete Runtime Certification Framework is in [`docs/runtime/`](docs/runtime/). Start with:

| Document | Purpose |
|----------|---------|
| [`RUNTIME_ENGINEERING_GUIDE.md`](docs/runtime/RUNTIME_ENGINEERING_GUIDE.md) | Engineering philosophy handbook |
| [`PHASE0_OLYMPICS.md`](docs/runtime/PHASE0_OLYMPICS.md) | Olympic event definitions |
| [`RUNTIME_CERTIFICATION.md`](docs/runtime/RUNTIME_CERTIFICATION.md) | Current certification status |
| [`RUNTIME_WORKFLOW.md`](docs/runtime/RUNTIME_WORKFLOW.md) | Runtime lifecycle workflow |

## Engineering Ledger

Runtime failures and engineering issues are tracked in the [`RUNTIME_ENGINEERING_LEDGER.md`](./RUNTIME_ENGINEERING_LEDGER.md). This is the authoritative engineering backlog for BuilderBoard Version 1.

Each ledger entry represents one independently fixable runtime problem with explicit dependencies, Olympic event linkage, verification source, and success criteria. The normalization that produced this structure is documented in [`LEDGER_NORMALIZATION_SUMMARY.md`](./LEDGER_NORMALIZATION_SUMMARY.md). Status changes from Builder C's stabilization sprint are in [`LEDGER_REVISION_2_SUMMARY.md`](./LEDGER_REVISION_2_SUMMARY.md).

**Current state:** 2 CLOSED, 2 RESOLVED (Pending Certification), 2 PARTIALLY RESOLVED, 5 OPEN.

## Quick Start

### Prerequisites

- macOS (Sonoma or later recommended)
- Node.js 18+
- Rust 1.77+
- Xcode Command Line Tools

### Build and Run

```sh
# Install frontend dependencies
npm install

# Create the local code-signing identity (one-time)
npm run runtime:setup

# Build, package, sign, and install the local runtime
npm run runtime:build

# Launch the application
npm run runtime:launch
```

Use the packaged runtime for authenticated work; `npm run dev` is only for unauthenticated UI development. If macOS keeps asking for Keychain access after prior debug-runtime use, reset stale BuilderBoard Keychain entries, then reconnect accounts from the packaged app:

```sh
npm run runtime:keychain:reset -- --dry-run
npm run runtime:keychain:reset -- --yes
npm run runtime:build -- --launch
```

See [`LOCAL_DEVELOPMENT_RUNTIME.md`](./LOCAL_DEVELOPMENT_RUNTIME.md) for detailed instructions, including troubleshooting common macOS Keychain issues.

### Validate

```sh
npm run build        # Build the frontend
npm run typecheck    # TypeScript type checking
cargo check          # Rust compilation check
cargo test --lib     # Run Rust unit tests
```

### Environment Variables

Copy `.env.example` to `.env` and fill in the required values. See [`.env.example`](./.env.example) for documentation.

## How To Contribute

1. Read the [Core Promise](docs/runtime/CORE_PROMISE.md) — understand what BuilderBoard exists to do.
2. Read the [Engineering Laws](docs/runtime/ENGINEERING_LAWS.md) — understand the permanent rules.
3. Read the [Engineering Guide](docs/runtime/RUNTIME_ENGINEERING_GUIDE.md) — understand the Runtime First philosophy.
4. Check the [Runtime Engineering Ledger](RUNTIME_ENGINEERING_LEDGER.md) — understand what currently needs fixing.
5. Check the [Current Deficiencies](BuilderBoard%201.0-Current%20Deficiencies%20Against%20Core%20Definition.md) — understand the gaps against Version 1.
6. Follow the [Runtime Workflow](docs/runtime/RUNTIME_WORKFLOW.md) — understand how testing, validation, and certification work.
7. See [JULES.md](./JULES.md) if you are an AI engineering agent — context and instructions specific to LLM-based contributors.

## Project Structure

```
BuilderBoard/
├── CORE_PROMISE.md                  # The single mission
├── ENGINEERING_LAWS.md              # Permanent engineering rules
├── BuilderBoard 1.0-Core Definition.md
├── BuilderBoard 1.0-Current Deficiencies Against Core Definition.md
├── RUNTIME_ENGINEERING_LEDGER.md    # Active engineering issues
├── LOCAL_DEVELOPMENT_RUNTIME.md     # Runtime build & launch instructions
├── JULES.md                         # Context for AI engineering agents
├── GITHUB_READINESS.md              # GitHub publishing checklist
├── REPOSITORY_AUDIT.md              # Repository audit findings
├── docs/
│   ├── runtime/                     # Runtime Certification Framework
│   ├── ARCHITECTURE.md              # Architecture documentation
│   ├── DECISIONS.md                 # Design decisions
│   └── ...                          # Phase implementation reports
├── src/                             # React/TypeScript frontend
├── src-tauri/src/                   # Rust backend
├── scripts/macos/                   # Build and runtime scripts
└── package.json
```

## License

*Not yet specified. A license file should be added before public publishing.*
