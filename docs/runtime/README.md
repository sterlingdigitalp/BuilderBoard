# BuilderBoard Runtime Certification Framework

This directory contains the permanent Runtime Certification framework for BuilderBoard.

## Structure

```
runtime/
├── README.md                    # This file
├── CORE_PROMISE.md              # The single reason BuilderBoard exists
├── ENGINEERING_LAWS.md          # Seven permanent engineering principles
├── RUNTIME_CERTIFICATION.md     # Current certification status (regenerated over time)
├── RUNTIME_ENGINEERING_GUIDE.md # Complete engineering philosophy handbook
├── PHASE0_OLYMPICS.md           # Definitive Runtime Certification document
├── BUILDER_T.md                 # Runtime Test Engineer definition
├── BUILDER_V.md                 # Validation Engineer definition
├── BUILDER_C.md                 # Runtime Certifier definition
├── RUNTIME_WORKFLOW.md          # Complete runtime lifecycle workflow
├── RUNTIME_FIRST_CHECKLIST.md   # Release checklist (mandatory before shipping)
├── RUNTIME_DASHBOARD_SPEC.md    # Dashboard specification for certification visibility
├── AUTOMATION_PLAN.md           # Future automation architecture
├── phase0/                      # Phase 0 specific artifacts
│   └── builders/                # Builder role definitions for Phase 0
├── certification/               # Issued certifications (snapshots over time)
├── ledger/                      # Runtime ledger entries
├── olympics/                    # Olympic event definitions
└── templates/                   # Reusable templates
```

## Philosophy

BuilderBoard is judged first by runtime behavior — whether a real user can accomplish real engineering work — not by unit tests, implementation quality, architecture, or documentation.

This framework enables continuous certification of runtime behavior.

## Quick Start

1. Read **CORE_PROMISE.md** — understand why BuilderBoard exists.
2. Read **ENGINEERING_LAWS.md** — understand the permanent rules.
3. Read **RUNTIME_ENGINEERING_GUIDE.md** — understand Runtime First philosophy.
4. Read **PHASE0_OLYMPICS.md** — understand what must work.

## How to Use

1. **Roadmap Gate** — Before starting any feature, verify the runtime is certified at the required level (see RUNTIME_ENGINEERING_GUIDE.md).
2. **Builder T** executes Olympic events against the running application.
3. **Builder T** records metrics and determines PASS/FAIL in the ledger.
4. **Builder V** independently validates Builder T's results.
5. **Builder C** reviews both, issues certification, and updates RUNTIME_CERTIFICATION.md.
6. **Release Checklist** — Before shipping, verify all 6 questions in RUNTIME_FIRST_CHECKLIST.md pass.

## Document Hierarchy

```
CORE_PROMISE.md           ─ Why we exist (the mission)
    ↓
ENGINEERING_LAWS.md       ─ The rules that govern us
    ↓
RUNTIME_ENGINEERING_GUIDE.md ─ How to think about runtime
    ↓
PHASE0_OLYMPICS.md        ─ What must work
    ↓
RUNTIME_WORKFLOW.md       ─ How the lifecycle works
    ↓
BUILDER_T / V / C         ─ Who does what
    ↓
RUNTIME_CERTIFICATION.md  ─ Current certification status
    ↓
RUNTIME_FIRST_CHECKLIST.md ─ Release readiness
```
