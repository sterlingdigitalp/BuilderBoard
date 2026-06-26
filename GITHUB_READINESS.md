# GitHub Readiness Checklist

*Verify that BuilderBoard is ready for GitHub publishing.*

---

## Instructions

Run through each item. If any item is unchecked, resolve it before pushing to GitHub.

---

## Build and Run

- [ ] Repository builds from a fresh clone (`npm install && cargo build`)
- [ ] Frontend builds successfully (`npm run build`)
- [ ] TypeScript passes type checking (`npm run typecheck`)
- [ ] Rust passes compilation (`cargo check`)
- [ ] Rust tests pass (`cargo test --lib`)
- [ ] Tauri configuration is valid (`npm run tauri -- --help`)
- [ ] Vite configuration is valid

## Documentation

- [ ] Root README describes what BuilderBoard is
- [ ] Root README describes the Core Promise
- [ ] Root README describes Version 1
- [ ] Root README describes current deficiencies
- [ ] Root README provides build and run instructions
- [ ] Root README provides contribution guidance
- [ ] Root README directs readers to critical documents
- [ ] Core Definition document exists (`BuilderBoard 1.0-Core Definition.md`)
- [ ] Current Deficiencies document exists (`BuilderBoard 1.0-Current Deficiencies Against Core Definition.md`)
- [ ] Runtime Certification Framework exists (`docs/runtime/`)
- [ ] Runtime Engineering Guide exists (`docs/runtime/RUNTIME_ENGINEERING_GUIDE.md`)
- [ ] Phase 0 Olympics document exists (`docs/runtime/PHASE0_OLYMPICS.md`)
- [ ] Runtime Workflow document exists (`docs/runtime/RUNTIME_WORKFLOW.md`)
- [ ] Runtime Certification status exists (`docs/runtime/RUNTIME_CERTIFICATION.md`)
- [ ] Release Checklist exists (`docs/runtime/RUNTIME_FIRST_CHECKLIST.md`)
- [ ] Local Development Runtime document exists (`LOCAL_DEVELOPMENT_RUNTIME.md`)
- [ ] Engineering Ledger exists (`RUNTIME_ENGINEERING_LEDGER.md`)
- [ ] Jules context document exists (`JULES.md`)
- [ ] GitHub Readiness Checklist exists (this file)
- [ ] Repository Audit exists (`REPOSITORY_AUDIT.md`)
- [ ] Documentation index exists (`docs/README.md`)
- [ ] Builder Workflow documented (`BUILDER_WORKFLOW.md`)
- [ ] All environment variables documented (`.env.example`)

## Runtime Scripts

- [ ] `scripts/macos/setup-local-signing.sh` exists and is executable
- [ ] `scripts/macos/build-dev-runtime.sh` exists and is executable
- [ ] `scripts/macos/launch-dev-runtime.sh` exists and is executable
- [ ] `scripts/macos/runtime-certification-loop.sh` exists and is executable
- [ ] All scripts are referenced in `package.json`
- [ ] All scripts are documented in `LOCAL_DEVELOPMENT_RUNTIME.md`

## Environment

- [ ] `.env.example` exists and documents all required variables
- [ ] `.env` is in `.gitignore`
- [ ] No secrets or credentials are committed
- [ ] No hardcoded paths assume the developer's machine

## Git

- [ ] `.gitignore` covers all generated artifacts
- [ ] No large binary files are tracked
- [ ] No `node_modules/` or `target/` directories are tracked
- [ ] No `.DS_Store` or other system files are tracked
- [ ] Git history is clean (no merge commits, no temporary commits)

## Repository Readiness for Jules

- [ ] An AI engineering agent can clone the repository
- [ ] The agent can understand what BuilderBoard is from the repository alone
- [ ] The agent can understand Version 1 from the repository alone
- [ ] The agent can understand current deficiencies from the repository alone
- [ ] The agent can understand how runtime is tested from the repository alone
- [ ] The agent can understand how runtime is certified from the repository alone
- [ ] The agent can identify current engineering priorities from the repository alone
- [ ] The agent can build the application from the repository alone
- [ ] The agent can run the runtime from the repository alone
- [ ] The agent can begin engineering work without asking the project owner questions

## Uncommitted Files

The following critical files are **not yet committed** to the repository. A fresh clone will not contain them. These must be committed before GitHub publishing:

| File | Why It Is Required |
|------|--------------------|
| `BuilderBoard 1.0-Core Definition.md` | Version 1 definition |
| `BuilderBoard 1.0-Current Deficiencies Against Core Definition.md` | Known gaps against Version 1 |
| `LOCAL_DEVELOPMENT_RUNTIME.md` | Build and launch instructions |
| `RUNTIME_ENGINEERING_LEDGER.md` | Active engineering issues |
| `docs/runtime/` (14+ files) | Complete Runtime Certification Framework |
| `scripts/macos/` (4 files) | Build, launch, certification scripts |
| `.env.example` | Environment variable documentation |
| `JULES.md` | AI agent context document |
| `GITHUB_READINESS.md` | This checklist |
| `BUILDER_WORKFLOW.md` | Engineering workflow documentation |
| `REPOSITORY_AUDIT.md` | Repository audit |
| `docs/README.md` | Documentation index |

## Sign-Off

When all items above (including the uncommitted files section) are checked and the files are committed, the repository is ready for GitHub publishing.

| Role | Signature | Date |
|------|-----------|------|
| Builder T | | |
| Builder V | | |
| Builder C | | |
