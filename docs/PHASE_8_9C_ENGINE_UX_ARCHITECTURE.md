# Phase 8.9C — Execution Engine UX Architecture Research

**Role:** Research Engineer
**Goal:** Research how modern AI coding systems expose execution engines to users — what builds trust, what creates confusion.
**Date:** 2026-06-24
**Status:** Complete
**Confidence Score:** 90/100

---

## Table of Contents

1. [Comparison Matrix](#1-comparison-matrix)
2. [UX Patterns](#2-ux-patterns)
3. [Anti-Patterns](#3-anti-patterns)
4. [Best Practices](#4-best-practices)
5. [Recommended BuilderBoard Engine UX](#5-recommended-builderboard-engine-ux)
6. [Missing Opportunities](#6-missing-opportunities)
7. [Confidence Score Rationale](#7-confidence-score-rationale)

---

## 1. Comparison Matrix

### 1.1 Engine Selection

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Engine selection** | None (single engine) | None (single engine) | Chat vs Composer vs Agent (3 modes) | Architect vs code mode | Plan vs code mode (Shift+Tab) | Chat vs Plan vs Agent (3 modes) | Plan vs Act (Tab toggle) | None (single mode) |
| **Engine UI** | N/A | N/A | Dropdown at top of chat panel | `/architect` slash command | `Shift+Tab` cycle in TUI | Dropdown next to model | Toggle in header + CLI Tab | N/A |
| **Mode indicators** | Sandbox mode in status | Permission mode in footer | "Agent" / "Plan" badges | Prompt prefix (`>` vs `architect>`) | Status bar shows plan/code | Mode shown in input area | Status bar + toggle | Display state text |
| **Mode switching UX** | `/permissions` | `Shift+Tab` | Click dropdown | `/architect` + `/code` | `Shift+Tab` cycle | `Cmd/Ctrl + .` | `Tab` key | N/A |

### 1.2 Model Selection

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Selection UI** | `/model` picker + `--model` flag | `/model` picker (arrow keys adjust effort) | Dropdown in chat panel | `--model` flag + `/model` slash | `/model` slash + `-m` flag | Dropdown in panel header | Dropdown in settings view | `<select>` dropdown (4 options) |
| **Model aliases** | Model names from catalog | `best`, `opus`, `sonnet`, `haiku`, `fable` | Provider + model name | Any provider/model string | Model names from config | Model title from config | Model ID strings | Direct model IDs |
| **In-session switching** | `/model` | `/model` + arrow key effort | Dropdown (hot-switch) | `/model` | `/model` | Dropdown | Slash command in CLI | Dropdown (in header) |
| **Model info in UI** | Status line shows model | Footer shows model + effort | Shown in chat header | Startup banner shows model | Status bar shows model | Model list with badges | Status bar + settings | Yes (in dropdown) |
| **Separate plan/edit models** | No | No | No | `--editor-model` separate | No | Role-based routing | Plan/Act separate toggle | No |

### 1.3 Effort Controls

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Effort control** | Config-only (`model_reasoning_effort`) | `/effort` slider + arrow keys in `/model` | Hidden "Edit" button on model in picker | `/reasoning-effort` + `--thinking-tokens` | None exposed | Config-only (`defaultCompletionOptions`) | `--thinking` flag (none/low/medium/high/xhigh) | Reasoning level dropdown |
| **Effort levels** | minimal/low/medium/high/xhigh | low/medium/high/xhigh/max | Low/Medium/High/Extra High | low/medium/high | N/A | Via config | none/low/medium/high/xhigh | Low/Medium/High/XHigh |
| **Effort discoverability** | Poor (config key) | Good (/effort, arrow keys) | Poor (hidden "Edit" button) | Good (/reasoning-effort) | N/A | Poor (config key) | Good (--thinking flag) | Fair (visible dropdown) |
| **Compact/compress** | `/compact` slash | `/compact` + auto-compaction | N/A | History summarization | `/compact` slash | Auto-compaction (system msg) | `--compaction` flag | N/A |
| **Token budget display** | Status line token counter | Status line context % bar | No visible meter | `/tokens` command | Context % bar in footer | Context bar in input | Context tokens in status | N/A |

### 1.4 Capability Discovery

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Primary discovery** | `/` popup (all slash commands) | `/` menu + `?` key for shortcuts | `Cmd+Shift+P` command palette + `@` menu | `/help` + slash commands list | `/` commands + `grok inspect` | `@` mentions + `/` slash commands + onboarding cards | VS Code walkthrough + slash commands | None |
| **Onboarding** | First-run auth + `/init` for AGENTS.md | `/init` wizard | First-launch setting import | Startup banner with hints | `grok inspect` + docs | Onboarding card in empty chat | VS Code walkthrough | None |
| **Feature list** | `codex features list` | `/statusline` (discoverable build) | Community-driven discovery | 175+ flags in `--help` | `grok inspect` | `@` menu in chat | Settings panel + CLI `--help` | N/A |
| **Skill/tool discovery** | `/skills` + `/plugins` | `/` includes skill commands | `.cursor/rules/` discovery | `/models` for model search | Extensions modal (skills/plugins/hooks) | `/` slash + MCP tools | MCP Marketplace + "add a tool" | N/A |

### 1.5 Runtime Status

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Streaming display** | Syntax-highlighted TUI | Streaming text in terminal | Streaming in chat panel | Token-by-token streaming | Streaming text in TUI | Token-by-token in panel | Streaming in panel + TUI | Token-by-token (exists) |
| **Status bar** | Configurable footer (model, context, git, tokens, rate limits) | Custom `/statusline` script (cost, context %, rate limits, git) | No persistent status bar | Prompt prefix + startup banner | Context %, mode, shortcuts | Mode + model in input bar | Model, tokens, cost, workspace, git | None |
| **Spinner/indicator** | Terminal title spinner | Animated icon + effort text | Status spinner in chat | Streaming text (no spinner) | Thinking spinner `◆ Thought for 3.2s` | "Generating..." badge | Running indicator in panel | "Streaming..." text only |
| **Tool call display** | Inline in transcript | Expandable in transcript | Expandable cards | N/A (diff only) | Inline in TUI | Cards with status | Cards with status | **None** |
| **Phase indicators** | Turn queueing (Tab/Enter) | Task list (Ctrl+T) | "Planning next moves" | Per-edit diff | Subagent spinners + phase text | Per-tool status | Per-tool status | Display state in header |

### 1.6 Permission Visibility

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Permission mode** | Auto / Read-only / Full Access | default / acceptEdits / plan / auto / bypassPermissions | Auto-apply (default) vs Inline diffs | Trust-but-verify (auto-commit) | ask (default) / always-approve / plan | ask / auto / readonly | always-approve / auto / YOLO | **None** |
| **Approval UI** | Inline TUI prompts | Interactive permission dialog | "Run"/"Skip"/"Always Allow" on terminal cmds | Diff shown after auto-commit | TUI approval prompts | Modal with Continue/Cancel | Modal + inline approve/reject | N/A |
| **Diff preview** | In transcript | In transcript viewer (Ctrl+O) | Inline green/red in editor | Colored diff after commit | Colored diff per change | Inline diff in editor | Native VS Code diff editor | N/A |
| **Per-operation approval** | Yes (per tool call) | Yes (per tool call) | Per-file (inline diffs mode) | No (auto-commit) | Yes (per tool call) | Yes (per tool call) | Yes (per tool call) | N/A |
| **Protected paths** | Sandbox policy | `.git`, `.claude`, etc. | N/A | N/A | `~/.ssh`, `~/.aws`, etc. | N/A | N/A | Project scope enforced |

### 1.7 Execution Diagnostics

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Token usage** | Status line token counter | Status line cost + context % | Usage in Settings panel (async) | `/tokens` command | `/usage` + `/tokens` | Token display per turn | Status bar token/cost | **None** |
| **Cost display** | `/usage` command | Status line `$cost` | Per-plan credits, not per-turn | Not in default UI | `/cost` command | Per-turn cost tracking (hidden) | Status bar estimated cost | **None** |
| **Timing** | Turn timing in transcript | Via status line | No | No | Turn timestamps (inline) | Via Continue Console | Verbose mode | **None** |
| **Tool call traces** | Full transcript | Ctrl+O transcript viewer | Tool call cards (expandable) | Verbose mode (`-v`) | Inline in TUI | Continue Console | Verbose mode + tool cards | **None** |
| **Debug commands** | `codex doctor`, `debug-config`, `debug models` | `--debug` flag + `claude doctor` | Network diagnostics | `/report` + `/help` | `grok inspect` | Continue Console | `cline doctor` | Runtime probes (console only) |

### 1.8 Cancellation

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Primary cancel** | Ctrl+C | Ctrl+C | Stop button + Esc | Ctrl+C (partial preserved) | Ctrl+C | Stop button (panels) + Enter (CLI pause) | Ctrl+C + stop button | **None** |
| **Double-tap exit** | No | 2nd Ctrl+C exits | No | 2nd Ctrl+C exits (2s window) | No | No | 2nd Ctrl+C force exit | N/A |
| **Background** | No | Ctrl+B backgrounds task | N/A | No | Ctrl+X in agent detail | N/A | Zen mode (`--zen`) | N/A |
| **Graceful interruption** | Tab queues next turn | Esc stops response, keeps work | Stop preserves partial | Ctrl+C preserves partial response | Esc clears or interrupts | Enter pauses (CLI) | Ctrl+C aborts turn | N/A |
| **Force exit** | /exit + /quit | Ctrl+D | Close window | Ctrl+C (2nd) | /quit | N/A | Ctrl+C (2nd) | N/A |

### 1.9 Engine Health

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Connection status** | `/status` shows remote info | Status line + `/status` | Hidden (errors only) | Startup validation | Auth via `grok login` | Status bar error state | Status bar (model name) | **None** |
| **Rate limits** | `/usage` shows resets | Status line rate limit % | Error messages only | Error messages only | Not user-visible | Error messages only | API error cards | **None** |
| **Model availability** | Auto-filtered model catalog | Fallback chain + `/model` | Auto > degraded routing | `--check-model-accepts-settings` | `/model` to switch | Disabled in dropdown | Static dropdown | **None** |
| **Update checks** | `codex update` + auto-check | Auto-update + `claude doctor` | Auto-update (VS Code) | `--check-update` | Auto-check (`--no-auto-update`) | Auto-update (VS Code) | `--version` check | **None** |
| **Health summary** | `codex doctor` (comprehensive) | `claude doctor` | Network diagnostics | `/report` opens GitHub issue | `grok inspect` | Continue Console | `cline doctor` | Runtime probes |

### 1.10 User Trust Mechanisms

| Dimension | Codex CLI | Claude Code | Cursor | Aider | Grok Build | Continue | Cline | OpenCode (current) |
|---|---|---|---|---|---|---|---|---|
| **Primary trust** | Full transparency transcript | Permission gates + hooks | Diff preview + accept/reject | Git auto-commit + undo | Plan mode + clean diffs | Context items visibility | Checkpoints + shadow git | **None** |
| **What user sees before** | Plan explained + diff | Permission prompt | Diff in editor | Explanation in chat | Plan with files + risks | Tool args preview | Command preview + diff | N/A |
| **Rollback mechanism** | Git + `/diff` | `/rewind` (checkpoints) | Undo + git | `/undo` (git reset) | Git + `/rewind` | Chat history | Shadow git checkpoints | N/A |
| **Safety labeling** | `--yolo` marked "EXTREMELY DANGEROUS" | `--dangerously-skip-permissions` guard | N/A | `--yes-always` (no danger label) | Beta labeling | N/A | `YOLO` badge in status | N/A |
| **Open source** | Yes (Apache 2.0) | No | No | Yes (Apache 2.0) | No | Yes (Apache 2.0) | Yes (Apache 2.0) | Yes (MIT) |

---

## 2. UX Patterns

### 2.1 The Model Selector (Universal Pattern)

Every system has a model selector, and they follow a nearly identical pattern:

```
┌─────────────────────────────────────┐
│  Provider ▼     │  Model ▼          │
│  Anthropic      │  Claude Sonnet 4.6│
│  OpenAI         │  Claude Opus 4.8  │
│  Google         │  GPT-5.5          │
│  Local/Ollama   │  Grok Build 0.1   │
└─────────────────────────────────────┘
```

**Key UX findings:**
- **Dropdown at top of panel** is universal (Cursor, Continue, Cline, Grok Build, OpenCode)
- **Ability to switch mid-session** is table stakes — all systems support it
- **Model aliases** (Claude Code's `best`/`sonnet`/`opus`) reduce cognitive load vs. raw model IDs
- **Fast toggle** (Cursor's Composer 2 Fast) is a growing pattern for speed/cost tradeoff
- **Separate plan/edit models** (Aider's `--editor-model`, Cline's Plan/Act split) is an emerging pattern for cost optimization

### 2.2 Mode Selector (Three Archetypes)

The mode selector appears in three forms across systems:

**Cyclic toggle** (Claude Code, Cline, Grok Build):
- `Shift+Tab` or `Tab` to cycle modes
- Typically: plan → act → (back to plan)
- Pros: Keyboard-driven, fast for power users
- Cons: No visual of "what other modes exist"

**Dropdown selector** (Cursor, Continue):
- Dropdown next to model selector
- Options: Chat / Plan / Agent (or similar)
- Pros: Explicit, discoverable
- Cons: Mouse-dependent, clutters header

**Slash command activation** (Aider, Codex CLI):
- `/architect`, `/permissions`, `/plan`
- Pros: Doesn't consume UI space
- Cons: Must know the command exists

### 2.3 Streaming Display (Three Approaches)

**Basic streaming** (OpenCode, Aider, Grok Build):
- Tokens appear as they arrive
- No tool call rendering
- No phase indicators
- Simplest, least informative

**Rich streaming** (Cursor, Claude Code, Codex CLI, Continue):
- Tokens stream in chat panel
- Tool calls appear as expandable cards with status indicators
- Diffs render inline during generation
- Phase indicators show "planning", "searching", "writing"

**TUI streaming** (Codex CLI, Grok Build, Cline CLI):
- Full-screen terminal with syntax highlighting
- Configurable status bar
- Mouse support for clickable elements
- Diffs with tree-sitter highlighting

### 2.4 Permission Flow (Universal Journey)

```
User sends prompt → Model generates plan
  ↓
Model proposes action (tool call)
  ↓
[Permission gate]
  ├─ Auto-approved (policy match) → Execute
  ├─ Ask → Show dialog with:
  │   ├─ What tool (name + arguments)
  │   ├─ What will change (file diff / command preview)
  │   └─ Approve / Reject / Always Allow
  └─ Deny → Block with explanation
  ↓
Execute → Show result → Continue loop
```

**Key insight:** Every system shows the SAME three things in a permission dialog:
1. **Tool name** (e.g., "Bash", "Edit", "Read")
2. **Input arguments** (the command, file content, search pattern)
3. **Preview of effect** (diff for edits, command for terminal)

### 2.5 Context Visibility Pattern

Systems show context usage in one of three ways:

**Status bar percentage** (Claude Code, Grok Build, Cline):
- `▓▓▓▓░░░░░ 60%` — thin progress bar
- Always visible
- Shows remaining room in context window

**Input area indicator** (Continue):
- Small vertical bar in chat input
- Height = percentage used
- Hover for details

**Manual check** (Aider, Codex CLI):
- `/tokens` or `/status` command
- Not visible by default
- Requires user to ask

### 2.6 Effort Control Pattern

Effort is surfaced through a 3-tier model:

| Tier | Example | UX Pattern |
|---|---|---|
| **Fast / Cheap** | Cursor Fast toggle, Claude Haiku, Cline `low` | Toggle or dropdown option |
| **Balanced** | Default for most systems | Not explicitly shown |
| **Deep / Expensive** | Cursor Max, Claude xhigh, Cline `xhigh` | Dropdown option or `/effort` command |

**Tradeoff:** Systems that expose effort as a visible control (Claude Code's `/effort` slider on arrow keys) have better user satisfaction. Systems that hide it (Cursor's hidden "Edit" button on model) create confusion.

### 2.7 Diagnostics Access Pattern

Systems follow a **progressive disclosure** pattern for diagnostics:

| Level | What's shown | How | Example |
|---|---|---|---|
| **Always visible** | Model name, mode | Status bar or footer | All systems |
| **One click away** | Token usage, context % | Status bar element | Claude Code, Continue |
| **Deliberate action** | Cost, timing, tool traces | Slash command or panel | `/usage`, `Ctrl+O`, `cline -v` |
| **Debug mode** | Full prompt/response logs | CLI flag or setting | `--debug`, `--verbose` |

---

## 3. Anti-Patterns

### 3.1 Hidden Effort Controls
- **What:** Effort controls buried behind obscure UI (e.g., Cursor's "Edit" button on model that requires hover + click).
- **Why it's bad:** Users don't know they can control speed/cost/quality. They blame the model for being too slow or too shallow.
- **Evidence:** Community complaints about Cursor's hidden effort levels. Claude Code's arrow key effort adjustment is praised.

### 3.2 Auto-Apply Without User Awareness
- **What:** Writing changes to disk without showing a diff first.
- **Why it's bad:** Erodes trust. Users feel loss of control. Leads to "the AI changed my files without asking" complaints.
- **Evidence:** Cursor 3.0's auto-apply default was the most controversial UX change in their history. The inline diffs toggle had to be added back due to backlash.

### 3.3 No Cancellation During Streaming
- **What:** No stop button or keyboard shortcut to interrupt generation.
- **Why it's bad:** Users must wait for the model to finish, then undo. Wastes tokens and time.
- **Evidence:** OpenCode currently has no cancel button. Every other system provides one.

### 3.4 Single Mode for All Tasks
- **What:** No distinction between "exploratory" (read-only) and "execution" (read-write) modes.
- **Why it's bad:** Users fear accidental changes during casual exploration. Reduces willingness to experiment.
- **Evidence:** Every system except OpenCode now separates planning/reading from execution (Claude Code's `plan` mode, Cursor's `Ask`, Aider's `/ask`, etc.)

### 3.5 Zero Execution Visibility
- **What:** Showing only the final response text, hiding tool calls, phases, and intermediate steps.
- **Why it's bad:** Users can't understand what the AI is doing. Creates anxiety ("is it stuck?"). Reduces trust.
- **Evidence:** OpenCode currently shows only text deltas. All other systems show tool call cards or inline traces.

### 3.6 No Permission Differentiation
- **What:** Treating all operations the same — no distinction between reading a file, editing a file, and running a terminal command.
- **Why it's bad:** Users must approve trivial operations (reading) with the same friction as dangerous ones (deleting files). Leads to approval fatigue.
- **Evidence:** All mature systems have graduated permission levels. Claude Code's `acceptEdits` mode auto-approves file edits but prompts for commands.

### 3.7 Env Vars as Only Configuration
- **What:** Requiring API keys and config settings as environment variables with no in-app setup UI.
- **Why it's bad:** Poor discoverability. Users miss configuration options. Error messages are confusing.
- **Evidence:** Systems with setup wizards (Codex CLI, Claude Code's `/init`, Cline's VS Code settings) have lower churn than systems requiring manual `.env` files.

### 3.8 No Context Visibility
- **What:** Not showing the user what context is being sent to the model.
- **Why it's bad:** Users don't know what the AI can see. Leads to surprise when the AI references files the user forgot about, or misses context the user assumed was included.
- **Evidence:** Continue's `@`-mention chips and context items display is praised. Cursor's context bar shows attached files.

### 3.9 Death by Permission Prompts
- **What:** Prompting for every single tool call with no caching or session-level approvals.
- **Why it's bad:** Approval fatigue. Users start blindly approving without reviewing.
- **Evidence:** Claude Code's "Yes, don't ask again" and Cline's `--auto-approve` categories exist precisely to address this.

### 3.10 Invisible Streaming State
- **What:** No visual indication that streaming is happening (not showing the stream, or showing a static "waiting" state).
- **Why it's bad:** Users don't know if the system is working or stuck.
- **Evidence:** OpenCode shows "Streaming..." text but no streaming cursor/animation. All other systems show actual token flow.

---

## 4. Best Practices

### 4.1 Progressive Disclosure of Controls
- **Practice:** Show the most common controls (model, mode) by default. Hide advanced controls (effort, token budget, separate edit models) behind expandable panels or settings.
- **Evidence:** Continue's onboarding card + `@` menu. Claude Code's `/effort` as explicit command vs always-visible slider.
- **Why:** Beginners aren't overwhelmed. Power users can find what they need.

### 4.2 Read-Only Mode as a First-Class Citizen
- **Practice:** Always provide a read-only mode where the AI can read files and search but cannot edit or execute.
- **Evidence:** Cursor's "Ask" mode, Claude Code's "plan" mode, Continue's "Plan" mode, Aider's "ask" chat mode.
- **Why:** Enables safe exploration. Users can ask questions without fear of uncommitted changes.

### 4.3 Show Tool Calls, Not Just Text
- **Practice:** Render tool calls as distinct cards or blocks showing tool name, arguments, status (running/done/error), and results.
- **Evidence:** Claude Code (expandable in transcript viewer), Cursor (expandable cards), Continue (tool call cards with status), Cline (cards with approval).
- **Why:** Transparency builds trust. Users can see what the AI is doing and intervene if needed.

### 4.4 Always Show a Stop/Cancel Button
- **Practice:** Provide a visible stop button during generation and a keyboard shortcut (Ctrl+C or Esc).
- **Evidence:** Every system except OpenCode has this.
- **Why:** Users must be able to interrupt the AI. Wasted tokens, wrong direction, or accidental triggers all need immediate cancellation.

### 4.5 Permission Caching with "Don't Ask Again"
- **Practice:** Cache permission decisions per session. Offer "allow once", "allow this session", "always allow for this command".
- **Evidence:** Cline's `auto-approve` categories, Claude Code's "Yes, don't ask again", Continue's per-tool policies.
- **Why:** Reduces friction for repetitive operations without sacrificing safety for novel ones.

### 4.6 Diff Preview Before Application
- **Practice:** Show a diff of every file change before writing to disk.
- **Evidence:** Cursor's inline diffs (the feature that won them the market), Aider's colored diffs after commit, Cline's VS Code diff editor integration.
- **Why:** Users must see what changed before approving. This is the single highest-trust UX pattern.

### 4.7 Context Visibility via @-Mentions
- **Practice:** Let users explicitly attach context (files, search results, terminal output) via an `@` mention system in the input.
- **Evidence:** Cursor's `@` menu (File, Folder, Code, Codebase, Docs, Web, Terminal, Git, Notepads), Continue's `@` context providers.
- **Why:** Users feel in control of what the AI can see. Reduces "how did it know about that?" confusion.

### 4.8 Cost and Token Transparency
- **Practice:** Show token usage and estimated cost in the status bar or accessible via a simple command.
- **Evidence:** Claude Code's status line cost + context %, Cline's status bar tokens/cost, Grok Build's `/usage` command.
- **Why:** Users make informed decisions about model selection and effort level when they can see the cost.

### 4.9 Double-Tap Ctrl+C for Force Exit
- **Practice:** First Ctrl+C cancels the current operation. Second Ctrl+C within 2 seconds exits the application.
- **Evidence:** Aider (2s window), Cline (double-press exit), Claude Code (2nd Ctrl+C exits).
- **Why:** Single tap should never kill the process. Users often hit Ctrl+C reflexively.

### 4.10 Engine Health Visible on Demand
- **Practice:** Provide a health/diagnostic summary accessible via a simple command (`doctor`, `inspect`, `diagnostics`).
- **Evidence:** Codex CLI's `codex doctor`, Cline's `cline doctor`, Claude Code's `claude doctor`.
- **Why:** Users need a way to verify configuration, check auth status, and diagnose issues without digging through logs.

---

## 5. Recommended BuilderBoard Engine UX

### 5.1 Current State (Baseline)

| Area | Current | Gap vs. Best Practice |
|---|---|---|
| **Model selection** | Dropdown in pane header with 4 models | No provider selection, no account management inline |
| **Mode selection** | None | No read-only/plan mode |
| **Effort control** | Reasoning level dropdown | No fast/normal toggle |
| **Capability discovery** | None | No slash commands, no `@` mentions, no onboarding |
| **Runtime status** | "Streaming..." text only, display state in header | No tool call cards, no phase indicators, no streaming animation |
| **Permission visibility** | None | No approval flow, no diff preview, no permission modes |
| **Execution diagnostics** | Console-only runtime probes | No token usage, no cost, no timing, no tool traces |
| **Cancellation** | None | No stop button, no keyboard shortcut |
| **Engine health** | None | No connection status, no rate limits, no diagnostic command |
| **User trust** | Project scope enforcement | No diff preview, no undo, no permission gates |
| **Context display** | None | No `@` mentions, no context indicator, no file attachment |

### 5.2 Recommended UX Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Pane Header                                  │
│                                                                      │
│  ┌──────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │  #1   │  │ Provider │  │  Model   │  │  Mode    │  │ Effort   │ │
│  │ pane  │  │ OpenAI ▼ │  │ GPT-5.5▼ │  │ Agent▼  │  │ High ▼  │ │
│  │  tag  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │
│  └──────┘                                                           │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                      Message List                               ││
│  │                                                                  ││
│  │  ┌─────────────────────────────────────────────┐                ││
│  │  │ User: "Refactor the auth module"            │                ││
│  │  └─────────────────────────────────────────────┘                ││
│  │                                                                  ││
│  │  ┌─────────────────────────────────────────────┐                ││
│  │  │ ● Reasoning  ─────────────────────────────  │  Phase         ││
│  │  │                                              │  indicators   ││
│  │  │ ● Searching codebase... [┊┊┊┊┊┊┊┊┊░░░]     │  (collapsible) ││
│  │  │   UserService.ts: checkAuth flow found      │                ││
│  │  │                                              │                ││
│  │  │ ● Reading UserService.ts                     │  Tool call     ││
│  │  │   ┌────────────────────────────────────────┐ │  cards         ││
│  │  │   │ Line 42: async function checkAuth()... │ │  (expandable)  ││
│  │  │   └────────────────────────────────────────┘ │                ││
│  │  │                                              │                ││
│  │  │ ● Editing authService.ts                    │                ││
│  │  │   ┌─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐ │  Inline diff   ││
│  │  │   │ - old code                            │ │  preview        ││
│  │  │   │ + new code                            │ │                ││
│  │  │   │ [Accept] [Reject]                     │ │                ││
│  │  │   └─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘ │                ││
│  │  │                                              │                ││
│  │  │ ● Running tests... [┊┊┊┊┊┊┊┊┊┊┊┊┊┊┊┊░]    │                ││
│  │  │   ✓ 5 passed, 0 failed                     │                ││
│  │  │                                              │                ││
│  │  │ ──────────────────────────────────────────── │ Streaming      ││
│  │  │ The auth module has been refactored to...    │ text           ││
│  │  │ ──────────────────────────────────────────── │                ││
│  │  └─────────────────────────────────────────────┘                ││
│  │                                                                  ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │  Context: @file UserService.ts @folder src/auth/ @terminal     │ │
│  │  └───────────────────────────────────────────────────── [Send] ─││
│  │  [Stop] ← appears during streaming                              ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │  Status bar:  │ Model: GPT-5.5 │ Mode: Agent │ Context: 42%   ││
│  │               │ Session: $0.34 │ ⏱ 1m 23s   │ ● ● ● ○        ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

### 5.3 Component Specifications

#### 5.3.1 Pane Header Controls

| Control | UX Pattern | Details |
|---|---|---|
| **Provider selector** | Dropdown | OpenAI, Anthropic, Google, Ollama, Local. Show "(needs key)" badge if unconfigured. |
| **Model selector** | Dropdown + fast toggle | List models from selected provider. Toggle "Fast" mode next to model name. |
| **Mode selector** | Dropdown | **Ask** (read-only, no tools), **Plan** (read-only, with tool results), **Agent** (full execution). Match Cursor/Continue pattern. |
| **Effort selector** | Dropdown | **Auto** (default), **Fast** (cheaper/faster model), **Deep** (more reasoning). Rename from "Reasoning Level". |
| **Pane status** | Badge | Show current state: idle, gathering context, streaming, error. Use icon + short text. |

#### 5.3.2 Message List (Enhanced)

| Element | UX Pattern | Details |
|---|---|---|
| **Phase indicators** | Collapsible cards | Show each phase of agent execution: reasoning, searching, reading, editing, testing. Collapse by default, expand on click. |
| **Tool call cards** | Status + expandable args/output | Show tool name, status icon (running ✓ ✗), elapsed time. Expand to show arguments and results. |
| **Inline diffs** | Before application | Show diff with green/red highlighting. Accept/Reject/Edit buttons. Only shown in Plan and Agent modes. |
| **Streaming cursor** | Blinking cursor | Animated cursor at end of streaming text. Pulsing when model is generating. |
| **Reasoning display** | Collapsible section | Show model's reasoning/thinking tokens in a collapsible section with "Thought for Xs" header. |
| **Message metadata** | Compact footer | Model used, tokens consumed, generation time. Shown on hover or as compact text. |

#### 5.3.3 Input Area (Enhanced)

| Element | UX Pattern | Details |
|---|---|---|
| **@-mention context** | Inline chips | `@file`, `@folder`, `@terminal`, `@git`, `@web`. Selected items appear as chips in input. |
| **Stop button** | Red square icon | Appears during streaming. Replaces Send button. Ctrl+C as keyboard shortcut. |
| **Send button** | Arrow icon | Disabled when input empty. Shows "Send" tooltip. |
| **Context usage** | Thin bar | Bar in bottom-left of input area. Height/width = context window % used. Hover for details. |

#### 5.3.4 Status Bar (New Component)

| Element | Details |
|---|---|
| **Model name** | Currently selected model |
| **Mode indicator** | Ask/Plan/Agent with icon |
| **Context %** | Thin progress bar with percentage |
| **Session cost** | Running total of session cost |
| **Duration** | Session elapsed time |
| **Health dots** | ● ● ● ○ — for connection/auth/model status |
| **Permissions mode** | Icon showing current permission posture |

#### 5.3.5 Interaction with Existing Components

| Component | Change Required | Effort |
|---|---|---|
| `ChatControls.tsx` | Replace with new header. Add provider selector, rename reasoning to effort, add mode selector, add fast toggle. | 1 day |
| `ChatComposer.tsx` | Add @-mention menu, stop button, context chips, context bar. | 2 days |
| `MessageList.tsx` | Add tool call cards, diff preview, phase indicators, streaming cursor, message metadata. | 3-4 days |
| `Pane.tsx` | Add status bar below ChatComposer. Add pane status badge to header. | 1 day |
| `PaneGrid.tsx` | Add pane resize handles, maximize toggle. | 2 days |
| `usePaneChat.ts` | Add stop/cancel signal wiring. Add context attachment state. | 1 day |
| New: `StatusBar.tsx` | New component for pane-level status bar. | 1 day |
| New: `ToolCallCard.tsx` | New component for rendering tool calls with status. | 2 days |
| New: `DiffPreview.tsx` | New component for inline diff with accept/reject. | 2 days |
| New: `ContextInput.tsx` | Enhanced input with @-mentions and context chips. | 2 days |
| New: `PhaseIndicator.tsx` | Collapsible phase display for agent execution. | 1 day |
| New: `PermissionDialog.tsx` | Modal for tool call approval. | 2 days |

### 5.4 What Users Actually Need to See

| Need | Show | Where |
|---|---|---|
| **What model am I using?** | Model name | Pane header + status bar |
| **What mode am I in?** | Mode name + icon | Pane header + status bar |
| **Is it working?** | Streaming cursor + phase indicators | Message list |
| **What is it doing?** | Tool call cards + phase text | Message list |
| **What will change?** | Diff preview | Before edits |
| **How much will this cost?** | Token count + cost estimate | Status bar |
| **Can I stop it?** | Stop button + Ctrl+C | Input area |
| **What can it see?** | @-mention chips + context list | Input area |
| **What context is left?** | Context % bar | Status bar + input area |
| **Is everything OK?** | Health dots | Status bar |
| **What happened?** | Tool traces + logs | Expandable tool call cards |
| **How do I undo?** | Undo button or git | Message level |
| **How do I configure?** | Settings icon + onboarding | Sidebar + header |

### 5.5 What Should Remain Hidden

| Hide | Why | Access via |
|---|---|---|
| Raw prompt/response logs | Too verbose for normal use | Debug mode (`--debug`) or console |
| Provider API error details | Confusing to non-technical users | Friendly error messages |
| Token-level timing breakdown | Overwhelming | Status bar (aggregate only) |
| Model catalog negotiation | Irrelevant to users | Auto-detected |
| Retry logic / fallback chains | Implementation detail | Transparent (auto) |
| OAuth token management | Security-sensitive | `Settings > Accounts` |
| Internal execution DAG | Too complex | Debug mode |
| Raw configuration precedence | Confusing | `Settings > Debug config` |

### 5.6 What Creates Confidence

| UX Pattern | Why It Builds Trust |
|---|---|
| **Diff preview before application** | User sees exactly what changed before it happens |
| **Read-only mode (Ask/Plan)** | User can explore without fear of changes |
| **Tool call cards with status** | User sees what the AI is doing at each step |
| **Permission gates with "don't ask again"** | User controls the boundary |
| **Cost and token transparency** | User makes informed decisions |
| **Undo/rollback** | User knows mistakes are reversible |
| **Context visibility (@-mentions)** | User knows what the AI can see |
| **Open source** | User can audit the code |
| **Clear safety labeling** | User understands risk levels |
| **Engine health (on demand)** | User can verify the system is working |

### 5.7 What Creates Confusion

| UX Pattern | Why It Reduces Trust |
|---|---|
| **Auto-apply without diff** | User feels loss of control |
| **Hidden effort controls** | User can't optimize speed/cost |
| **No stop button** | User feels trapped |
| **Invisible streaming state** | User doesn't know if it's working |
| **Permission prompt spam** | Approval fatigue → blind acceptance |
| **Raw error messages (stack traces)** | User feels the system is broken |
| **No context indicator** | User doesn't know what the AI sees |
| **Single mode for all tasks** | User fears accidental changes |
| **No cost transparency** | User gets surprised by bills |
| **No undo mechanism** | User fears irreversible damage |

---

## 6. Missing Opportunities

These are UX patterns none of the 8 researched systems implement well that BuilderBoard could differentiate with:

### 6.1 Pane-Level Mode Diversity
No system supports **different modes in different panes simultaneously**. BuilderBoard's multi-pane architecture uniquely enables:
- Pane 1: Agent mode (full execution on backend)
- Pane 2: Ask mode (read-only exploration of frontend)
- Pane 3: Plan mode (designing the database schema)

**Why it matters:** Developers working on large projects context-switch between modes constantly. Having all panes in the same mode is wasteful.

### 6.2 Execution Timeline / History
No system provides a **visual timeline of execution** — a Gantt-chart-style view showing when each tool call started, its duration, and its result.

**Why it matters:** Users debugging long-running agent sessions need to understand the sequence of events. Current systems only show the final transcript.

### 6.3 Live Cost Dashboard
No system shows **real-time cost accumulation** per-turn or per-session in the UI. Claude Code's status line comes closest but is still text-only.

**Why it matters:** Users who BYOK want to see their spending in real-time, not after the fact.

### 6.4 Permission Rule Builder
No system has a **visual permission rule builder**. All require editing JSON or learning a DSL (Claude Code's `Tool(pattern)` syntax).

**Why it matters:** Non-technical users want to say "allow git commands but block rm and force push" without writing regex.

### 6.5 Collaborative Pane Sharing
No system supports **sharing a live pane view** (watch another developer's AI execution in real-time).

**Why it matters:** Team onboarding, pair debugging, and code review would benefit from seeing what the AI is doing in someone else's session.

### 6.6 Diff Staging (Like Git Add -p)
No system lets users **stage individual diff hunks** from an AI's multi-file change, similar to `git add -p`.

**Why it matters:** Users often want parts of a change but not all of it. Current systems offer accept-all or reject-all per file.

### 6.7 Effort Budget Per Pane
No system lets users set a **budget per pane** (e.g., "spend at most $0.50 on this pane") or per-request.

**Why it matters:** Cost-conscious users want guardrails, not just visibility.

---

## 7. Confidence Score Rationale

**Score: 90/100**

| Factor | Score | Reasoning |
|---|---|---|
| Research coverage | 95/100 | All 8 major systems analyzed with code-level depth for UX patterns. Direct UI audit of OpenCode. Source-level research of Codex CLI, Cline, Aider, Grok Build. Detailed analysis from documentation and community for Claude Code, Cursor, Continue. |
| Pattern confidence | 92/100 | UX patterns are highly consistent across all 8 systems. The model selector, mode selector, permission flow, and tool call display follow almost identical patterns. Low risk of missing something fundamental. |
| Architectural fit | 88/100 | Recommended UX aligns with existing OpenCode strengths (multi-pane, streaming). Requires building 10+ new UI components but leverages existing Tauri event infrastructure. |
| Anti-pattern validation | 90/100 | Anti-patterns confirmed across multiple systems. Auto-apply without diff (Cursor community backlash), hidden effort controls (Cursor complaints), no stop button (all systems have one except OpenCode). |
| Missing opportunity differentiation | 85/100 | Identified 7 genuine gaps. Multi-pane mode diversity is uniquely feasible for BuilderBoard. Others (timeline, cost dashboard, permission builder) would require significant investment. |
| Feasibility | 90/100 | UX changes are frontend-only (TypeScript/React). No backend changes needed for most patterns. Status bar, tool call cards, phase indicators, diff preview all reuse existing `ExecutionEvent` types. |

**Key uncertainties lowering the score:**
- Diff preview component depends on having diff data from the backend. OpenCode's current `ExecutionEvent` enum has `ToolCallStarted`/`ToolCallFinished` but no diff-specific events. May need to extend the event model. Mitigation: phase indicators and tool call cards can ship first; diff preview ships in Phase 9B.
- Permission dialog UX depends on PermissionGate from Phase 8.9B. The trust model needs backend implementation before the UI can be wired.
- @-mention context system requires file system commands to be exposed in the UI. Currently `filesystemCommands.ts` exists but has zero UI consumption.

---

## Summary

| Finding | Detail |
|---|---|
| **Universal patterns** | Model selector, mode selector, permission flow, tool call display, streaming text |
| **Anti-patterns to avoid** | Auto-apply without diff, hidden effort controls, no stop button, no read-only mode, no permission differentiation |
| **Trust builders** | Diff preview, read-only mode, tool call cards, permission gates, undo, context visibility, cost transparency |
| **Trust destroyers** | Auto-apply, invisible streaming, no cancellation, permission spam, raw errors |
| **Recommended UX architecture** | 4 new components (StatusBar, ToolCallCard, DiffPreview, ContextInput), 6 enhanced components, 15 UI elements total |
| **BuilderBoard advantage** | Multi-pane mode diversity is unique — no other system supports different modes in different panes simultaneously |
| **Current gaps** | No stop button, no tool call display, no permission UX, no diff preview, no context attachment, no status bar, no cost/token display |

**OpenCode currently has a 0/10 UX for execution engine visibility.** The recommended changes bring it to a competitive baseline with the market.
