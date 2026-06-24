# BuilderBoard Skill Specification v1

**Status:** Draft  
**Date:** 2026-06-24  
**Author:** BuilderBoard Architecture  
**Applies to:** Phase 9A.5 (specification), Phase 9B–9D (implementation)  
**License:** Apache 2.0

---

## Section 1 — Definitions

Every term in this specification has exactly one meaning. No overlap. No ambiguity.

### Skill

A **Skill** is a reusable, declarable bundle of expertise stored as a filesystem directory with a `SKILL.md` entry point. A Skill:
- Contains **instructions** (markdown guidance for the model)
- Declares **context requirements** (intents, tools, filesystem scope, project access)
- May bundle **scripts** (deterministic operations executed via bash)
- May bundle **resources** (reference materials, templates, schemas)
- Is **discovered automatically** by BuilderBoard at startup
- Is **loaded on demand** (progressive disclosure: metadata → instructions → resources)
- Is **versioned** via semver in the YAML frontmatter

**Not:** A Skill is not an executable. It does not have its own agent loop. It does not own state. It does not persist data across sessions.

**Analogy:** A Skill is a training manual for a specialist. It tells the specialist what to know and what tools to use, but the specialist (the model) does the actual work.

### Agent

An **Agent** is an independent decision-making entity with:
- Its own system prompt (persona, behavior, constraints)
- Its own tool access list (allow/deny)
- Its own model assignment
- Its own agentic loop (gather context → act → verify → repeat)
- Its own isolated context window
- Its own lifecycle (spawn → execute → return → destroy)

**Agent vs Skill:** An agent *does work*. A skill *tells the agent how to work*. The same skill can be loaded by different agents. The same agent can load multiple skills.

**BuilderBoard implementation:** Agents will be implemented as Tauri background processes or subprocesses with isolated SQLite connections. They are not in scope for Phase 9B (they arrive in 9C).

### Subagent

A **Subagent** is an Agent spawned by another Agent. Subagents:
- Run in an isolated context window (parent context is never contaminated)
- Have independent tool access (parent defines the allowlist)
- Return a structured result (parent receives summary, not raw conversation)
- Support configurable depth limits (default: 3, prevents infinite delegation)
- May be nested (subagents can spawn subagents within the depth limit)

**Subagent vs Skill:** A Skill may *instruct* the model to spawn a subagent, but the Skill is not the subagent. The Skill provides the instructions; the runtime spawns the subagent with those instructions as its system prompt.

### Workflow

A **Workflow** is a declared sequence of steps that chains multiple Skills and/or subagents. A Workflow:
- Has explicit step ordering (not model-decided)
- Supports conditional branching (if/else based on step output)
- Has error handling per step (retry, skip, abort)
- Supports human-in-the-loop checkpoints (pause and ask)
- Is itself a Skill (a `SKILL.md` that references other skills via wikilinks)

**Workflow vs Skill:** Every workflow is a Skill. Not every Skill is a workflow. A simple Skill (e.g., "explain this code") is instructions only. A workflow Skill (e.g., "production readiness review") chains multiple sub-skills.

### Tool

A **Tool** is a callable function with typed inputs and typed outputs. Tools:
- Execute deterministically (same inputs → same outputs)
- Are registered in a global `ToolRegistry`
- Are the model's only way to affect the outside world
- Have an allow/deny permission model per agent/skill
- Are discovered via the ToolRegistry, not declared in SKILL.md tools section

**Built-in tools:** `list_directory`, `read_file`, `find_files`, `search_files`, `bash` (sandboxed), `skill` (load a skill), `task` (spawn subagent), `web_fetch`, `web_search`, `write_file` (future).

**Tool vs Skill:** A Tool *does something*. A Skill *describes when and how to use tools*. A Skill may declare `context.tools: [read_file, search_files]` to request that those tool descriptions be included in its system prompt, but the tool implementations exist independently.

### Intent

An **Intent** is a detectable user goal extracted from natural language input. Intents:
- Are detected by `IntentRouter` (pattern/keyword matching or LLM classification)
- Map to Skills via `context.intents`
- Carry typed metadata (e.g., `severity`, `scope`, `language`)
- Are the primary routing mechanism from user input to Skill activation

**Built-in intents:** `architecture_review`, `security_review`, `technical_debt_review`, `production_readiness_review`, `code_quality_review`, `filesystem_discovery`, `project_overview`, `explain_code`, `refactor_code`, `write_code`, `search_codebase`, `debug_issue`.

### Project

A **Project** is a workspace with metadata (`kind: folder`, `approvedRoot`, `name`, `code`). Projects:
- Define the filesystem boundary (`ApprovedScope`)
- Are the scope for Skill execution (all file operations are relative to the project root)
- Are assigned to Panes via `pane.project_id`
- Are resolved by `ProjectRepository` before any Skill loads

### Pane

A **Pane** is a UI container for one conversation thread. Panes:
- Have a `project_id` (determines filesystem scope)
- Have a `provider_id` (determines which LLM provider)
- Have a `model_id` (determines which model)
- Own a list of Messages
- Are the execution context for all chat operations

### Session

A **Session** is a runtime conversation between the user and one Pane. Sessions:
- Begin when the user sends a message
- End when the streaming response completes (or errors)
- Are the lifecycle boundary for Skill execution
- Own the accumulated context (loaded skills, tool results, conversation history)

### Execution

**Execution** is the process of: receiving a user message → resolving intents → loading matching Skills → injecting Skill instructions as system messages → enriching context with tools/scopes → calling the LLM → streaming the response → persisting messages. Execution is synchronous for the user (they wait for a response) but the runtime may perform async operations during preparation.

### Context

**Context** is the accumulated information available to the model for a single turn. It includes:
- System prompt (persona, constraints)
- Skill instructions (loaded from SKILL.md)
- Tool descriptions (from ToolRegistry)
- Project scope (approved root, project metadata)
- Conversation history (previous messages in the pane)
- Filesystem enrichment results (scan context, tool outputs)

Context is **assembled per-turn, never cached across turns** (message history is the only cross-turn state).

### Memory

**Memory** is persisted state that survives across sessions. BuilderBoard does NOT have a general-purpose memory system in Phase 9. Memory is limited to:
- Messages (persisted to SQLite)
- Pane configuration (project, provider, model)
- Skill enable/disable state

Skills do NOT have their own persistent storage. If a Skill needs to remember something across sessions, it must use the message history (via tool results stored as messages with `content_type: json`).

### Event

An **Event** is a timestamped observation emitted by the runtime during execution. Events include:
- `skill_loaded` (skill name, tokens consumed)
- `skill_matched` (intent → skill mapping)
- `tool_invoked` (tool name, duration, success/failure)
- `tool_result` (tool name, output size)
- `execution_started` / `execution_completed` / `execution_failed`
- `subagent_spawned` / `subagent_completed`

Events are emitted to:
1. The frontend (via Tauri events, for real-time UI)
2. The execution trace (for debugging and observability)
3. Future: event log storage (for analytics and audit)

---

## Section 2 — Skill Philosophy

### What Skills Should Solve

1. **Encapsulate expertise.** A Skill captures domain knowledge so the model doesn't need to remember it. Security audit steps, code review criteria, deployment checklists — these are expertise that should live in Skills, not in the model's training data or the user's memory.

2. **Encapsulate workflows.** A Skill can define a sequence of steps (via wikilinks to other Skills or via procedural instructions). Production readiness review = security → architecture → dependencies. The Skill captures the *process*, not just the *knowledge*.

3. **Encapsulate context requirements.** A Skill declares what it needs (intents, tools, filesystem access) so the runtime can automatically wire those resources without manual configuration.

4. **Reduce repetition.** Skills replace copy-pasted instructions across projects. Install once, use everywhere.

5. **Enable composition.** Skills reference other Skills via wikilinks, building a navigable graph of expertise that the model can traverse on demand.

### What Skills Should NOT Solve

1. **Encapsulate prompts.** A Skill is not a prompt template. It does not contain `{user_variables}`. It does not generate text via template expansion. Skills are read by the model as context, not executed as templates.

2. **Encapsulate tools.** Skills describe *how to use* tools, but they do not *implement* tools. Tool implementations live in the Rust `ToolRegistry`. A Skill can request access to `search_files`, but the actual `search_files` function is a separate concern.

3. **Encapsulate agent behavior.** Skills do not define agent personas. They do not set system prompts or model parameters. They do not configure tool permissions. Agent configuration is a separate layer (see Section 8: Builder Personas).

4. **Encapsulate state.** Skills are stateless. They do not have persistent storage. They do not maintain counters, caches, or accumulated results across invocations. All state belongs to the Session or the Messages table.

5. **Encapsulate security boundaries.** Skills define *desired* access, not *enforced* access. Security boundaries (tool permissions, filesystem containment, project isolation) are enforced by the runtime, not by the Skill.

6. **Replace conversation.** Skills augment conversation. They do not replace the user's messages or the model's responses. The model always sees the full conversation context; Skills are additional context injected alongside it.

7. **Replace the model's judgment.** Skills provide guidance, not instructions. The model decides whether and how to follow the Skill's advice. A Skill should be written as "Consider X when Y happens" not "Always do X".

---

## Section 3 — Skill File Format

### File Location

Every Skill lives in its own directory with a mandatory `SKILL.md` file:

```
.builderboard/skills/<skill-name>/SKILL.md
```

The directory name MUST match the `name` field in the YAML frontmatter.

### Schema

```yaml
---
# REQUIRED FIELDS
name: skill-name                      # 1-64 chars, lowercase + hyphens only
description: What this skill does     # 10-200 chars, shown in tool description
version: 1.0.0                        # semver, required for marketplace support

# RECOMMENDED FIELDS
author: ""                            # name or org
license: MIT                          # SPDX identifier

# OPTIONAL FIELDS
display_name: "Security Audit"        # Human-readable name (defaults to name)
categories:                           # For marketplace browsing
  - security
  - code-review
tags:                                 # For search/discovery
  - audit
  - vulnerability
  - secrets
icon: ""                              # Emoji or icon reference
compatibility:                        # Compatible agent types
  - builderboard
min_app_version: 0.8.0                # Minimum BuilderBoard version

# CONTEXT DECLARATION (see Section 4)
context:
  intents: []                         # Which intents trigger this skill
  tools: []                           # Which tools this skill needs
  filesystem: optional                # none | optional | required
  project: optional                   # none | optional | required
  execution: conversation             # conversation | subagent | background
  max_tokens: 4000                    # Max tokens for skill instructions
  depends_on: []                      # Skill names this skill requires
  optional_skills: []                 # Skills that enhance this skill

# METADATA (extensible)
metadata:
  key: value                          # Arbitrary key-value pairs
---

# Skill Title

## Overview

Brief description of what this skill provides. 2-3 sentences max.

## Instructions

Step-by-step guidance for the model. Use markdown formatting.

### Step 1: Do X
- Check for Y
- If Z is found, do W

### Step 2: Do Y
...

## Best Practices

- Do this
- Don't do that

## Examples

### Example 1: Basic usage
```
Input: "Check my codebase for security issues"
Expected: Run security scan, report findings
```

### Wikilinks

For workflow skills that compose other skills:
- [[security-audit]] — Run security audit
- [[architecture-review]] — Review architecture

## Resources

Reference materials are in the skill directory:
- [patterns.yaml](patterns.yaml) — Custom search patterns
- [checklist.md](checklist.md) — Verification checklist
```

### Key Design Decisions

1. **Version is mandatory.** Even for local-only skills, versioning enables marketplace migration later.
2. **Context block is the innovation.** It bridges "here's what I know" (instructions) with "here's what I need" (runtime resources).
3. **No template variables.** Skills are static documents. They do not accept arguments. Variable needs are handled by the context block (the runtime wires the correct scope/project).
4. **Wikilinks for composition.** `[[skill-name]]` syntax in the body declares a dependency relationship that the runtime uses for graph traversal.
5. **Metadata is extensible.** The `metadata` block accepts arbitrary key-value pairs for future use (marketplace ratings, usage stats, etc.).

### Field Validation Rules

| Field | Rule |
|---|---|
| `name` | `^[a-z][a-z0-9-]{0,63}$` |
| `version` | Valid semver (`^\\d+\\.\\d+\\.\\d+$`) |
| `description` | No XML/HTML tags |
| `context.intents` | Must reference known intent names |
| `context.tools` | Must reference known tool names |
| `context.filesystem` | One of: `none`, `optional`, `required` |
| `context.project` | One of: `none`, `optional`, `required` |
| `context.execution` | One of: `conversation`, `subagent`, `background` |
| `SKILL.md` body | Must not exceed `context.max_tokens` when tokenized |

---

## Section 4 — Context Declaration

This is the most important section of this specification. The `context` block in SKILL.md frontmatter determines how the BuilderBoard runtime automatically wires resources for each Skill.

### Declaration Schema

```yaml
context:
  # INTENT MAPPING (required at least one)
  # Which user intents trigger automatic loading of this Skill
  intents:
    - security_review
    - code_quality_review

  # TOOL ACCESS (recommended)
  # Which tools this Skill needs to function
  tools:
    - list_directory
    - read_file
    - search_files
    - find_files

  # FILESYSTEM ACCESS (recommended)
  # Whether this Skill needs filesystem operations
  filesystem: required    # none | optional | required

  # PROJECT ACCESS (recommended)
  # Whether this Skill needs a project context
  project: required       # none | optional | required

  # MEMORY ACCESS (future)
  # Whether this Skill needs persisted memory
  memory: none            # none | read | write | readwrite

  # EXECUTION MODE (optional, default: conversation)
  # How this Skill runs
  execution: conversation  # conversation | subagent | background

  # TOKEN BUDGET (optional, default: 4000)
  # Maximum tokens for skill instructions (not tool results)
  max_tokens: 8000

  # DEPENDENCIES (optional)
  # Skills that must be loaded before this one
  depends_on:
    - architecture-review

  # OPTIONAL ENHANCEMENTS (optional)
  # Skills that enhance this one if also loaded
  optional_skills:
    - dependency-review
```

### Resolution Logic

When a Skill is activated, the runtime follows this resolution order:

1. **Intent match:** The user's input is routed through `IntentRouter`. If the detected intent appears in `context.intents`, the Skill is eligible for loading.

2. **Tool registration:** All tool names in `context.tools` are looked up in the `ToolRegistry`. Their JSON schemas are added to the system prompt. If a tool is not registered, the runtime logs a warning and continues (the Skill may still work with reduced functionality).

3. **Filesystem scope resolution:** If `context.filesystem` is `required` or `optional`, the runtime resolves the project's `ApprovedScope` via `ProjectRepository::load_scope(pane.project_id)`. If `required` and no scope is available, the Skill activation fails with a clear error.

4. **Project context resolution:** If `context.project` is `required` or `optional`, the runtime loads the project metadata (name, code, approved root) and injects it as context. If `required` and no project is available, the Skill activation fails.

5. **Dependency resolution:** Skills in `depends_on` are loaded first. Their instructions are prepended (in order) to the Skill's instructions. Circular dependencies cause a load failure.

6. **Execution mode routing:** If `execution` is `subagent`, the runtime spawns a subagent with the Skill's instructions as its system prompt. If `conversation` (default), instructions are injected as a system message in the current conversation.

### Complete Resolution Example

```yaml
# Skill: security-audit
context:
  intents: [security_review]
  tools: [list_directory, read_file, search_files, find_files]
  filesystem: required
  project: required
  execution: conversation
  max_tokens: 8000
```

When user says "Check my code for security issues":
1. `IntentRouter` returns `security_review`
2. `security_review` matches `security-audit.intents`
3. Runtime reads `security-audit/SKILL.md` from filesystem
4. Runtime resolves project scope from `pane.project_id`
5. Runtime registers `list_directory`, `read_file`, `search_files`, `find_files` in tool descriptions
6. Runtime injects SKILL.md body as a system message
7. Model executes with security expertise + correct tools + correct scope

---

## Section 5 — Execution Lifecycle

### Exact Sequence

```
User Input
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 1. INPUT RECEIVED                                     │
│    Frontend sends message_create → stream_chat        │
│    Backend receives Tauri command                      │
│    User message persisted to SQLite (status=complete)  │
│    Assistant placeholder created (status=pending)      │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 2. INTENT RESOLUTION                                  │
│    IntentRouter.analyze(user_message)                  │
│    Returns: Vec<Intent> with confidence scores         │
│    Example: [security_review: 0.92, filesystem: 0.10] │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 3. SKILL RESOLUTION                                   │
│    SkillRegistry.match(intents)                        │
│    For each matched intent:                            │
│      Look up Skills with matching context.intents      │
│      Filter by enabled status                          │
│      Sort by version (latest wins)                     │
│      Load SKILL.md from filesystem                     │
│    Returns: Vec<LoadedSkill>                           │
│    Event emitted: skill_matched(skill_name, intent)    │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 4. CONTEXT WIRING                                     │
│    For each loaded Skill:                              │
│      Resolve context.project → ProjectRepository       │
│      Resolve context.filesystem → ApprovedScope        │
│      Resolve context.tools → ToolRegistry.schemas()    │
│      Resolve context.depends_on → load prerequisites   │
│      Validate all required resources are available     │
│    If required resource missing → fail with error      │
│    Event emitted: skill_loaded(skill_name, token_cost) │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 5. FILESYSTEM ENRICHMENT (if intent matches)          │
│    Existing Phase 8 pipeline:                          │
│      route_filesystem_tools(prompt, intents)           │
│      prepare_filesystem_enrichment(scope, prompt)      │
│      Optionally execute tool calls for scan context    │
│    Tool results injected as System message             │
│    Event emitted: enrichment_completed(tokens_used)    │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 6. MODEL CONTEXT ASSEMBLY                             │
│    Final system prompt =                               │
│      [base system prompt]                              │
│      + [Skill instructions (loaded SKILL.md bodies)]   │
│      + [Tool descriptions (from ToolRegistry)]         │
│      + [Project context (name, root)]                  │
│      + [Filesystem enrichment results]                 │
│      + [Conversation history]                          │
│    Truncation: oldest messages first if over limit     │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 7. MODEL EXECUTION                                    │
│    Provider.stream_chunks_async(assembled_context)     │
│    Streaming: each chunk → MessageRepository.update   │
│    Streaming: each chunk → Tauri event to frontend    │
│    Event emitted: execution_started(model, tokens_in) │
│    Event emitted: execution_completed(tokens_out)     │
└──────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────┐
│ 8. RESPONSE (streaming completes)                     │
│    Message status set to 'complete'                    │
│    Tauri event: message_stream_complete                │
│    Frontend updates UI                                 │
└──────────────────────────────────────────────────────┘
```

### Key Properties

- **Skills load AFTER intent resolution, BEFORE context assembly.** This ensures Skill instructions are part of the model's input, not applied after the fact.
- **Multiple Skills can load per turn.** If the intent matches 3 Skills, all 3 are loaded (subject to token budget limits).
- **Skill loading is lazy.** Only matched Skills are loaded. Unmatched Skills never enter context.
- **Filesystem enrichment is parallel to Skill loading.** Both happen in the "preparation" phase before the model call.
- **Execution mode routing happens here.** If any matched Skill declares `execution: subagent`, the context assembly for that Skill goes to a subagent instead of the main conversation.

---

## Section 6 — Skill Discovery

### Filesystem Layout

BuilderBoard searches for Skills in this order (later overrides earlier for same `name`):

```
1. BUILT-IN (read-only, app bundle)
   <app>/resources/builderboard/skills/<name>/SKILL.md

2. PROJECT (version-controlled)
   <project-root>/.builderboard/skills/<name>/SKILL.md

3. USER (personal, cross-project)
   ~/.config/builderboard/skills/<name>/SKILL.md
```

### Directory Structure per Skill

```
.builderboard/skills/
├── security-audit/
│   ├── SKILL.md              # Required: main instructions
│   ├── patterns.yaml         # Optional: custom search patterns [1]
│   ├── scripts/              # Optional: deterministic scripts
│   │   └── check_secrets.py
│   └── resources/            # Optional: reference materials
│       └── api_schemas.md
│
├── architecture-review/
│   ├── SKILL.md
│   └── templates/
│       └── arch_report.md
│
└── production-readiness/
    ├── SKILL.md
    └── checklists/
        └── go_live.md
```

[1] `patterns.yaml`: Custom keyword patterns for intent detection, skill-specific. These are merged with the global intent patterns at runtime.

### SQLite Index

Skills are indexed in the `skills` SQLite table:

```sql
CREATE TABLE skills (
    id TEXT PRIMARY KEY,                    -- UUID
    name TEXT NOT NULL UNIQUE,              -- lowercase-hyphenated
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    version TEXT NOT NULL,                  -- semver
    author TEXT DEFAULT '',
    license TEXT DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,     -- 0 = disabled, 1 = enabled
    is_builtin INTEGER NOT NULL DEFAULT 0,  -- 1 = bundled with app
    source_path TEXT NOT NULL,              -- absolute filesystem path
    source_type TEXT NOT NULL DEFAULT 'project',  -- 'builtin' | 'project' | 'user'
    context_json TEXT NOT NULL,             -- serialized context declaration
    metadata_json TEXT DEFAULT '{}',        -- extensible metadata
    token_count INTEGER DEFAULT 0,          -- approximate token count of SKILL.md body
    installed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_skills_enabled ON skills(enabled);
CREATE INDEX idx_skills_name ON skills(name);
CREATE UNIQUE INDEX idx_skills_name_source ON skills(name, source_type);
```

### Startup Behavior

1. **Scan filesystem directories** (builtin, project, user).
2. **Parse SKILL.md** for each discovered Skill.
3. **Upsert into SQLite index** (match by `name + source_type`).
4. **Detect removals** (Skills in DB but not on disk → mark `enabled = 0`).
5. **Load metadata** (name + description) into system prompt.
6. **Full SKILL.md body** remains on disk — not loaded into memory.

Total metadata cost: ~100 tokens per Skill. With 50 skills = ~5000 tokens at startup.

### Hot Reload

- **File watcher** monitors `.builderboard/skills/` and `~/.config/builderboard/skills/`.
- On change: re-scan, re-index, re-load metadata into system prompt.
- Skill body changes take effect on next Skill load (not immediate).
- Frontend notified via Tauri event when skill list changes.
- No restart required.

### Caching

- **Metadata cache:** In-memory HashMap of `name → (display_name, description, version)`. Invalidated on file change.
- **Body cache:** LRU cache of recently loaded SKILL.md bodies (max 10 skills, 5 minutes TTL). Cleared on file change.
- **Index cache:** SQLite is the source of truth. Reads are fast (indexed queries). No additional caching layer needed.

---

## Section 7 — Skill Composition

### Can Skills Call Skills?

**Yes, via wikilinks.** A Skill body can contain `[[skill-name]]` references. The runtime interprets these as navigable edges in a skill graph.

### Can Skills Depend on Skills?

**Yes, via `context.depends_on`.** A Skill can declare prerequisite Skills that must be loaded first. Dependency resolution is:
1. Topological sort of the dependency graph
2. Load dependencies in order (prerequisites first)
3. Fail on circular dependencies (detectable in O(V+E))

### Can Skills Inherit from Skills?

**No.** There is no inheritance. Composition replaces inheritance. A Skill that needs another Skill's functionality uses:
- `depends_on` (prerequisite — load both)
- `optional_skills` (enhancement — load if available)
- `wikilinks` (reference — navigate at runtime)

### Can Skills Compose Workflows?

**Yes, via wikilinks with ordered context.** The recommended pattern is a "workflow Skill" whose body is primarily wikilinks:

```markdown
# Production Readiness Review

## Pre-requisites
This workflow requires these Skills loaded first:
- [[code-quality-review]] — Check code quality baseline
- [[dependency-review]] — Check dependency health

## Ordered Steps

### Step 1: Architecture Review
Run [[architecture-review]] to verify:
- Module boundaries are clean
- Dependency injection is correct
- No circular imports

### Step 2: Security Audit
Run [[security-audit]] to verify:
- No credential leaks
- Auth flows are correct
- Input validation exists

### Step 3: Performance Review
If the project is user-facing, run [[performance-review]].

## Output
Compile findings into a JSON report with:
- pass/fail per section
- Critical issues
- Recommendations
```

The runtime:
1. Loads the workflow Skill (production-readiness)
2. Detects wikilinks → registers them as dependencies
3. Loads dependency Skills (code-quality-review, dependency-review)
4. The model navigates to sub-skills on demand during execution

### Composition Patterns

| Pattern | Mechanism | Use Case |
|---|---|---|
| **Sequential** | Workflow SKILL.md with ordered wikilinks | Production readiness review |
| **Parallel** | Multiple Skills loaded from multiple matched intents | User says "Check security and architecture" |
| **Conditional** | Skill body instructs model to check conditions | "If project is user-facing, run performance review" |
| **Hierarchical** | Skill declares `execution: subagent` for sub-tasks | Deep analysis tasks |
| **Enhancement** | `optional_skills` for additive capabilities | "Use dependency-review if available" |

### Recommendation

**Start with flat + wikilinks.** Do NOT implement full dependency resolution or graph traversal in Phase 9B. Start with:
1. Single Skills loaded by intent matching (flat)
2. Workflow Skills that contain wikilinks but expect the model to navigate them (model-routed)
3. No automatic recursive loading of wikilink targets

**Add dependency resolution in Phase 9C** when the skill graph grows large enough to need it.

---

## Section 8 — Builder Personas

### What Are Builders?

Builders are **named configurations of Skills + tool permissions + project context**. They are NOT Skills themselves. They are NOT Agents. They are **preset activation patterns**.

- **Builder A**: "Full codebase review" — loads `architecture-review`, `security-audit`, `code-quality-review`, `technical-debt-review`. Full tool access. Project-required.
- **Builder B**: "Quick code explain" — loads `explain-code`. Read-only tools. Project-optional.
- **Builder C**: "Production readiness" — loads `production-readiness` workflow. Full tool access. Project-required.

### Why Not Agents?

Builders do NOT have:
- Independent system prompts (they use the conversation's existing persona)
- Independent tool permissions (they use the pane's configured permissions)
- Independent models (they use the pane's configured model)
- Isolated context windows (they share the conversation context)

Builders are **skill bundles** — convenient shortcuts to activate a specific set of Skills at once. They are implemented as:
1. A preset list of Skill names in the Pane configuration (stored in `pane.metadata_json.skills.active_builder`)
2. OR a special "Builder" SKILL.md that references sub-skills via required dependencies

### Implementation

```yaml
# .builderboard/builders/builder-a.yaml
name: builder-a
display_name: "Builder A — Codebase Review"
description: "Full codebase analysis: architecture, security, quality, technical debt"
skills:
  - architecture-review
  - security-audit
  - code-quality-review
  - technical-debt-review
tools: [list_directory, read_file, search_files, find_files]
filesystem: required
project: required
```

At pane creation (or `/builder-a` command):
1. All referenced Skills are loaded
2. All referenced tools are registered
3. Project scope is resolved
4. Execution begins with all Skills active

Builder configurations live in `.builderboard/builders/<name>.yaml`. They are discovered and indexed the same way as Skills (filesystem + SQLite index).

---

## Section 9 — Marketplace Readiness

### Import/Export Format

Skills are distributed as `.zip` archives containing the Skill directory:

```
security-audit-1.2.0.zip
├── SKILL.md
├── patterns.yaml
├── scripts/
│   └── check_secrets.py
└── resources/
    └── api_schemas.md
```

### Import Flow

```
User runs: builderboard skill install ./security-audit-1.2.0.zip
1. Validate SKILL.md (parse frontmatter, validate fields)
2. Check version (is this newer than existing?)
3. Extract to ~/.config/builderboard/skills/security-audit/
4. Re-index (scan, parse, update SQLite)
5. Emit event: skill_installed(name, version)
```

### Export Flow

```
User runs: builderboard skill export security-audit
1. Locate SKILL.md in filesystem
2. Package directory into zip
3. Write to ./security-audit-1.2.0.zip
```

### Versioning

- **semver** (`MAJOR.MINOR.PATCH`)
- MAJOR bump = breaking changes to context declarations
- MINOR bump = new instructions, new resources
- PATCH bump = fixes, clarifications
- Version comparison: `builderboard skill upgrade` checks remote registry for newer version

### Sharing Model

| Scope | Location | Versioned | Shared With |
|---|---|---|---|
| Built-in | App bundle | App release | All users |
| Project | `.builderboard/skills/` | Git | Team (via repo) |
| User | `~/.config/builderboard/skills/` | Manual | No one |
| Marketplace | Remote registry | Registry | All users |

### Remote Registry (Architecture Only — Not Phase 9B)

```json
// https://registry.builderboard.app/v1/skills/index.json
{
  "schema_version": 1,
  "skills": [
    {
      "name": "security-audit",
      "display_name": "Security Audit",
      "description": "Review codebase for security vulnerabilities",
      "version": "1.2.0",
      "author": "BuilderBoard",
      "license": "MIT",
      "download_url": "https://registry.builderboard.app/v1/skills/security-audit/1.2.0.zip",
      "checksum_sha256": "abc123...",
      "min_app_version": "0.8.0",
      "context": { ... }
    }
  ]
}
```

### Architecture Decisions That Enable Marketplace Without Redesign

1. **Filesystem storage.** Skills are directories. Distribution is zipping/unzipping directories. No schema migration needed.
2. **Version field in frontmatter.** Enables semver-based updates from day one.
3. **No code execution at load time.** Skills are Markdown + scripts. No dynamic loading. No sandboxing issues at resolution time.
4. **Content-addressed paths.** Skill directories are named by Skill name. No path collisions between sources (project vs user vs builtin).
5. **YAML frontmatter is stable.** Adding marketplace fields (rating, downloads, author) is additive — existing skills continue to work.

---

## Section 10 — Events & Observability

### Event Types

| Event | Payload | Emitted When | Consumer |
|---|---|---|---|
| `skill_matched` | `{ skill_name, intent, confidence }` | Intent → Skill mapping succeeded | Frontend (show loading skill indicator) |
| `skill_loaded` | `{ skill_name, token_count }` | Skill body loaded from filesystem | Execution trace |
| `skill_load_failed` | `{ skill_name, error }` | Skill resolution failed | Frontend (show error) |
| `tool_registered` | `{ tool_name, skill_name }` | Tool added to context for a skill | Execution trace |
| `context_wired` | `{ project_name, filesystem_root, tools_count }` | All context resolved | Execution trace |
| `enrichment_started` | `{ intent, tool_calls }` | Filesystem enrichment begins | Frontend (existing event) |
| `enrichment_completed` | `{ tokens_used, results_count }` | Enrichment done | Execution trace |
| `execution_started` | `{ model, tokens_in }` | Model call begins | Perf metrics |
| `execution_completed` | `{ tokens_out, duration_ms }` | Model call ends | Perf metrics |
| `execution_failed` | `{ error, duration_ms }` | Model call failed | Error tracking |
| `subagent_spawned` | `{ subagent_name, skills, depth }` | Subagent created | Execution trace |
| `subagent_completed` | `{ subagent_name, tokens_used, result_size }` | Subagent returned | Execution trace |
| `skill_installed` | `{ skill_name, version }` | Skill installed/updated | Index maintenance |
| `skill_removed` | `{ skill_name }` | Skill deleted | Index maintenance |

### Frontend Events (Tauri)

The frontend already listens for `message_stream_enrichment_started`, `message_stream_chunk`, `message_stream_complete`, `message_stream_error`. Phase 9 adds:

```typescript
// New Tauri events
interface SkillMatchedEvent {
  skillName: string;
  intent: string;
}

interface SkillLoadedEvent {
  skillName: string;
  tokenCount: number;
}

interface SubagentSpawnedEvent {
  subagentName: string;
  depth: number;
}
```

### Execution Trace

Each execution produces a structured trace:

```json
{
  "execution_id": "uuid",
  "pane_id": "...",
  "user_message_id": "...",
  "timestamp": "2026-06-24T12:00:00Z",
  "events": [
    {"type": "intent_resolved", "intents": ["security_review"], "at": "..."},
    {"type": "skill_matched", "skill_name": "security-audit", "at": "..."},
    {"type": "skill_loaded", "skill_name": "security-audit", "token_count": 3200, "at": "..."},
    {"type": "enrichment_started", "tool_calls": ["search_files"], "at": "..."},
    {"type": "enrichment_completed", "results_count": 12, "at": "..."},
    {"type": "execution_started", "model": "gpt-5.5", "tokens_in": 28000, "at": "..."},
    {"type": "execution_completed", "tokens_out": 450, "duration_ms": 8400, "at": "..."}
  ],
  "duration_ms": 9200,
  "total_tokens": 28450,
  "skills_used": ["security-audit"],
  "result": "success"
}
```

Traces are:
- Emitted as Tauri events during execution
- Stored in a ring buffer (last 100) for the `/debug` panel
- Optionally persisted to a `execution_traces` table in Phase 9D

---

## Section 11 — Security Model

### Principle

**Skills declare desired access. The runtime enforces actual access.** A Skill cannot escalate its privileges beyond what the runtime allows.

### Permission Layers

```
Layer 1: Global Runtime Defaults (deny by default)
  - All tools: denied
  - All filesystem access: denied
  - All project access: denied
  
Layer 2: User Settings (user can allow)
  - ~/.config/builderboard/permissions.yaml
  
Layer 3: Project Settings (team can allow)
  - .builderboard/permissions.yaml
  
Layer 4: Skill Declarations (skill requests)
  - context.tools, context.filesystem, context.project
  
Layer 5: Session Permissions (per-turn approval)
  - User approved or denied at runtime
```

### Permission Resolution

For each resource (tool, filesystem, project):
1. If any layer denies → resource is unavailable
2. If any layer asks → user is prompted
3. If all layers allow (or are silent) → resource is available

### Skill Permission Request Flow

When a Skill declares `context.tools: [search_files]`:
1. Runtime checks if `search_files` is allowed by layers 1-3
2. If denied → Skill loads but tool is not registered. Skill body may indicate reduced functionality.
3. If ask → frontend shows permission dialog: "Security Audit Skill wants to use search_files. Allow for this session?"
4. If allowed → tool is registered in the model's tool descriptions

### Filesystem Containment

All filesystem operations go through `ApprovedScope` (existing Phase 8 infrastructure). Skills cannot:
- Escape the project root (traversal prevention)
- Access files outside the approved scope
- Follow symlinks outside the scope

### Script Execution

Skills can bundle scripts (`.py`, `.sh`, etc.). Script execution:
- Always sandboxed (via bubblewrap or macOS sandbox-exec)
- No network access for scripts (unless explicitly allowed)
- Timeout enforced (default: 30 seconds)
- Output only enters context (script source code never does)
- Scripts are inspected at Skill install time (checksum verified)

### Future: Permission Manifest (Phase 9D)

```yaml
# .builderboard/permissions.yaml
skills:
  security-audit:
    enabled: true
    tools: [list_directory, read_file, search_files, find_files]
    filesystem: true
    project: true
    scripts: false  # block script execution
    max_tokens: 4000
  
  unknown-skill:    # deny by default for untrusted
    enabled: false
```

---

## Section 12 — Future-Proofing

### Will Skill Spec v1 Survive Voice?

**Yes, with minor additions.** Voice input is just a different input modality. The Skill execution pipeline (intent → skill → context → model → response) is agnostic to whether the input was typed or spoken. Changes needed:
- `context.intents` may need audio-specific intents (e.g., `transcribe_audio`, `analyze_speech`)
- No structural changes to the SKILL.md format

### Will Skill Spec v1 Survive Attachments?

**Yes.** Attachments (images, files, blobs) are part of the provider request body, not the Skill system. Skills may declare `context.input_types: [text, image, file]` but this is additive to the existing spec. The SKILL.md format does not change.

### Will Skill Spec v1 Survive Subagents?

**Yes.** The `execution: subagent` field in the context block was designed specifically for this. Skills that need isolated execution declare it upfront. Subagent lifecycle (spawn/wait/close) is a runtime concern, not a format concern.

### Will Skill Spec v1 Survive Tool Chains?

**Yes.** Tool chaining is a consequence of the model's multi-turn reasoning within a single execution. The `context.tools` declaration gives the model the tools it needs. The model chains them naturally via its agentic loop. No format changes needed.

### Will Skill Spec v1 Survive Memory?

**Partially.** The spec has `context.memory: none` as a placeholder. When memory arrives:
- Skills will need `context.memory: read | write | readwrite`
- Skills will need to declare memory keys or namespaces
- The `metadata_json` in the skills table can absorb this without migration
- **Weakness identified:** The `context` block does not currently define memory schema (what keys, what TTL, what conflict resolution). This will require a `memory` sub-block in a later version.

### Will Skill Spec v1 Survive Marketplace?

**Yes.** The `version`, `author`, `license`, and `metadata` fields were designed for marketplace compatibility. The zip distribution format and remote registry JSON schema are forward-compatible. No format changes needed.

### Identified Weaknesses

| Weakness | Risk | Mitigation |
|---|---|---|
| No memory schema | Medium | `context.memory` is a placeholder; will need sub-block in v2 |
| No input type declaration | Low | Add `context.input_types: [text, image, file]` in v1.1 |
| No output format contract | Medium | Skills currently rely on model judgment for output format; structured output contracts may be needed |
| No permission scope for scripts | Low | Script execution is sandboxed but not declarable; add `context.scripts: allowed | denied` in v1.1 |
| No cross-skill state sharing | Low | Skills should not share state; this is by design |
| No skill testing framework | Medium | Spec doesn't define how to unit-test a Skill; needed before marketplace |
| No subagent resource limits | Low | `context.execution: subagent` doesn't specify max_turns, timeouts; defaults needed |

### Verdict

**Skill Spec v1 can survive all Phase 9 features with at most minor additive changes** (new context fields, new intent types, new execution modes). No structural redesign is required. The spec's weaknesses are in areas that will be addressed in v1.1 (memory schema) or are intentional design constraints (no cross-skill state).

---

## Section 13 — Reference Examples

### Example 1: security-audit.SKILL.md

```yaml
---
name: security-audit
description: Review codebase for security vulnerabilities, credential leaks, and authorization issues
version: 1.0.0
author: BuilderBoard
license: MIT
display_name: "Security Audit"
categories:
  - security
  - code-review
tags:
  - audit
  - vulnerability
  - secrets
  - auth
compatibility:
  - builderboard

context:
  intents:
    - security_review
    - code_quality_review
  tools:
    - list_directory
    - read_file
    - search_files
    - find_files
  filesystem: required
  project: required
  execution: conversation
  max_tokens: 8000
  optional_skills:
    - dependency-review
---

# Security Audit

## Overview

Analyze the codebase for security issues including credential leaks, authorization bypasses, input validation gaps, and insecure dependencies.

## Instructions

### Step 1: Identify Authentication & Authorization
- Search for auth-related patterns in the codebase
- Check for hardcoded API keys, tokens, or passwords (`search_files --pattern credential|api_key|password|secret|token`)
- Examine auth middleware and route guards

### Step 2: Check Input Validation
- Search for user input handling patterns
- Look for SQL injection vectors, command injection, path traversal
- Examine form validation and sanitization

### Step 3: Review Dependencies
- If dependency-review skill is available, run it
- Check for known vulnerable dependencies

### Step 4: Report Format
Return results as a structured markdown report with:
- `CRITICAL` issues (active credential leaks, auth bypasses)
- `HIGH` issues (missing validation, weak auth)
- `MEDIUM` issues (best practice violations)
- `LOW` issues (suggestions)

## Best Practices
- Do not output detected credentials in the report — reference them by file and line
- False positives are expected; flag them as LOW severity
- Prioritize CRITICAL findings that could lead to data breaches

## Examples

### Example: Checking for secrets
```
User: "Check if we have any hardcoded credentials"
Model: Runs search_files for credential patterns,
       reviews results,
       reports findings with severity
```

### Wikilinks
For deeper analysis, use [[dependency-review]] to check dependency vulnerabilities.
```

### Example 2: production-readiness.SKILL.md

```yaml
---
name: production-readiness
description: Full production readiness review — architecture, security, dependencies, and go-live checklist
version: 1.0.0
author: BuilderBoard
license: MIT
display_name: "Production Readiness Review"
categories:
  - deployment
  - operations
tags:
  - production
  - go-live
  - review
compatibility:
  - builderboard

context:
  intents:
    - production_readiness_review
  tools:
    - list_directory
    - read_file
    - search_files
    - find_files
  filesystem: required
  project: required
  execution: conversation
  max_tokens: 6000
  depends_on:
    - architecture-review
    - security-audit
---

# Production Readiness Review

## Pre-requisites

This workflow depends on:
- [[architecture-review]] — Must complete architecture review first
- [[security-audit]] — Must complete security audit second

Load these before executing this workflow.

## Overview

Verify that the codebase meets production readiness criteria across five dimensions.

## Ordered Steps

### Step 1: Verify Prerequisites
- Confirm [[architecture-review]] has been run
- Confirm [[security-audit]] has been run
- If either is missing, run them first

### Step 2: Error Handling Review
- Search for uncaught error patterns (`search_files --pattern catch|error|panic|unwrap`)
- Verify error boundaries exist at module boundaries
- Check for proper error logging

### Step 3: Configuration Review
- Check configuration files (`read_file --path .env.example`, `read_file --path config.yaml`)
- Verify secrets are not hardcoded
- Check environment-specific configuration

### Step 4: Performance Checklist
- Review bundle sizes for frontend projects
- Check for N+1 query patterns in backend code
- Verify caching headers or mechanisms exist

### Step 5: Go-Live Checklist
- [ ] Database migrations are idempotent
- [ ] Feature flags are in place for risky changes
- [ ] Monitoring and alerting are configured
- [ ] Rollback plan exists
- [ ] Load testing has been performed

## Output

Generate a production readiness scorecard:

```json
{
  "overall": "PASS" | "PASS_WITH_RISKS" | "FAIL",
  "sections": {
    "architecture": { "status": "PASS", "issues": [] },
    "security": { "status": "PASS_WITH_RISKS", "issues": ["..."], "critical": 0, "high": 2 },
    "error_handling": { "status": "PASS", "issues": [] },
    "configuration": { "status": "PASS", "issues": [] },
    "performance": { "status": "PASS_WITH_RISKS", "issues": ["..."] }
  },
  "go_live_checklist": {
    "passed": 5,
    "total": 6,
    "blocking": ["Load testing not performed"]
  },
  "recommendations": ["Run load testing before deployment"]
}
```

## Best Practices
- Do not block on non-critical issues — flag them as risks
- If prerequisites fail, report them as blocking
- Produce a machine-readable JSON report for CI/CD integration
```

### Example 3: builder-a.SKILL.md

```yaml
---
name: builder-a
description: Full codebase analysis — architecture, security, quality, and technical debt review
version: 1.0.0
author: BuilderBoard
license: MIT
display_name: "Builder A"
categories:
  - meta
  - workflow
tags:
  - full-review
  - codebase-analysis
compatibility:
  - builderboard

context:
  intents:
    - codebase_analysis
    - project_review
  tools:
    - list_directory
    - read_file
    - search_files
    - find_files
  filesystem: required
  project: required
  execution: conversation
  max_tokens: 2000
  depends_on:
    - architecture-review
    - security-audit
    - code-quality-review
    - technical-debt-review
---

# Builder A — Full Codebase Analysis

## Overview

Builder A is a meta-skill that activates four parallel analysis skills for a comprehensive codebase review.

## Instructions

This skill does not contain its own analysis logic. It orchestrates the following skills:

### Loaded Skills
- [[architecture-review]] — Module structure, dependency injection, code organization
- [[security-audit]] — Vulnerability scanning, credential detection, auth review
- [[code-quality-review]] — Code style, test coverage, documentation quality
- [[technical-debt-review]] — Outdated dependencies, deprecated APIs, migration needs

### Execution Flow

1. Load all four dependency skills (done by runtime)
2. Present the user with the analysis scope confirmation
3. Run each skill in sequence, collecting results
4. Compile findings into a unified report

## Output

```json
{
  "builder": "builder-a",
  "project": "<project-name>",
  "findings": {
    "architecture": { ... },
    "security": { ... },
    "code_quality": { ... },
    "technical_debt": { ... }
  },
  "summary": {
    "total_issues": 42,
    "critical": 2,
    "high": 8,
    "medium": 15,
    "low": 17,
    "recommendations": ["..."],
    "overall_health": "FAIR"
  }
}
```
```

### Example 4: technical-debt.SKILL.md

```yaml
---
name: technical-debt
description: Identify technical debt — outdated dependencies, deprecated APIs, migration needs, and code quality issues
version: 1.0.0
author: BuilderBoard
license: MIT
display_name: "Technical Debt Review"
categories:
  - code-quality
  - maintenance
tags:
  - tech-debt
  - refactoring
  - dependencies
compatibility:
  - builderboard

context:
  intents:
    - technical_debt_review
    - code_quality_review
  tools:
    - list_directory
    - read_file
    - search_files
    - find_files
  filesystem: required
  project: required
  execution: conversation
  max_tokens: 6000
---

# Technical Debt Review

## Overview

Identify areas of technical debt including outdated dependencies, deprecated API usage, overly complex code, and missing test coverage.

## Instructions

### Step 1: Dependency Analysis
- Read package files: `package.json`, `Cargo.toml`, `requirements.txt`, `Gemfile`
- Check for outdated major versions (v1 → v2, legacy frameworks)
- Flag pinned dependencies that should be updated

### Step 2: Deprecated API Detection
- Search for known deprecated patterns specific to the project's language/framework
- Search for TODO, FIXME, HACK, XXX comments (`search_files --pattern "TODO|FIXME|HACK|XXX"`)
- Large comment blocks on old code

### Step 3: Complexity Analysis
- Find files with excessive line counts (`find_files --pattern *.rs --min-size 500`)
- Find deeply nested code (search for excessive indentation)
- Identify duplicated code blocks (search_files for repeated patterns)

### Step 4: Test Gap Analysis
- Check test directory structure
- Report untested modules
- Check test coverage if coverage reports exist

### Step 5: Migration Assessment
- Identify any data migration or schema migration files
- Check for version compatibility issues
- Flag blocking migrations

## Output

```json
{
  "dependencies": {
    "outdated": [{"name": "left-pad", "current": "1.0.0", "latest": "2.0.0", "severity": "low"}],
    "deprecated": [],
    "blocking_migrations": []
  },
  "code_quality": {
    "todo_count": 23,
    "fixme_count": 5,
    "large_files": ["src/legacy/old_parser.rs:1200 lines"],
    "duplication_warnings": []
  },
  "test_gaps": {
    "untested_modules": ["src/auth/oauth_service.rs"],
    "coverage_percentage": null,
    "test_count": 187
  },
  "overall_assessment": "MODERATE_DEBT",
  "recommendations": [
    "Update left-pad to v2 (breaking changes in API)",
    "Address 5 FIXME comments in auth module",
    "Add tests for oauth_service.rs"
  ]
}
```
```

---

## Section 14 — Mistakes To Avoid (Top 20)

1. **Skills that modify their own SKILL.md.** Skills are instructions. They should not write to their own definition. If a Skill needs to remember something, it writes to the message history, not to the skill file.

2. **Skills with implicit dependencies.** If Skill A depends on Skill B being loaded first, declare it in `depends_on`. Do not rely on loading order or coincidence.

3. **Skill body too large.** If a SKILL.md exceeds `max_tokens` (default 4000), the model will lose context. Keep each Skill focused. Use wikilinks to split large knowledge across multiple Skills.

4. **Overloading the context block.** `context.tools` should list tools the Skill *actually uses*, not every tool in the system. Over-declaration bloats the system prompt with unused tool schemas.

5. **Skills as agent configurations.** Skills are not agents. Do not put system prompt instructions (persona, behavior constraints) in a Skill. Builder Personas (Section 8) handle agent configuration.

6. **Mutable skill state.** Skills should produce the same instructions every time they're loaded. If a Skill's behavior changes based on previous executions, that state belongs in messages or a future memory system.

7. **Hardcoding file paths in Skill instructions.** Skill instructions should reference files by role ("read the package file"), not by absolute path. The runtime provides scope resolution.

8. **Skills that require network access.** Skills should work offline. Network access (MCP servers, API calls) should be optional enhancements, not core requirements.

9. **Circular dependencies between Skills.** Skill A depends on B, B depends on A. The dependency resolver must detect and reject this. Skill authors should use wikilinks (navigable references) instead of circular dependencies.

10. **One giant "everything" Skill.** Resist the urge to create a single Skill that does everything. Break knowledge into focused Skills (architecture, security, dependencies) and compose them via workflows.

11. **Copying prompts into Skills.** Skills are not prompt libraries. Do not create Skills that are just system prompt fragments. A Skill should capture *expertise and process*, not just instruction text.

12. **Forgetting the context budget.** Skill metadata (~100 tokens each) adds up. With 100 skills, that's 10,000 tokens of system prompt just for descriptions. Enable/disable unused skills. Keep descriptions short.

13. **Skills as the only extensibility mechanism.** Skills cover "what to do". Hooks cover "when to do it". MCP covers "how to connect to external tools". All three are needed. Do not force everything into Skills.

14. **No skill testing before marketplace.** A Skill that works on one project may fail on another (different language, different structure). Build a skill testing framework before opening the marketplace.

15. **Skills that encourage bad practices.** A Skill should not tell the model to do dangerous things (delete files, modify production data). Review Skills for safety before installation.

16. **Storing secrets in Skill resources.** Bundled scripts and config files should not contain hardcoded credentials. Skills reference the project's own config/schema files at runtime.

17. **Ignoring the `execution` mode.** Every Skill defaults to `conversation` mode. Skills that do heavy analysis should use `subagent` mode to avoid polluting the main context. Authors must choose deliberately.

18. **Over-nesting subagents.** If Skill A spawns subagent B, which spawns subagent C, which spawns subagent D — the user waits for all levels to complete. Limit subagent depth (default: 3) and use subagents only for genuinely parallel work.

19. **Marketplace skills that break on update.** When a Skill updates, its instructions change. The new version may produce different results. Version pinning (`skill install security-audit@1.2.0`) protects against unexpected changes.

20. **Designing for the 1% case.** Start with the common case: single Skill loaded from intent match, instructions injected as system message, tools registered from context. Do not build a visual workflow editor, a skill graph debugger, or a marketplace recommendation engine before the basic flow works.

---

## Final Confidence Assessment

**If BuilderBoard implements this specification exactly, how confident are you that Skills can scale to hundreds of skills, marketplace distribution, subagents, voice, and attachments without a major redesign?**

### Confidence Score: 88/100

### Rationale

**What makes me confident (the score would be higher):**

1. **Progressive disclosure is built into the format.** The 3-level loading (metadata → instructions → resources) scales to hundreds of skills because only ~100 tokens per skill enters the startup context. The full body is loaded on demand. This is proven by Claude Code's production deployment with hundreds of skills.

2. **The context declaration system is forward-compatible.** Adding new fields to the `context` block (e.g., `context.input_types`, `context.memory`) is additive. Existing skills continue to work unchanged.

3. **The execution lifecycle separates concerns cleanly.** Intent resolution, skill loading, context wiring, and model execution are distinct phases. Adding voice (new input modality) or subagents (new execution mode) only changes one phase, not the whole pipeline.

4. **Filesystem storage is inherently scalable.** Skills are directories on disk. There is no database schema to maintain for skill content. The SQLite index is a cache, not a source of truth. Marketplace distribution is zipping/unzipping directories.

5. **The spec avoids the most expensive mistakes.** "Everything is an agent" (Mistake #1) is rejected by the clear Skill ≠ Agent distinction. DAG-based orchestration (Mistake #5) is avoided by the single-threaded agentic loop with lazy skill graph navigation.

**What makes me cautious (why not higher):**

1. **Memory is a placeholder.** The `context.memory: none` field exists but has no schema. When memory arrives (Phase 9D or 10), the context block will need a `memory` sub-block. This is a v1.1 addition, not a v1 redesign, but it's an unknown.

2. **The context block schema hasn't been battle-tested.** The `context.intents`, `context.tools`, `context.filesystem`, `context.project` fields seem right, but real skill authors may find gaps. The spec needs at least 10-20 real skills written against it before we can be confident the schema is complete.

3. **No structured output contracts.** Skills currently rely on the model's judgment for output format. For marketplace skills that need to produce machine-parseable results (CI/CD integration, automated pipelines), the spec needs output contract declarations. This is a v1.1 addition.

4. **The subagent execution model is specified but unimplemented.** The `execution: subagent` field defines intent, but the actual subagent runtime (isolation, lifecycle, communication) is complex and may reveal design issues during implementation.

5. **Permission model needs real-world testing.** The 5-layer permission system (globals → user → project → skill → session) is conceptually clean but may be too complex for typical users. A simplified "skill is trusted | ask me | deny me" mode may be needed for non-technical users.

### Verdict

**Implement Skill Spec v1 as specified.** The spec is stable enough to guide Phase 9B, 9C, and 9D without major redesign. Address the weaknesses (memory schema, output contracts) in v1.1 after gaining real-world experience with 10-20 authored skills. The core architecture — SKILL.md filesystem storage, progressive disclosure, context declarations, lazy loading, wikilink graph navigation, and the extended execution lifecycle — is proven by multiple production systems and will not require redesign.
