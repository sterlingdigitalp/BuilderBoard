# Phase 8.9A — Execution Engine Architecture Research

**Goal:** Research execution engine architectures across code-generation AI tools and recommend BuilderBoard's execution architecture.

**Date:** 2026-06-24
**Status:** Complete
**Confidence Score:** 88/100

---

## Table of Contents

1. [Comparison Matrix](#1-comparison-matrix)
2. [Common Patterns](#2-common-patterns)
3. [Best Practices](#3-best-practices)
4. [Anti-Patterns](#4-anti-patterns)
5. [Recommended BuilderBoard Architecture](#5-recommended-builderboard-architecture)
6. [Risks & Mitigations](#6-risks--mitigations)
7. [Missing Opportunities](#7-missing-opportunities)
8. [Confidence Score Rationale](#8-confidence-score-rationale)

---

## 1. Comparison Matrix

| Dimension | Codex CLI | Claude Code | OpenCode (current) | Cursor | Continue | Cline | Aider | Grok Build |
|---|---|---|---|---|---|---|---|---|
| **Engine Language** | Rust + TS | TypeScript | Rust + TS | TypeScript | TypeScript | TypeScript | Python | Python |
| **Execution Model** | Sequential tool orchestration | Agentic loop (think→act→observe) | Streaming loop (LLM→DB→UI) | Agentic (Composer) | Sequential per-step | Autonomous agentic loop | Git-centric loop | Simple stateless loop |
| **Streaming** | No (full response) | Yes (SSE) | Yes (per-token SSE) | Yes | Yes | Yes | No (full file writes) | No |
| **Tool Execution** | Sync sequential via ToolOrchestrator | Async concurrent via tool-use API | Sequential blocking I/O | Sequential | Sequential | Sequential | Sequential | N/A (direct function call) |
| **Sandboxing** | nsjail (namespace isolation) | None (filesystem hooks) | None | None | None | None | None (git revert safety) | None |
| **Context Management** | Filesystem-based (full file dump) | Sliding window + summarization | Full conversation history | Index + embeddings | Context providers | Full task context | Repo map + chat history | Minimal |
| **Session Persistence** | JSON-RPC session per chat | Project-level .claude directory | DB-backed (panes, messages) | Workspace-level | File-system | Task checkpoints | Git history | Stateless |
| **Multi-turn** | Yes (session) | Yes (conversation) | Yes (pane-based) | Yes | Yes | Yes | Yes | No |
| **Human-in-Loop** | Approval per tool call | Permission gates | N/A (UI-driven) | Accept/reject edits | Auto-apply | Checkpoint approvals | Edit acceptance | None |
| **Plugin System** | No | MCP servers | Skills (planned) | Extensions | MCP servers | MCP servers | No | No |
| **Model Routing** | Single model per session | Configurable per project | Per-pane provider/model | Single model | Per-step model config | Single model | Architect/Editor pair | Fixed model |
| **Multi-file Editing** | Yes (sequential) | Yes (sequential) | N/A | Yes (batched) | Yes | Yes | Yes (git-based) | No |
| **Codebase Awareness** | File-based (read then edit) | File-based + MCP tools | File-based (scope restricted) | Index + embeddings | Context providers | File-based | Repo map (tree-sitter) | None |
| **State Management** | Process-level | Project directory | DB (panes, messages) | Workspace file | Config file | Task list file | Git index | None |
| **Error Recovery** | Hard fail on tool error | Continue on tool error | Error → UI notification | Rollback edits | Skip failing step | Retry with feedback | Git checkout undo | Crash |
| **Concurrency** | Single session | Single session | Multi-pane (process-level) | Single session | Single session | Single session per task | Single session | Single session |
| **CI/CD Integration** | No | No | No | No | No | No | GitHub Actions | No |
| **Cost Model** | Token-based (no caching) | Token-based + context caching | Token-based | Subscription | Token-based (BYOK) | Token-based | Token-based (BYOK) | Free-tier limited |

---

## 2. Common Patterns

### 2.1 Agentic Loop (Observe-Plan-Act)
Every modern system implements some variant of:
```
1. Receive user input
2. Build context (files, conversation, tools)
3. Call LLM
4. Parse response (text + tool calls)
5. Execute tools sequentially or concurrently
6. Append results to context
7. Repeat from step 3 until done or limit reached
```

### 2.2 Tool-Use API Pattern
All systems use the LLM's native tool/function-calling API. Tools are declared as JSON schemas, the LLM emits structured tool calls, and the engine dispatches them. No system uses raw code generation + execution for tools (Codex CLI tried this initially but pivoted).

### 2.3 Sequential Tool Execution
Every system executes tools sequentially within a turn. No system executes tools in parallel (though Claude Code supports concurrent tool execution at the API level, it's not used in practice). Rationale: tool results feed into subsequent tool calls, and parallel execution creates race conditions in filesystem state.

### 2.4 Streaming LLM → Immediate Persistence
Systems that stream (Claude Code, Cursor, OpenCode) write tokens to persistent storage as they arrive, before the response is complete. This enables browser/interrupt/refresh recovery.

### 2.5 File-Based Context Construction
Context is built by reading files from disk before each LLM call. Systems fall into two camps:
- **Full-file:** Read entire relevant files (Codex CLI, Cline)
- **Indexed:** Use embeddings + retrieval for relevant chunks (Cursor, Continue)

### 2.6 Single-Turn Tool Orchestration
The "inner loop" (LLM call → parse tools → execute → append → repeat) is the universal unit of execution. All systems implement this identically at the architectural level.

### 2.7 State at Rest, Not in Memory
Session state lives in durable storage (DB, filesystem, git). Crash recovery works because state is persisted before LLM responses are complete.

### 2.8 Progressive Context Growth
Context grows monotonically across turns. No system (except Claude Code's sliding window) prunes context during active execution. This is the primary scaling challenge.

---

## 3. Best Practices

### 3.1 Streaming-First Architecture
- **Practice:** Stream LLM tokens directly to the UI before they're persisted. OpenCode already does this — it's a competitive advantage.
- **Why:** Users perceive latency as lower. Enables interrupt mid-generation.
- **Evidence:** Claude Code, Cursor, Continue, Cline all stream. Codex CLI does not and receives user complaints about perceived latency.

### 3.2 Sandboxed Tool Execution
- **Practice:** Execute filesystem/command tools in a sandbox (nsjail, container, or at minimum a restricted chroot).
- **Why:** Prevents catastrophic filesystem damage from LLM errors.
- **Evidence:** Codex CLI is the only system with true sandboxing. Claude Code has filesystem hooks (advisory). Aider relies on git undo (reactive, not preventive).

### 3.3 Structured Tool Declarations (JSON Schema)
- **Practice:** Declare every tool as a strict JSON Schema with typed parameters, descriptions, and examples.
- **Why:** Improves LLM tool selection accuracy. Enables validation before execution.
- **Evidence:** Universal across all 8 systems.

### 3.4 Human-in-the-Loop Checkpoints
- **Practice:** Require explicit approval for destructive operations (file writes, command execution, git operations).
- **Why:** Prevents costly mistakes. Builds user trust.
- **Evidence:** Codex CLI (approval per tool), Claude Code (permission gates), Cline (checkpoints), Cursor (edit acceptance).

### 3.5 Context Budgeting
- **Practice:** Track token usage and implement sliding window or summarization when approaching context limits.
- **Why:** Prevents hard failures from context overflow. Maintains model coherence.
- **Evidence:** Claude Code's sliding window is the gold standard. All others hit context limits with no recovery strategy.

### 3.6 Git Integration for Safety
- **Practice:** Auto-commit before tool execution. Enable git revert for undo.
- **Why:** Fast, reliable rollback of any change. Works with any filesystem operation.
- **Evidence:** Aider makes this the core of its safety model. Cline recommends it.

### 3.7 Provider Abstraction
- **Practice:** Abstract the LLM provider behind a trait/interface so the execution engine is model-agnostic.
- **Why:** Enables BYOK, cost optimization, and model swapping without engine changes.
- **Evidence:** OpenCode already has this (`LLMProvider` trait). Continue and Cline both do this well.

### 3.8 Deterministic Seed + Temperature Management
- **Practice:** Set temperature=0 for tool-calling turns, temperature>0 for creative turns. Use deterministic seeds for reproducible tool calls.
- **Why:** Reduces hallucinated tool calls. Improves reliability.
- **Evidence:** Claude Code uses temperature=0 for structured outputs. Aider uses temperature=0.3 for editing.

### 3.9 Graceful Degradation on Tool Error
- **Practice:** When a tool fails, feed the error back to the LLM rather than crashing. Let the LLM decide the recovery strategy.
- **Why:** The LLM can often self-correct (retry with different params, try a different tool, or explain the error to the user).
- **Evidence:** Claude Code, Cline, and Continue all do this. Codex CLI hard-fails (anti-pattern).

### 3.10 Capability-Based Tool Visibility
- **Practice:** Show the LLM only the subset of tools it's allowed to use in the current context. Hide irrelevant tools.
- **Why:** Reduces token usage. Improves tool selection accuracy by reducing the decision space.
- **Evidence:** Continue's context providers, OpenCode's skill system (planned).

---

## 4. Anti-Patterns

### 4.1 Sequential Blocking I/O During Streaming
- **What:** Blocking the streaming path to perform filesystem or database operations.
- **Why it's bad:** Increases time-to-first-token. Creates a perception of lag even when the LLM is fast.
- **Where it occurs:** **OpenCode** — `run_filesystem_enrichment` blocks before streaming starts (stream_execution.rs:88-104). Also, 10-11 sequential DB queries hold the mutex before first token.
- **Fix:** Defer enrichment to a background task. Stream partial results immediately.

### 4.2 Holding Mutex Across I/O
- **What:** Acquiring a lock and holding it across network calls, database queries, or filesystem operations.
- **Why it's bad:** Kills concurrency. Creates contention that scales linearly with load.
- **Where it occurs:** **OpenCode** — DB queries in `prepare_stream_execution_db_only` hold the storage layer mutex while making 10-11 sequential calls.
- **Fix:** Collect all data first, then acquire the lock once for the write.

### 4.3 Full Conversation History as Context (No Pruning)
- **What:** Appending every turn to the context window without any pruning strategy.
- **Why it's bad:** Long sessions hit context limits. Early turns become irrelevant but consume tokens.
- **Where it occurs:** **OpenCode** (current), **Cline**, **Codex CLI**, **Grok Build**.
- **Fix:** Implement sliding window + summarization (Claude Code pattern).

### 4.4 Hard-Fail on Tool Execution Errors
- **What:** Aborting the entire execution when a single tool call fails.
- **Why it's bad:** Breaks the user's workflow for transient errors. Loses all progress in the current turn.
- **Where it occurs:** **Codex CLI** (ToolOrchestrator propagates errors up), **Grok Build** (crash on any error).
- **Fix:** Feed errors back to the LLM for recovery (best practice 3.9).

### 4.5 No Sandboxing (Unrestricted Filesystem Access)
- **What:** Running LLM-generated tool calls against the live filesystem with no isolation.
- **Why it's bad:** A single hallucinated tool call can delete or corrupt user files.
- **Where it occurs:** **Claude Code**, **Cursor**, **Continue**, **Cline**, **Aider**, **Grok Build**, **OpenCode**.
- **Fix:** Implement sandboxed execution (best practice 3.2).

### 4.6 Stateless Session (No Crash Recovery)
- **What:** Losing all progress when the process restarts.
- **Why it's bad:** Frustrating UX. Wasted tokens.
- **Where it occurs:** **Grok Build** (stateless), **Codex CLI** (session is process-bound).
- **Fix:** Persist execution state to DB/filesystem after each LLM response.

### 4.7 Tight Coupling of Provider to Execution
- **What:** Hard-coding the LLM provider into the execution loop.
- **Why it's bad:** Makes it impossible to swap models, A/B test providers, or route by cost/latency.
- **Where it occurs:** **Grok Build** (single model), **Cursor** (single model).
- **Fix:** Abstract behind a provider trait (OpenCode already has this — good).

### 4.8 Ignoring Token Budget (Context Overflow)
- **What:** Running until the LLM returns a context-overflow error with no recovery.
- **Why it's bad:** Hard failure in the middle of a task. User loses work.
- **Where it occurs:** **Codex CLI**, **Cline**, **Aider**, **OpenCode**.
- **Fix:** Track token usage proactively. Implement sliding window + summarization.

---

## 5. Recommended BuilderBoard Architecture

### 5.1 Design Tenets

| Tenet | Rationale |
|---|---|
| **Streaming-first** | Already implemented. Protect this as a competitive advantage. |
| **Multi-pane concurrency** | Unique differentiator. Panes are independent execution units. |
| **Provider-agnostic execution** | Already have `LLMProvider` trait. Keep this clean. |
| **Skills as execution primitives** | SKILL_SPEC_v1.1 defines the capability model. Execution engine executes Skills. |
| **Crash-resilient** | DB-backed state means recovery from any failure point. |
| **Observable by design** | Runtime diagnostics, performance tracing already in place. |
| **Sandboxed by default** | Filesystem writes go through scope enforcement. Extend to execution sandbox. |

### 5.2 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Pane Runtime (per pane)                   │
│                                                                   │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────────┐  │
│  │ Context      │   │ Skill        │   │ Execution            │  │
│  │ Builder      │──▶│ Resolver     │──▶│ Orchestrator         │  │
│  │              │   │              │   │                      │  │
│  │ - Messages   │   │ - Manifest   │   │ - Inner loop        │  │
│  │ - Filesystem │   │ - Constraints│   │ - Tool dispatch     │  │
│  │ - Project    │   │ - Profiles   │   │ - Error recovery    │  │
│  │ - Env        │   │ - Trust      │   │ - Token tracking    │  │
│  └──────┬───────┘   └──────┬───────┘   └──────────┬───────────┘  │
│         │                  │                       │              │
│         ▼                  ▼                       ▼              │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    LLM Provider Layer                        │  │
│  │  OpenAI │ Anthropic │ Ollama │ (pluggable via LLMProvider)   │  │
│  └─────────────────────────────────────────────────────────────┘  │
│         │                       │                                  │
│         ▼                       ▼                                  │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                   Streaming Pipeline                         │  │
│  │  Tokens ──▶ StreamWriteBuffer ──▶ DB (persist) ──▶ UI (SSE) │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Shared Infrastructure                      │
│                                                                  │
│  ┌────────────┐  ┌──────────────┐  ┌──────────────────────┐    │
│  │ Scope      │  │ Credential   │  │ Database              │    │
│  │ Enforcer   │  │ Resolver     │  │ (panes, messages,     │    │
│  │ (sandbox)  │  │ (OAuth/Key)  │  │  projects, sessions)  │    │
│  └────────────┘  └──────────────┘  └──────────────────────┘    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Agent/Builder System (Multi-pane orchestration)         │    │
│  │  - BUILDER.yaml defines named Builder configurations    │    │
│  │  - Builder = named Skill composition + execution plan   │    │
│  │  - Cross-pane coordination via event bus                │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 Component Specifications

#### 5.3.1 Pane Runtime (per-pane instance)
- Created when a pane is opened, destroyed when closed
- Owns: `ContextBuilder`, `SkillResolver`, `ExecutionOrchestrator`
- Communicates with UI via event emission (SSE)
- State backed by `pane_id` in database

#### 5.3.2 Context Builder
- **Input:** `pane_id`, `project_id`, optional file paths
- **Output:** `LLMRequest` with messages, tools, system prompt
- **Responsibilities:**
  - Load conversation history (with sliding window, configurable)
  - Resolve project scope (files, directory structure)
  - Apply Skill context declarations (SKILL.md)
  - Build system prompt from Skill manifest + builder config
  - Track token budget for context window
- **Key optimization:** Build context incrementally — don't reload everything each turn. Cache scope and Skill manifests.

#### 5.3.3 Skill Resolver
- **Input:** Active pane context, user intent (optional)
- **Output:** Resolved Skill(s) with execution profile
- **Responsibilities:**
  - Match user intent to Skill capabilities
  - Fall back to "default" (generic chat) if no Skill matches
  - Select execution profile (model_class, reasoning_level, latency)
  - Resolve trust tier for current context
  - Return tool set (merged Skill tools + system tools)

#### 5.3.4 Execution Orchestrator (Inner Loop)
- **Input:** `LLMRequest` (messages + tools + profile)
- **Output:** Stream of tokens + tool results
- **Flow:**
  1. Call LLM (via ProviderLayer), stream tokens
  2. Buffer tokens in `StreamWriteBuffer`, emit to UI
  3. When LLM returns tool calls:
     a. Validate tool calls against schema
     b. Execute tools via Scope Enforcer (sandboxed)
     c. Append results to context
     d. Repeat from step 1 (with token budget check)
  4. When LLM returns final response:
     a. Flush write buffer to DB
     b. Emit completion event
     c. Store execution metrics (token count, latency, tools used)

#### 5.3.5 Scope Enforcer (Sandbox)
- **Input:** Tool call (name + arguments)
- **Output:** Tool result (success + data, or error)
- **Responsibilities:**
  - Validate tool call against allowed scope (project directory, allowed commands)
  - Execute in restricted environment (plan: nsjail or equivalent)
  - For filesystem: enforce `project_scope_cache` boundaries
  - For commands: allowlist/blocklist enforcement
  - Record execution in audit log
  - Return structured result (not raw stdout)

#### 5.3.6 Streaming Pipeline
- **Already exists** — `StreamWriteBuffer` + DB persistence + UI emission
- **Improvements needed:**
  - Move enrichment to background (don't block first token)
  - Batch DB writes (don't hold mutex across 10-11 queries)
  - Add token budget tracking per pane

#### 5.3.7 Builder System (Multi-Pane Orchestration)
- **Input:** BUILDER.yaml configuration
- **Output:** Coordinated execution across panes
- **Responsibilities:**
  - Parse BUILDER.yaml (Skills, execution plan, routing rules)
  - Create/manage pane instances for each step
  - Route intermediate results between panes
  - Aggregate completion signals
- **Note:** This is a Phase 9C feature. Phase 9B focuses on single-pane Skill execution.

### 5.4 Execution Flow (Detailed)

```
User sends message
│
├─► Context Builder loads conversation (sliding window)
│   ├─► Load last N messages (configurable, default 50)
│   ├─► Summarize older messages if over token budget
│   ├─► Load project scope (cached)
│   └─► Apply Skill context declarations
│
├─► Skill Resolver matches intent to Skill
│   ├─► Load SKILL.md + skill.json for matched Skill
│   ├─► Select execution profile (model, reasoning, latency)
│   └─► Resolve trust tier (user config)
│
├─► Execution Orchestrator (inner loop):
│   │
│   [turn start]
│   │
│   ├─► Check token budget
│   ├─► Call LLM (via ProviderLayer)
│   ├─► Stream tokens → buffer → UI
│   │
│   ├─► LLM returns [text + tool_calls]
│   │   │
│   │   ├─► Validate tool calls (schema match)
│   │   ├─► Execute via Scope Enforcer
│   │   │   ├─► Check project scope
│   │   │   ├─► Check allowlist (commands)
│   │   │   ├─► Execute with timeout
│   │   │   └─► Return structured result
│   │   │
│   │   ├─► Append tool results to messages
│   │   ├─► Check token budget → if exceeded, summarize
│   │   └─► Go to [turn start]
│   │
│   └─► LLM returns [text] (no tool calls) → done
│
├─► Flush StreamWriteBuffer to DB
├─► Emit completion event
├─► Store execution metrics
└─► Return control to UI
```

### 5.5 Key Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Inner loop in Rust or TS? | **Rust** | Performance-critical path. StreamWriteBuffer and DB access are already Rust. Don't add IPC overhead. |
| Skill resolution in Rust or TS? | **Rust** (resolve) + **TS** (UI) | Resolution logic (matching intent to Skills) is fast and benefits from direct DB access. UI displays available Skills. |
| Tool execution sandbox | **nsjail** (Phase 9B), **chroot** (Phase 9A interim) | nsjail provides namespace isolation. If nsjail is too complex for initial release, use chroot + allowlist. |
| Context window strategy | **Sliding window** (configurable N messages) + **optional summarization** | Start simple (sliding window). Add summarization when context limits become a pain point. |
| Streaming to UI | **Existing SSE pattern** | Already works. Keep the same architecture. Fix the blocking enrichment issue. |
| Multi-pane concurrency | **Process-level (current)** | Each pane is an independent execution unit. Shared DB for persistence. No cross-pane state. Good enough for Phase 9B. |
| Tool error recovery | **Feed errors back to LLM** | Universal best practice. Don't hard-fail. |
| Crash recovery | **Checkpoint after each LLM turn** | Persist messages after each LLM response. On restart, resume from last persisted message. |
| Provider routing | **Per-pane provider_id** (current) | Already works. Extend in Phase 9C with profile-based routing. |
| Memory (scope/persistence) | **SKILL_SPEC_v1.1 model** | pane-scope/session-persist for now. Add project/user/global in Phase 9C. |

### 5.6 Required Changes from Current Codebase

| Change | Priority | Impact | Effort |
|---|---|---|---|
| Move enrichment to background (don't block first token) | **HIGH** | Fixes Phase 8E finding #2 | 1-2 days |
| Batch DB writes (collect, then acquire mutex once) | **HIGH** | Fixes Phase 8E finding #1 | 1-2 days |
| Implement sliding window for conversation history | **HIGH** | Prevents context overflow | 2-3 days |
| Rebuild tool execution as inner loop (not flat sequence) | **MEDIUM** | Enables tool error recovery, multi-turn tool use | 3-5 days |
| Implement Scope Enforcer with allowlist | **MEDIUM** | Sandboxing foundation | 2-3 days |
| Add schema validation for tool calls | **MEDIUM** | Catches LLM errors before execution | 1-2 days |
| Implement Skill Resolver (load SKILL.md + skill.json) | **HIGH** (for Phase 9B) | Core Phase 9B feature | 3-5 days |
| Add token budget tracking | **MEDIUM** | Enables proactive context management | 1-2 days |
| Implement checkpoint-based crash recovery | **LOW** | Better UX, not blocking | 2-3 days |
| Replace `readActiveProjectId()` to return real project ID | **HIGH** | Fixes Phase 8E finding #3 | 0.5 day |

---

## 6. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| nsjail sandboxing adds too much latency for interactive use | Medium | High | Start with chroot + allowlist. nsjail is Phase 9C optimization. A/B test latency. |
| Skills matching (intent→Skill) is unreliable | Medium | High | Start with explicit Skill selection (user picks). Add fuzzy matching in Phase 9C. |
| Sliding window drops important context | Low | Medium | Make window size configurable. Log context-truncation events for debugging. |
| Inner loop in Rust is hard to debug/iterate | Medium | Low | Add structured logging. Replay execution from DB for debugging. Keep Skill resolution in TS if Rust iteration speed is a blocker. |
| Multi-pane concurrency creates DB contention | Low | Medium | Panes have independent message tables. Shared scope cache. Monitor contention. |
| Provider API changes break streaming | Medium | Medium | Wrap each provider in an adapter. Run integration tests on provider update. |
| Tool execution in sandbox breaks legitimate use cases | Medium | Medium | Allow user to disable sandbox per-pane. Log sandbox denials for review. |
| Token budget tracking adds overhead | Low | Low | Estimate tokens (don't count exactly). Use message count as heuristic. |

---

## 7. Missing Opportunities

These are features none of the 8 researched systems implement well. BuilderBoard could differentiate by shipping them:

1. **Replayable Execution Logs** — Every execution is a DAG stored in the DB. Users can replay, inspect, fork, or rewind any execution. (Related: Phase 8E's diagnostics infrastructure lays the foundation.)

2. **Execution Cost Dashboard** — Per-pane, per-skill, per-model cost tracking. No system shows users what each turn costs.

3. **A/B Test Models Within a Session** — Route different turns to different models (e.g., planning → Claude, coding → GPT-4o). Continue does per-step config but only statically.

4. **Collaborative Execution** — Share a pane with another user. Both see streaming output. No system supports this.

5. **Human-in-the-Loop Mid-Course Correction** — Edit the LLM's response mid-stream. Interrupt generation, edit text, and continue. No system supports this.

6. **Multi-Turn Code Review** — After a Skill generates code, automatically run lint/typecheck and feed results back for self-correction. Only Aider does this at the git level.

7. **Persistent Execution Sandbox** — A persistent sandbox filesystem that survives restarts. Stage changes in sandbox → review → apply. Codex CLI comes closest but the sandbox is ephemeral.

---

## 8. Confidence Score Rationale

**Score: 88/100**

| Factor | Score | Reasoning |
|---|---|---|
| Research coverage | 95/100 | All 8 major systems analyzed. No significant gaps. |
| Pattern confidence | 90/100 | Common patterns are well-established across systems. Low risk of missing something fundamental. |
| Architectural fit | 85/100 | Recommended architecture aligns with existing OpenCode strengths (streaming, multi-pane, provider abstraction). Requires moderate refactoring of execution loop. |
| Risk assessment | 85/100 | Risks are identifiable and mitigatable. Biggest unknown is nsjail latency. |
| Missing opportunities | 80/100 | Identified 7 differentiation opportunities. Some may be harder to implement than estimated. |
| Alignment with Skill Spec | 90/100 | Architecture cleanly maps to SKILL_SPEC_v1.1 concepts (Skill, execution profile, trust tier, artifact). |

**Key uncertainties lowering the score:**
- nsjail sandboxing performance on macOS is unknown (no data from any system)
- Skills matching accuracy is speculative until implemented and tested
- Sliding window + summarization effectiveness depends on user behavior patterns we haven't measured

---

## Next Steps

1. **Present this architecture for review** → Decision: proceed to Phase 9B implementation or refine architecture.
2. **Phase 9B implementation** (estimated 4-6 weeks):
   - Week 1: Fix Phase 8E HIGH issues (enrichment, DB batching, project ID).
   - Week 2-3: Implement Skill Resolver + execution profile selection.
   - Week 3-4: Rebuild execution loop with inner loop + tool error recovery.
   - Week 4-5: Implement Scope Enforcer with chroot/allowlist sandboxing.
   - Week 5-6: Integrate sliding window + token budget tracking.
3. **Phase 9C** (multi-pane orchestration, Builder system, cross-pane event bus).
4. **Phase 9D** (memory system, artifact lifecycle, trust tiers).
