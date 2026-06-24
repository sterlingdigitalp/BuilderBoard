# BuilderBoard Skill Specification v1.1

**Status:** Draft — supersedes SKILL_SPEC_v1.md  
**Date:** 2026-06-24  
**Author:** BuilderBoard Architecture  
**Applies to:** Phase 9B–9D (implementation)  
**License:** Apache 2.0

---

## Preamble

This specification defines the BuilderBoard Skill architecture. It is the authoritative reference for all Phase 9 implementation work. Every concept, field, lifecycle phase, and security boundary is described here.

The architecture is built around a single conceptual spine:

```
Input → Context → Capability → Execution → Artifacts → Review → Memory
```

Every feature — Voice, Attachments, Skills, Subagents, Marketplace — is an expression of this same model. The architecture does not fragment into disconnected subsystems; it extends the spine.

---

## Section 1 — Definitions

Every term has exactly one meaning. No overlap. No ambiguity.

### Skill

A **Skill** is a reusable, declarable bundle of expertise stored as a filesystem directory with a `SKILL.md` entry point and an optional `skill.json` manifest. A Skill:
- Contains **instructions** (markdown guidance for the model)
- Declares **context requirements** (intents, tools, filesystem scope, memory, project)
- Declares **artifact contracts** (what it produces, what it consumes)
- May bundle **scripts** (deterministic operations executed in a sandbox)
- May bundle **resources** (reference materials, templates, schemas)
- Is **discovered automatically** at startup
- Is **loaded on demand** (metadata → instructions → resources)
- Is **versioned** via semver

**Not:** A Skill is not an executable. It does not have its own agent loop. It does not own state. It does not persist data across sessions except through the artifact and memory system.

**Analogy:** A Skill is a specialist's handbook with a manifest on the cover that says "I need these tools, I produce these reports."

### Agent

An **Agent** is an independent decision-making entity with its own system prompt, tool access, model assignment, agentic loop, and isolated context window. Agents are spawned by the runtime (not by Skills) and follow the lifecycle: spawn → execute → return → destroy.

**Agent vs Skill:** An agent *does work*. A skill *tells the agent how to work*. The same skill can be loaded by different agents. The same agent can load multiple skills.

### Subagent

An **Agent spawned by another Agent**. Subagents run in an isolated context window with independent tool access and return structured results to the parent. Subagent depth is configurable (default: 3).

### Workflow

A **Workflow** is a Skill whose instructions primarily orchestrate other Skills. Workflows use wikilinks (`[[skill-name]]`) to declare dependencies and sequenced steps. Every Workflow is a Skill; not every Skill is a Workflow.

### Tool

A **Tool** is a callable function with typed inputs and typed outputs, registered in a global `ToolRegistry`. Tools execute deterministically and are the model's only way to affect the outside world. Tools are not Skills; Skills describe *when and how to use* tools.

### Intent

An **Intent** is a detectable user goal extracted from natural language input. Intents are detected by the `IntentRouter` and map to Skills via `context.intents`. Intents are the primary routing mechanism from user input to Skill activation.

### Project

A **Project** is a workspace with metadata (`kind: folder`, `approvedRoot`, `name`, `code`). Projects define the filesystem boundary (`ApprovedScope`) and are the scope for Skill execution.

### Pane

A **Pane** is a UI container for one conversation thread. Panes own a `project_id`, `provider_id`, `model_id`, and a list of Messages. Panes are the execution context for all chat operations.

### Artifact

An **Artifact** is a structured, reviewable output produced by a Skill. Artifacts are the bridge between Skills: a Skill produces an artifact; another Skill consumes it. Artifacts follow a lifecycle: **draft → review → accepted → durable** (or **draft → review → rejected**).

### Memory

**Memory** is structured, durable context that survives across sessions. Memory is scoped (pane, project, builder, user, or global), has persistence semantics (session or persistent), and has visibility (private or shared). Memory is not a conversation transcript; it is curated, structured knowledge.

### Event

An **Event** is a timestamped observation emitted by the runtime during execution. Events feed execution traces, frontend UI updates, future analytics, and audit logs.

---

## Section 2 — Skill Philosophy

### What Skills Should Solve

1. **Encapsulate expertise.** Domain knowledge that the model should reference — security audit steps, code review criteria, deployment checklists.

2. **Encapsulate workflows.** Procedural sequences — production readiness review = security → architecture → dependencies.

3. **Encapsulate context requirements.** What the Skill needs (intents, tools, filesystem, memory, project) declared explicitly so the runtime wires it automatically.

4. **Produce artifacts.** Skills produce structured, reviewable outputs that other Skills consume. Artifacts are the composition mechanism, not prompt chaining.

5. **Reduce repetition.** Install once, use everywhere.

### What Skills Should NOT Solve

1. **Encapsulate prompts.** Skills are not prompt templates. No `{user_variables}`. Skills are read as context, not executed as templates.

2. **Encapsulate tools.** Skills describe *how to use* tools. Tool implementations live in the Rust `ToolRegistry`.

3. **Encapsulate agent behavior.** Skills do not define agent personas, system prompts, or model parameters. That is the domain of Builder Bundles (Section 8).

4. **Encapsulate state.** Skills are stateless. All durable state belongs to the Memory system.

5. **Encapsulate security boundaries.** Skills declare *desired* access. The runtime enforces *actual* access.

6. **Replace the model's judgment.** Skills provide guidance, not instructions. The model decides whether and how to follow the Skill's advice.

### The Conceptual Spine

BuilderBoard is centered around this flow:

```
User Input
    ↓
Context Assembly (project, pane, memory)
    ↓
Capability Resolution (intents → skills → tools)
    ↓
Execution (model + tools + enrichment)
    ↓
Artifact Production (structured output)
    ↓
Review (human or automated)
    ↓
Memory (durable persistence)
```

Each phase is clearly separated. Skills enter at Capability Resolution. Artifacts exit at Artifact Production. Memory persists across sessions. Voice changes the Input phase. Subagents change the Execution phase. Attachments change the Context Assembly phase. The spine remains the same.

---

## Section 3 — Skill File Format

### File Location

```
.builderboard/skills/<skill-name>/
├── SKILL.md          # Human-readable instructions (MANDATORY)
├── skill.json        # Machine-readable manifest (MANDATORY for marketplace)
├── scripts/          # Deterministic sandboxed scripts (OPTIONAL)
└── resources/        # Reference materials (OPTIONAL)
```

### SKILL.md

```yaml
---
name: skill-name                    # 1-64 chars, lowercase + hyphens only
description: What this skill does   # 10-200 chars, shown in tool description
version: 1.0.0                      # semver, required for marketplace

author: ""                          # name or org (optional)
license: MIT                        # SPDX identifier (optional)
display_name: "Security Audit"      # human-readable (defaults to name)
categories: [security, code-review] # marketplace browsing (optional)
tags: [audit, vulnerability]        # search/discovery (optional)
icon: ""                            # emoji or icon reference (optional)
compatibility: [builderboard]       # compatible platforms (optional)
min_app_version: 0.8.0             # minimum app version (optional)

context:
  intents: [security_review]        # which intents trigger this skill
  tools: [list_directory, read_file, search_files, find_files, bash]
  filesystem: required              # none | optional | required
  project: required                 # none | optional | required
  memory:                           # see Section 4
    scope: project
    persistence: session
    visibility: private

execution:
  preferred_model_class: reasoning  # fast | reasoning | creative | cheap
  reasoning: high                   # none | low | medium | high
  latency: standard                 # fast | standard | background
  context_budget: 8000              # max tokens for skill body

produces:                           # artifact contracts (see Section 7)
  - security_audit
  - review_item

consumes:                           # artifact dependencies (see Section 7)
  - architecture_review
  - dependency_audit

metadata:
  key: value                        # extensible
---

# Security Audit

## Overview
...

## Instructions
...

## Wikilinks
- [[architecture-review]]
- [[dependency-review]]
```

### skill.json — Machine-Readable Manifest

```json
{
  "schema_version": 1,
  "name": "security-audit",
  "version": "1.0.0",
  "display_name": "Security Audit",
  "description": "Review codebase for security vulnerabilities, credential leaks, and authorization issues",
  "author": {
    "name": "BuilderBoard",
    "url": "https://builderboard.app"
  },
  "license": "MIT",
  "permissions": {
    "tools": ["list_directory", "read_file", "search_files", "find_files"],
    "filesystem": "required",
    "project": "required",
    "memory": {
      "scope": "project",
      "persistence": "session",
      "visibility": "private"
    }
  },
  "compatibility": {
    "platforms": ["builderboard"],
    "min_app_version": "0.8.0",
    "models": ["openai/*", "anthropic/*"]
  },
  "trust_tier": "verified",
  "dependencies": {
    "requires": ["architecture-review@>=1.0.0"],
    "optional": ["dependency-review@>=1.0.0"]
  },
  "outputs": {
    "produces": ["security_audit", "review_item"],
    "consumes": ["architecture_review", "dependency_audit"]
  },
  "evals": {
    "test_projects": ["sample-node-app", "sample-rust-app"],
    "expected_behaviors": ["detect_hardcoded_secrets", "identify_auth_bypass"]
  },
  "checksum_sha256": "abc123..."
}
```

**Why both SKILL.md and skill.json?** SKILL.md is authored by humans. Its frontmatter captures what a person needs to know about the skill. `skill.json` is the source of truth for the machine — it is strict, typed, and validated. For local-only skills, SKILL.md frontmatter is sufficient. For marketplace distribution, `skill.json` is mandatory.

### Key Design Decisions

1. **Version is mandatory.** Enables marketplace migration from day one.
2. **Execution profile replaces `max_tokens`.** Skills declare their performance requirements (model class, reasoning depth, latency budget). The routing engine decides which model to use, not the skill.
3. **Produces/consumes enable artifact-based composition.** Instead of prompt chaining, Skills compose through structured artifact contracts.
4. **Wikilinks for graph composition.** `[[skill-name]]` in the body declares navigable edges.
5. **No inheritance.** Composition replaces inheritance. Use `dependencies.requires` or `dependencies.optional`.

---

## Section 4 — Context Declaration & Memory Model

### The `context` Block

Every Skill declares what it needs. The runtime wires it automatically.

```yaml
context:
  intents: [security_review]
  tools: [list_directory, read_file, search_files, find_files]
  filesystem: required        # none | optional | required
  project: required           # none | optional | required
  memory:                     # explicit memory scope
    scope: project            # pane | project | builder | user | global
    persistence: session      # session | persistent
    visibility: private       # private | shared
```

### Memory Scope Explained

| Scope | Lifetime | Visibility | Use Case |
|---|---|---|---|
| **pane** | Pane's lifetime (closed pane = lost) | Private | Conversation-specific context (current review's findings, intermediate state) |
| **project** | Project lifetime (spans panes within the project) | Shared | Project knowledge (architecture decisions, known issues, team conventions) |
| **builder** | Builder configuration lifetime | Private to the builder instance | Builder-specific state (which skills are loaded, what step of a workflow we're on) |
| **user** | User lifetime (spans all projects) | Private or shared (user chooses) | User preferences, skill configuration, personal patterns |
| **global** | Application lifetime | Shared | System-level knowledge (skill ratings, usage stats, anonymous aggregate data) |

**Why pane memory exists:** A security audit running in pane 1 should not share its intermediate findings with a code review running in pane 2. Pane memory isolates conversation-specific state. When the pane is closed, pane memory is discarded.

**Why project memory exists:** A project-level decision ("we use Effect for error handling") spans all panes within that project. Any skill loaded in any pane of the same project can reference project memory without re-discovery.

**When builder memory is appropriate:** Builder A's workflow step ("step 2 of 4 completed") is builder-scoped. Another builder instance (Builder B) should not see this state. Builder memory tracks the workflow progression within a single builder session.

**How global memory differs:** Global memory is for system-aggregated data: "Skill X has been used 500 times with a 94% satisfaction rate." It is not written by individual skills during execution. It is maintained by the runtime for marketplace and analytics.

### Memory Persistence

| Persistence | Behavior |
|---|---|
| `session` | Survives within the current session only. Lost when session ends. |
| `persistent` | Written to SQLite. Survives app restarts. Survives pane close (if scope is project or above). |

### Memory Visibility

| Visibility | Behavior |
|---|---|
| `private` | Only the skill instance that wrote it can read it. |
| `shared` | All skills within the same scope can read it. |

### Resolution Logic

When a Skill is activated, the runtime:

1. **Matches intents** against `context.intents`
2. **Registers tools** from `context.tools` via ToolRegistry
3. **Resolves filesystem scope** via `ProjectRepository::load_scope()`
4. **Resolves project context** from `pane.project_id`
5. **Opens memory scope** according to `context.memory` — loads existing memory, creates a writable slot for this execution
6. **Loads dependencies** from `skill.json.dependencies.requires`
7. **Validates produces/consumes** — are all consumed artifacts available in memory?

---

## Section 5 — Execution Profile

Skills do not tie themselves to specific models. They declare their performance requirements through an **execution profile**:

```yaml
execution:
  preferred_model_class: reasoning  # fast | reasoning | creative | cheap
  reasoning: high                   # none | low | medium | high
  latency: standard                 # fast | standard | background
  context_budget: 8000              # max tokens for skill body
```

### Model Classes

| Class | Typical Models | When |
|---|---|---|
| `fast` | GPT-5.3-Codex-Spark, Claude Haiku | Simple lookups, file reads, quick checks |
| `reasoning` | GPT-5.5, Claude Sonnet | Analysis, architecture review, security audit |
| `creative` | GPT-5.5, Claude Sonnet | Code generation, documentation writing |
| `cheap` | GPT-5.3-Codex-Mini, Claude Haiku | Bulk processing, low-value tasks |

### Reasoning Depth

| Depth | Behavior |
|---|---|
| `none` | Direct response, no chain-of-thought |
| `low` | Minimal reasoning |
| `medium` | Moderate chain-of-thought |
| `high` | Deep reasoning, multi-step analysis |

### Latency Budget

| Budget | Behavior |
|---|---|
| `fast` | User expects near-instant response (<2s to first token) |
| `standard` | Normal conversational latency (<5s) |
| `background` | No time sensitivity. Can run as subagent or deferred. |

The routing engine (a future component) uses these profiles to select the optimal model for each Skill invocation. Profiles are independent of SKILL.md — the same Skill can be routed to different models in different deployments.

---

## Section 6 — Execution Lifecycle

```
User Input
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 1. INPUT RECEIVED                                            │
│    Frontend → Tauri command → message persisted to SQLite    │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. CONTEXT ASSEMBLY                                          │
│    Project scope resolved (ApprovedScope)                     │
│    Pane configuration loaded (model, provider, builder)       │
│    Memory loaded (scope-appropriate existing state)           │
│    Attachments loaded (if any)                                │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. CAPABILITY RESOLUTION                                     │
│    IntentRouter.analyze(user_message) → Vec<Intent>          │
│    SkillRegistry.match(intents) → Vec<LoadedSkill>            │
│    For each skilled:                                          │
│      - Load SKILL.md from filesystem (lazy)                  │
│      - Validate produces/consumes against available artifacts │
│      - Open memory scope                                     │
│      - Register tools in ToolRegistry                        │
│      - Load dependencies                                     │
│    Event: skill_matched / skill_loaded                        │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. ENRICHMENT (parallel with step 3)                         │
│    Filesystem enrichment (existing Phase 8 pipeline)         │
│    Memory enrichment (inject relevant memory into context)    │
│    Artifact enrichment (inject consumed artifact summaries)   │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. EXECUTION (varies by mode)                                │
│                                                              │
│    Mode: conversation (default)                               │
│      Skill instructions → System message                     │
│      Tool descriptions → Tool block                          │
│      Enriched context → Model input                          │
│      Provider.stream() → Streaming response                  │
│                                                              │
│    Mode: subagent                                            │
│      Spawn isolated Agent with:                              │
│        - Skill instructions as system prompt                 │
│        - Restricted tool access from context.tools           │
│        - Isolated context window                             │
│        - Memory scope (inherited from parent)                │
│      Parent awaits structured result                         │
│      Result injected as artifact in parent conversation       │
│                                                              │
│    Mode: background                                          │
│      Same as subagent but parent does not await              │
│      Result delivered via event when complete                │
│                                                              │
│    Event: execution_started / execution_completed / failure  │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 6. ARTIFACT PRODUCTION                                       │
│    Model response → structured artifact                      │
│    Artifact validated against output contract                │
│    Artifact enters Review queue (see Section 7)              │
│    Event: artifact_produced(artifact_type, summary)          │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 7. REVIEW (optional, depends on builder config)              │
│    Human reviews artifact in UI                              │
│    Or automated review (lint, validate)                      │
│    Status: accepted | rejected | needs_revision              │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 8. MEMORY COMMIT                                             │
│    If persistence == persistent:                              │
│      Write artifact summary to memory store                  │
│      Update scope-appropriate memory keys                    │
│    If visibility == shared:                                  │
│      Make artifact available to other skills within scope    │
│    Event: memory_updated(scope, key)                         │
└──────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 9. RESPONSE                                                  │
│    Streaming message completes                               │
│    Frontend updates UI                                       │
└──────────────────────────────────────────────────────────────┘
```

### Lifecycle Properties

- **Skills load AFTER context assembly, BEFORE enrichment.** Context is available; Skills add capability on top.
- **Multiple Skills can load per turn.** Each with independent context, tools, and memory scope.
- **Skill loading is lazy.** Only matched Skills load. Unmatched Skills never enter context.
- **Execution mode is per-Skill.** One conversation can mix conversation-mode and subagent-mode Skills.
- **Artifact production is the terminal phase.** Every execution produces zero or one primary artifact.

---

## Section 7 — Artifact Model

Artifacts are the bridge between Skills. A Skill does not pass its prompt to another Skill; it produces an artifact that another Skill consumes.

### Artifact Types

| Type | Produced By | Consumed By | Content |
|---|---|---|---|
| `architecture_review` | architecture-review skill | Any review skill | Module boundaries, dependency graph, code organization findings |
| `security_audit` | security-audit skill | production-readiness workflow | Vulnerabilities, credential leaks, auth issues |
| `technical_debt` | technical-debt-review skill | production-readiness workflow | Outdated deps, deprecated APIs, complexity hotspots |
| `implementation_plan` | Any planning skill | code-gen skill | Step-by-step implementation instructions |
| `review_item` | Any review skill | Review queue | Individual finding with severity, location, recommendation |
| `migration_plan` | migration-review skill | implementation skill | Migration steps, compatibility concerns, rollback plan |
| `decision_record` | Any skill with human approval | Project memory | Architecture Decision Record (ADR) |
| `task_list` | planning skill | execution tracking | Ordered tasks with assignments, dependencies |

### Artifact Lifecycle

```
DRAFT ──→ REVIEW ──→ ACCEPTED ──→ DURABLE
  │                    │
  │                    └──→ REJECTED
  │                           │
  └──→ CANCELLED              └──→ REVISED ──→ REVIEW (loop)
```

| Phase | Description |
|---|---|
| **DRAFT** | Artifact created by Skill execution. Not yet reviewed. |
| **REVIEW** | Artifact entered review queue. Waiting for human or automated review. |
| **ACCEPTED** | Review passed. Artifact is considered correct. |
| **DURABLE** | Accepted artifact committed to project memory. Survives across sessions. |
| **REJECTED** | Review failed. Artifact is not committed. |
| **CANCELLED** | Execution was cancelled before artifact completion. |
| **REVISED** | Artifact was updated after rejection and re-enters review. |

### Artifact Storage

```sql
CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    skill_name TEXT NOT NULL,
    artifact_type TEXT NOT NULL,      -- from the type table above
    pane_id TEXT REFERENCES panes(id),
    project_id TEXT REFERENCES workspaces(id),
    status TEXT NOT NULL DEFAULT 'draft',  -- draft | review | accepted | durable | rejected | cancelled
    content_json TEXT NOT NULL,       -- structured artifact content
    summary TEXT,                     -- human-readable summary (1-3 sentences)
    metadata_json TEXT DEFAULT '{}',  -- extensible
    produced_at TEXT NOT NULL,
    reviewed_at TEXT,
    reviewed_by TEXT,                 -- user or automated
    committed_at TEXT,
    parent_artifact_id TEXT REFERENCES artifacts(id),  -- for revision chains
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_artifacts_skill ON artifacts(skill_name);
CREATE INDEX idx_artifacts_project ON artifacts(project_id);
CREATE INDEX idx_artifacts_type_status ON artifacts(artifact_type, status);
```

### Output Contract Validation

Skill's `produces` and `consumes` declarations must match the available artifact types. At execution time:

1. If a Skill declares `consumes: [security_audit]` but no `security_audit` artifact exists in memory scope → the Skill loads but warns "consumed artifact not available"
2. If a Skill declares `produces: [security_audit]` but the artifact type is not registered in the runtime → the Skill loads but the artifact enters the review queue with a validation note
3. If a Skill's output does not match the artifact type schema → the artifact is flagged as `malformed` and is not accepted into memory

### Artifact Composition

Skills compose through artifact chains:

```
architecture-review skill
  └→ produces: architecture_review
      └→ consumed by: security-audit skill
          └→ produces: security_audit
              └→ consumed by: production-readiness workflow skill
                  └→ produces: review_item[] (multiple findings)
                      └→ consumed by: Review Queue (UI)
```

This is NOT prompt chaining. Each Skill executes independently. It receives a structured artifact summary (not the full conversation) from the previous Skill.

---

## Section 8 — Builder Bundles

Builders are not agents. Builders are **named configurations of Skills + layout + execution preferences + review behavior**. They are compositions, not implementations.

### BUILDER.yaml

```yaml
# .builderboard/builders/builder-a.yaml
name: builder-a
display_name: "Builder A — Full Codebase Review"
description: "Comprehensive codebase analysis across architecture, security, quality, and technical debt"
version: 1.0.0

default_skills:
  - architecture-review
  - security-audit
  - code-quality-review
  - technical-debt-review
  - dependency-review

execution:
  preferred_model_class: reasoning
  reasoning: high
  latency: standard

review:
  mode: sequential            # sequential | parallel | interactive
  require_approval: true      # human must review before memory commit
  auto_accept_threshold: low  # auto-accept findings below this severity

layout:
  panes: 2                    # preferred pane count
  sidebar: project-rail       # preferred sidebar mode

memory:
  project:
    enabled: true
    persistence: persistent
    visibility: shared
  user:
    enabled: false

produce:
  - architecture_review
  - security_audit
  - technical_debt
  - review_item[]

consume:
  - project_memory            # reads project knowledge
```

### Builder A — Full Codebase Review

- **Skills:** architecture-review, security-audit, code-quality-review, technical-debt-review, dependency-review
- **Execution:** reasoning model, high-reasoning, sequential review
- **Approval:** human must approve before memory commit
- **Layout:** 2 panes, project rail sidebar
- **Produces:** architecture_review, security_audit, technical_debt, review_item[]

### Builder B — Quick Code Explain

```yaml
name: builder-b
display_name: "Builder B — Quick Code Explain"
default_skills:
  - explain-code
execution:
  preferred_model_class: fast
  reasoning: low
  latency: fast
review:
  mode: interactive
  require_approval: false
memory:
  project:
    enabled: false
```

### Builder C — Production Readiness

```yaml
name: builder-c
display_name: "Builder C — Production Readiness"
default_skills:
  - production-readiness
execution:
  preferred_model_class: reasoning
  reasoning: high
  latency: background
review:
  mode: sequential
  require_approval: true
  auto_accept_threshold: low
memory:
  project:
    enabled: true
    persistence: persistent
    visibility: shared
produce:
  - production_readiness_report
  - decision_record
consume:
  - architecture_review
  - security_audit
  - technical_debt
```

### Builder Resolution

At pane creation (or `/builder-a` command):
1. Load BUILDER.yaml from `.builderboard/builders/`
2. Resolve all `default_skills` → load each Skill
3. Apply `execution` profile to the pane
4. Apply `layout` to the UI
5. Apply `review` behavior (sequential mode creates a review queue)
6. Apply `memory` configuration
7. Begin execution

Builders are discovered, indexed, and cached the same way as Skills (filesystem + SQLite).

---

## Section 9 — Marketplace Readiness

### Separation of Concerns

| Aspect | Format | Purpose |
|---|---|---|
| Human guidance | `SKILL.md` | Instructions, best practices, examples |
| Machine metadata | `skill.json` | Versions, permissions, dependencies, outputs, trust |

`skill.json` is the **source of truth** for marketplace operations. `SKILL.md` is the human-readable companion.

### skill.json — Complete Schema

```json
{
  "schema_version": 1,
  "name": "security-audit",
  "version": "1.0.0",
  "display_name": "Security Audit",
  "description": "...",

  "author": {
    "name": "BuilderBoard",
    "url": "https://builderboard.app",
    "email": "security@builderboard.app"
  },

  "license": "MIT",
  "homepage": "https://builderboard.app/skills/security-audit",
  "repository": "https://github.com/builderboard/skills/security-audit",

  "permissions": {
    "tools": ["list_directory", "read_file", "search_files", "find_files"],
    "filesystem": "required",
    "project": "required",
    "memory": {
      "scope": "project",
      "persistence": "session",
      "visibility": "private"
    }
  },

  "compatibility": {
    "platforms": ["builderboard"],
    "min_app_version": "0.8.0",
    "models": ["openai/*", "anthropic/*"]
  },

  "trust_tier": "verified",
  "trust_info": {
    "reviewed_by": "BuilderBoard Security Team",
    "reviewed_at": "2026-06-24T00:00:00Z",
    "review_url": "https://builderboard.app/skills/security-audit/review"
  },

  "dependencies": {
    "requires": [
      {"skill": "architecture-review", "version": ">=1.0.0"}
    ],
    "optional": [
      {"skill": "dependency-review", "version": ">=1.0.0"}
    ]
  },

  "outputs": {
    "produces": ["security_audit", "review_item"],
    "consumes": ["architecture_review", "dependency_audit"]
  },

  "evals": {
    "test_projects": ["sample-node-app", "sample-rust-app"],
    "expected_behaviors": ["detect_hardcoded_secrets", "identify_auth_bypass"],
    "pass_rate": 0.94
  },

  "signature": {
    "algorithm": "ed25519",
    "public_key_fingerprint": "SHA256:abc...",
    "signature_hex": "..."
  },

  "download_url": "https://registry.builderboard.app/v1/skills/security-audit/1.0.0.zip",
  "checksum_sha256": "abc123...",
  "size_bytes": 24576,

  "stats": {
    "downloads": 15000,
    "rating": 4.7,
    "rating_count": 234,
    "last_updated": "2026-06-24T00:00:00Z"
  }
}
```

### Trust Tiers

| Tier | Criteria | Behavior |
|---|---|---|
| `verified` | Reviewed by BuilderBoard team | Auto-install, full permissions |
| `community` | Published by community member | Ask user before install, limited permissions |
| `sandboxed` | Untrusted source | Sandboxed execution, no memory access, no project access |
| `blocked` | Known malicious | Cannot be installed |

### Distribution Protocol

```
Install:
  builderboard skill install security-audit
    1. Query registry for latest version
    2. Download zip from download_url
    3. Verify checksum_sha256
    4. Verify signature (if signed)
    5. Extract to ~/.config/builderboard/skills/security-audit/
    6. Validate skill.json and SKILL.md
    7. Add to SQLite index
    8. Emit: skill_installed(name, version)

Update:
  builderboard skill update security-audit
    1. Check registry for newer version
    2. Follow install flow

Remove:
  builderboard skill remove security-audit
    1. Remove from filesystem
    2. Remove from SQLite index
    3. Emit: skill_removed(name)
```

---

## Section 10 — Events & Observability

### Event Catalog

| Event | Payload | When | Consumer |
|---|---|---|---|
| `skill_matched` | `{ skill_name, intent, confidence }` | Intent → Skill mapping succeeded | Frontend, trace |
| `skill_loaded` | `{ skill_name, token_count }` | Skill body loaded from filesystem | Trace |
| `skill_install` | `{ skill_name, version }` | Skill installed/updated | Index, frontend |
| `skill_remove` | `{ skill_name }` | Skill deleted | Index, frontend |
| `context_assembled` | `{ project, tools_count, memory_keys }` | Context wiring complete | Trace |
| `execution_started` | `{ mode, model, tokens_in, profile }` | Execution begins | Perf, trace |
| `execution_chunk` | `{ tokens, duration_ms }` | Each streaming chunk (throttled) | Frontend |
| `execution_completed` | `{ tokens_out, duration_ms }` | Execution ends | Perf, trace |
| `execution_failed` | `{ error, duration_ms }` | Execution failed | Error tracking |
| `artifact_produced` | `{ artifact_type, status }` | Artifact created | Review queue, trace |
| `artifact_reviewed` | `{ artifact_type, status, reviewer }` | Artifact reviewed | Memory, frontend |
| `memory_committed` | `{ scope, key }` | Memory written | Memory store, trace |
| `memory_loaded` | `{ scope, keys_count }` | Memory loaded for execution | Trace |
| `subagent_spawned` | `{ subagent_id, skills, depth }` | Subagent created | Trace |
| `subagent_completed` | `{ subagent_id, result_summary }` | Subagent returned | Trace |

### Structured Execution Trace

```json
{
  "execution_id": "uuid",
  "pane_id": "...",
  "builder": "builder-a",
  "timestamp": "2026-06-24T12:00:00Z",
  "events": [
    {"type": "context_assembled", "project": "pepfox", "tools": 4, "memory_keys": 2, "at": "..."},
    {"type": "skill_matched", "skill": "security-audit", "intent": "security_review", "confidence": 0.92, "at": "..."},
    {"type": "skill_loaded", "skill": "security-audit", "token_count": 3200, "at": "..."},
    {"type": "enrichment_completed", "tool_calls": 3, "at": "..."},
    {"type": "execution_started", "mode": "conversation", "model": "gpt-5.5", "tokens_in": 28000, "at": "..."},
    {"type": "execution_completed", "tokens_out": 450, "duration_ms": 8400, "at": "..."},
    {"type": "artifact_produced", "type": "security_audit", "status": "draft", "at": "..."},
    {"type": "memory_committed", "scope": "project", "key": "security_audit_results", "at": "..."}
  ],
  "duration_ms": 9200,
  "total_tokens": 28450,
  "skills_used": ["security-audit"],
  "artifacts_produced": ["security_audit"],
  "result": "success"
}
```

Traces are emitted as Tauri events in real time and stored in a ring buffer (last 100) for the `/debug` panel. Future: deduplicated event log in SQLite for analytics, audit, and playback debugging.

---

## Section 11 — Security Model

### Principle

**Skills declare desired access. The runtime enforces actual access.** A Skill cannot escalate its privileges beyond what the runtime allows.

### Permission Layers (evaluated in order)

```
Layer 1: Global Runtime Defaults (deny by default)
  - All tools: denied
  - All filesystem access: denied
  - All project access: denied
  - Memory write: denied
  - Script execution: denied

Layer 2: User Permissions
  - ~/.config/builderboard/permissions.yaml

Layer 3: Project Permissions
  - .builderboard/permissions.yaml (team-managed)

Layer 4: Builder Permissions
  - BUILDER.yaml (inherited by builder sessions)

Layer 5: Skill Declarations (skill.json)
  - permissions.tools
  - permissions.filesystem
  - permissions.project
  - permissions.memory

Layer 6: Session Permissions (per-turn user approval)
```

### Permission Resolution

For each resource, evaluation proceeds through layers 1–6:
1. **If any layer denies** → resource is unavailable. Stop.
2. **If any layer asks** → user is prompted. Wait for response.
3. **If all layers allow or are silent** → resource is available.

### Filesystem Containment

All filesystem operations go through `ApprovedScope` (existing Phase 8 infrastructure). Skills cannot:
- Escape the project root (traversal prevention)
- Access files outside the approved scope
- Follow symlinks outside the scope
- Write to filesystem (unless explicitly allowed by a `write_file` tool)

### Script Execution

Skills can bundle scripts in the `scripts/` directory. Script execution:
- Always sandboxed (bubblewrap / macOS sandbox-exec / Windows AppContainer)
- No network access unless explicitly allowed
- Timeout enforced (default: 30s, configurable in `skill.json`)
- Output-only enters context (script source code never does)
- Scripts are checksum-verified at install time
- Scripts are inspected at install time for patterns that match known malware signatures

### Trust-Based Permissions

`skill.json.trust_tier` determines the default permission layer:

| Trust Tier | Default Tools | Default Filesystem | Default Memory | Default Scripts |
|---|---|---|---|---|
| `verified` | All declared | Allowed | Allowed | Allowed |
| `community` | Ask per-session | Ask per-session | Denied | Denied |
| `sandboxed` | Denied (subset only) | Sandboxed directory | Denied | Denied |
| `blocked` | Denied | Denied | Denied | Denied |

Users can override at any layer. Trust tiers are a starting point, not an enforcement boundary.

### Permission Manifest (User/Project)

```yaml
# .builderboard/permissions.yaml
skills:
  security-audit:
    enabled: true
    tools: [list_directory, read_file, search_files, find_files]
    filesystem: true
    project: true
    scripts: false              # block script execution for this skill
    memory:
      scope: project
      persistence: persistent
      visibility: private
    context_budget: 8000

  dependency-review:
    enabled: false               # disabled entirely

defaults:
  untrusted_skills:
    enabled: ask                 # ask user before enabling
    filesystem: ask
    memory: deny
```

---

## Section 12 — Builder Bundles (Expanded)

This section expands on Section 8 for completeness. Builder Bundles are the top-level configuration that ties together Skills, execution profiles, memory, review behavior, and layout.

### Directory Structure

```
.builderboard/builders/
├── builder-a.yaml
├── builder-b.yaml
└── builder-c.yaml
```

### Complete Schema

```yaml
name: builder-name                # 1-64 chars, lowercase + hyphens
display_name: "Builder Name"
description: "What this builder does"
version: 1.0.0

# SKILLS
default_skills:                   # loaded automatically
  - architecture-review
  - security-audit
available_skills:                 # available but not auto-loaded
  - dependency-review
  - performance-review

# EXECUTION
execution:
  preferred_model_class: reasoning
  reasoning: high
  latency: standard
  max_steps: 25                   # max agentic loop iterations

# REVIEW BEHAVIOR
review:
  mode: sequential                # sequential | parallel | interactive | none
  require_approval: true
  auto_accept_threshold: low      # none | low | medium | high
  review_queue_priority: normal   # low | normal | high

# LAYOUT
layout:
  panes: 1                        # preferred pane count
  theme: default                  # UI theme
  sidebar: project-rail           # sidebar mode

# MEMORY
memory:
  project:
    enabled: true
    persistence: persistent
    visibility: shared
  user:
    enabled: false
  global:
    enabled: false

# ARTIFACT CONTRACTS
produces:
  - architecture_review
  - security_audit
  - review_item[]

consumes:
  - project_memory
```

---

## Section 13 — Future Compatibility

### Voice

| Component | Compatible? | Notes |
|---|---|---|
| SKILL.md format | Yes | Instructions are text; voice is just a different input modality |
| skill.json | Yes | No changes needed |
| Context block | Yes | `context.intents` works the same; audio-specific intents are additive |
| Execution profile | Yes | Same model routing applies |
| Memory model | Yes | Voice sessions use same memory scope |
| Artifact model | Yes | Voice processing produces the same artifact types |
| Security model | Yes | Same permission layers apply |

**Needed:** Audio-specific intent types (`transcribe`, `analyze_speech`, `generate_speech`). These are additive — existing text-based intents continue to work.

### Attachments

| Component | Compatible? | Notes |
|---|---|---|
| SKILL.md format | Yes | Attachments are part of the provider request, not SKILL.md |
| Context block | Add `context.input_types: [text, image, file]` in v1.2 | Skills that need images declare it |
| Execution profile | Yes | Image-heavy Skills may prefer `reasoning` class models |
| Artifact model | Yes | Attachments can produce `image_analysis` artifact type |

**V1.2 topic:** `context.input_types` to declare what input formats a Skill can process.

### Local Models

| Component | Compatible? | Notes |
|---|---|---|
| Skill format | Yes | No skill changes needed |
| Execution profile | Yes | `preferred_model_class` already separates capability from model identity |
| Artifact model | Yes | Same artifacts regardless of model origin |
| Security model | Yes | Same permissions apply |

**Key design win:** The execution profile (`fast`, `reasoning`, `creative`, `cheap`) does NOT reference specific models. A local Ollama model can serve the `fast` class just as well as GPT-5.3-Codex-Spark. The routing engine is free to choose any model that meets the profile requirements.

### Subagents

| Component | Compatible? | Notes |
|---|---|---|
| Skill format | Yes | `execution: subagent` already defined in context block |
| Execution lifecycle | Yes | Defined in Section 6 (Mode: subagent) |
| Artifact model | Yes | Subagents produce artifacts received by parent |
| Memory model | Yes | Subagents inherit parent's memory scope |
| Security model | Yes | Subagent tools are a subset of parent's allowed tools |

**Already designed in v1.1.** No structural changes needed.

### Marketplace

| Component | Compatible? | Notes |
|---|---|---|
| Skill format | Yes | `skill.json` designed explicitly for marketplace |
| Distribution | Yes | Zip-based, checksum-verified, signature-optional |
| Versioning | Yes | semver from day one |
| Trust tiers | Yes | Designed in Section 11 |
| Dependencies | Yes | `skill.json.dependencies` for versioned dependency resolution |

**Already designed in v1.1.** No structural changes needed.

### Long-Running Workstreams

| Component | Compatible? | Notes |
|---|---|---|
| Execution profile | `latency: background` supports deferred execution | Workstreams that run for hours/days |
| Artifact model | Yes | Partial artifacts can be produced incrementally |
| Memory model | Yes | `persistence: persistent` supports cross-session state |
| Review queue | Yes | Artifacts enter review as they complete |

**V1.2 topic:** Event-driven workstream continuations (when a background artifact is reviewed, resume the parent workflow).

### V1.2 Topics (Explicit Callouts)

| Topic | Why Not in v1.1 |
|---|---|
| `context.input_types` | No attachment support yet; speculative |
| Structured output contracts (formal schema) | Artifact types exist but no formal schema language yet |
| Event log persistence to SQLite | Ring buffer is sufficient for Phase 9 |
| Workstream continuation (event-driven) | Requires artifact review + background execution to be stable first |
| Visual workflow editor | Design the primitives first; GUI editing is Phase 10 |
| Skill testing framework | Needed before marketplace launch, not before Phase 9B |
| Runtime plugin API (third-party tools via MCP) | MCP support is Phase 10; ToolRegistry is sufficient for now |

---

## Section 14 — Reference Examples

### Example 1: security-audit

**SKILL.md:** See Section 3 example.

**skill.json:** See Section 9 example.

**Context declaration:**
```yaml
context:
  intents: [security_review, code_quality_review]
  tools: [list_directory, read_file, search_files, find_files]
  filesystem: required
  project: required
  memory:
    scope: project
    persistence: session
    visibility: private
```

**Execution profile:**
```yaml
execution:
  preferred_model_class: reasoning
  reasoning: high
  latency: standard
  context_budget: 8000
```

**Produces:** `security_audit`, `review_item[]`

**Consumes:** `architecture_review`, `dependency_audit`

### Example 2: production-readiness (workflow skill)

**Directory:**
```
.builderboard/skills/production-readiness/
├── SKILL.md
├── skill.json
├── scripts/
│   └── check_deployment_config.py
└── resources/
    ├── go_live_checklist.md
    └── rollback_templates/
```

**skill.json:**
```json
{
  "name": "production-readiness",
  "version": "1.0.0",
  "display_name": "Production Readiness Review",
  "description": "Full production readiness review across architecture, security, dependencies, and operations",
  "permissions": {
    "tools": ["list_directory", "read_file", "search_files", "find_files", "bash"],
    "filesystem": "required",
    "project": "required"
  },
  "dependencies": {
    "requires": [
      {"skill": "architecture-review", "version": ">=1.0.0"},
      {"skill": "security-audit", "version": ">=1.0.0"},
      {"skill": "dependency-review", "version": ">=1.0.0"}
    ]
  },
  "outputs": {
    "produces": ["review_item", "decision_record"],
    "consumes": ["architecture_review", "security_audit", "technical_debt"]
  }
}
```

**Context declaration:**
```yaml
context:
  intents: [production_readiness_review]
  tools: [list_directory, read_file, search_files, find_files, bash]
  filesystem: required
  project: required
  memory:
    scope: builder
    persistence: session
    visibility: private
```

### Example 3: builder-a (builder bundle)

**BUILDER.yaml:**
```yaml
name: builder-a
display_name: "Builder A — Full Codebase Review"
description: "Comprehensive codebase analysis across architecture, security, quality, and technical debt"
default_skills:
  - architecture-review
  - security-audit
  - code-quality-review
  - technical-debt-review
  - dependency-review
execution:
  preferred_model_class: reasoning
  reasoning: high
  latency: standard
review:
  mode: sequential
  require_approval: true
memory:
  project:
    enabled: true
    persistence: persistent
    visibility: shared
produces:
  - architecture_review
  - security_audit
  - technical_debt
  - review_item[]
```

### Example 4: explain-code (simple skill)

**SKILL.md:**
```yaml
---
name: explain-code
description: Explain what a selected code block or file does
version: 1.0.0
display_name: "Explain Code"
context:
  intents: [explain_code, filesystem_discovery]
  tools: [read_file]
  filesystem: optional
  project: optional
  memory:
    scope: pane
    persistence: session
    visibility: private
execution:
  preferred_model_class: fast
  reasoning: low
  latency: fast
  context_budget: 2000
produces:
  - explanation
consumes: []
---

# Explain Code

## Overview
When the user asks about a specific code file or block, read the file and provide a concise explanation.

## Instructions
1. If the user references a file path, read the file first
2. Provide a concise explanation focused on:
   - What this code does
   - Key design decisions visible in the code
   - Any notable patterns or anti-patterns
3. Keep responses under 500 words unless the user asks for detail

## Best Practices
- Do not provide unsolicited refactoring suggestions
- If the code is complex, offer to run a deeper skill ([[architecture-review]])
```

---

## Section 15 — Mistakes To Avoid

1. **Skills that modify their own SKILL.md** — Skills are instructions, not writable state.
2. **Implicit dependencies** — Declare all dependencies in `skill.json.dependencies`.
3. **Skill body too large** — Keep each Skill focused (< `context_budget` tokens). Use wikilinks to split large knowledge.
4. **Over-declaring tools** — Only list tools the Skill actually uses. Over-declaration bloats the system prompt.
5. **Skills as agent configurations** — Skills are not agents. Builder Bundles (BUILDER.yaml) handle agent configuration.
6. **Mutable skill state** — Skills should produce the same instructions every load. State belongs in Memory.
7. **Hardcoding file paths** — Reference files by role ("read the package file"), not absolute path.
8. **Skills requiring network access** — Skills should work offline. Network features are optional enhancements.
9. **Circular dependencies** — Runtime must detect and reject `A depends on B depends on A`.
10. **One giant "everything" Skill** — Break knowledge into focused Skills and compose via workflows.
11. **Copying prompts into Skills** — Skills capture expertise and process, not instruction text.
12. **Forgetting the metadata budget** — ~100 tokens per Skill at startup. Keep descriptions short.
13. **Skills as the only extensibility** — Skills cover "what". Hooks cover "when". MCP covers "how".
14. **No skill testing before marketplace** — Build a test framework before opening distribution.
15. **Skills that encourage bad practices** — Review Skills for safety before installation or publication.
16. **Storing secrets in skill resources** — Bundled content should not contain credentials.
17. **Ignoring execution mode** — Heavy analysis should use `subagent` mode to avoid context contamination.
18. **Over-nesting subagents** — Default depth limit of 3. Use subagents for genuinely parallel work only.
19. **Marketplace skills that break on update** — Version pinning (`install name@1.2.0`) protects users.
20. **Designing for the 1% case** — Start with the common case (single Skill, intent match, conversation mode). Build visual editors and marketplace recommendation engines only after the basic flow works.

---

## Section 16 — Final Assessment

### Executive Summary

BuilderBoard Skill Spec v1.1 defines a complete architecture for skill-based agent extension. The architecture is built around a single conceptual spine — **Input → Context → Capability → Execution → Artifacts → Review → Memory** — where every feature (Voice, Attachments, Skills, Subagents, Marketplace) is an expression of the same model.

Key innovations over v1 are:
- **Explicit memory model** with scope, persistence, and visibility
- **Structured output contracts** through the artifact system
- **Execution profiles** that decouple Skills from specific models
- **Produce/consume declarations** for artifact-based composition
- **Separated manifests** (`SKILL.md` for humans, `skill.json` for machines)
- **Builder Bundles** as named compositions of Skills

### Major Changes from v1

| Area | v1 | v1.1 |
|---|---|---|
| Memory | `context.memory: none` (placeholder) | Full model: scope, persistence, visibility |
| Outputs | Implicit (model decides format) | Explicit artifact contracts (`produces`/`consumes`) |
| Artifacts | Not a concept | First-class with lifecycle: draft→review→accepted→durable |
| Execution config | `max_tokens: 4000` (single field) | Full profile: model_class, reasoning, latency, context_budget |
| Builders | Not defined | BUILDER.yaml with skills, layout, memory, review config |
| Marketplace | `version` field only | Complete manifest (`skill.json`), trust tiers, distribution protocol |
| Capability philosophy | Implicit | Explicit spine: Input→Context→Capability→Execution→Artifacts→Review→Memory |

### Remaining Open Questions (v1.2)

1. **Input type declarations.** Should Skills declare `context.input_types: [text, image, file]` to specify which input formats they can process? Needed for full attachment/image support.

2. **Formal output schemas.** Artifact types exist but are convention-based. Should artifact types have formal JSON Schema validation? Needed for CI/CD integration and inter-skill contract enforcement.

3. **Event log persistence.** Ring buffer is sufficient for Phase 9. Should execution traces be persisted to SQLite for audit, analytics, and playback debugging?

4. **Workstream continuation.** When a background subagent completes and its artifact is reviewed, should the parent workflow resume automatically? Event-driven continuation.

5. **Skill testing framework.** How do you unit-test a Skill? Expected: mock project + expected artifact output. Needed before marketplace launch.

### Confidence Score

**90/100**

**Rationale:**

- The **memory model** and **artifact system** are the most significant additions, but both are defined with clear scope boundaries and explicit lifecycle states. No open-ended commitments.
- The **execution profile** is the most important design decision — by decoupling Skills from specific models, the architecture survives model churn, local models, and future model classes.
- The **produces/consumes** system is the composition mechanism that replaces prompt chaining. It is simpler than DAG-based orchestration (LangGraph) and more structured than flat skill files (OpenCode).
- The **separated manifests** (`SKILL.md` + `skill.json`) handle both human authoring and machine distribution without forcing either into the wrong format.

The two biggest risks are: (1) the consent model for memory visibility (shared vs private) needs real-world testing, and (2) artifact schema validation may need to be more formal for marketplace Skills. Both are v1.2 refinements, not v1.1 redesigns.

The architecture is simple, modular, maintainable, and scalable. Phase 9B can proceed on this foundation.
