# Phase 8.9B — CLI Execution Engine Architecture Research

**Role:** Research Engineer
**Goal:** Research best practices for integrating CLI-native AI coding systems into a generalized execution architecture.
**Date:** 2026-06-24
**Status:** Complete
**Confidence Score:** 90/100

---

## Table of Contents

1. [Comparison Matrix](#1-comparison-matrix)
2. [Common Process Architecture](#2-common-process-architecture)
3. [Best Practices](#3-best-practices)
4. [Anti-Patterns](#4-anti-patterns)
5. [Recommended CLIExecutionEngine Design](#5-recommended-cliexecutionengine-design)
6. [Grok-Specific Considerations](#6-grok-specific-considerations)
7. [Claude/OpenCode Considerations](#7-claudeopencode-considerations)
8. [Confidence Score Rationale](#8-confidence-score-rationale)

---

## 1. Comparison Matrix

### 1.1 Process Lifecycle

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Spawn mechanism** | `tokio::process::Command` (pipe) or `portable_pty` | `child_process` (BashTool `exec`) | `child_process.spawn()` | `pexpect.spawn()` or `subprocess.Popen` | `child_process.exec()` (fg) / `spawn()` (bg) | `child_process.spawn()` | None |
| **Process tracking** | `HashMap<ProcessId, ProcessEntry>` with Starting/Running states | `StreamingToolExecutor` in-memory | Agent-level abort controller chain | None (sync) | `Map<number, BackgroundProcess>` | `Map<string, ProcessInfo>` in `processTerminalStates` | None |
| **Per-process overhead** | 3 background tasks (stdout, stderr, exit watcher) + 1MB retained buffer + 256-event log | Per-tool generator + React component | Rolling collector + timeout | None (sync wait) | Log file writer + exit handler | stdout/stderr listeners + 2 timeout handlers | N/A |
| **Max concurrency** | Single session, 1 process at a time by default | `MAX_TOOL_USE_CONCURRENCY = 10` + concurrency-safe classification | Sequential or parallel (configurable) | Single process (blocking) | 8 background processes max | Uncapped | N/A |
| **Background processes** | No | Yes (Ctrl+B, timeout, explicit) | No | No | Yes (detached subagents) | Yes (detached + unref) | N/A |

### 1.2 Stdout/Stderr Streaming

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Stdout streaming** | `mpsc` channel → retained buffer (1MB) → broadcast to subscribers | Generator-based, ~1s poll interval, `EndTruncatingAccumulator` | `RollingCollector` (middle-truncate at 48K chars) | Character-by-character `read(1)` | `exec()` buffer (10MB max) | `child.stdout.on('data')` → `onPartialOutput()` | HTTP SSE only (no subprocess) |
| **Stderr handling** | Separate `mpsc` channel, separate retained buffer | **Merged into stdout** (combined fds) | Appended to stdout with `[stderr]` separator | Redirected to stdout (`STDOUT`) | Appended `\nSTDERR: ${stderr}` | Separate `child.stderr.on('data')` | N/A |
| **Buffer limit** | 1MB per process, FIFO eviction | 64MB total output, truncated at 64MB | 48,000 chars, middle-truncated | Unlimited (accumulated in memory) | 10MB | Unlimited | N/A |
| **Retained output** | Event log with replay buffer (256 events) | Full output in memory during execution, persisted to disk for large | Full output until collect completes | None (console only) | Log files on disk | Full output in memory | N/A |
| **Progress granularity** | Chunk-level (8KB reads) | ~1s polling | Data event level | Character-level | Data event level | Data event level | Token-level (model output) |

### 1.3 Cancellation & Signals

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Cancellation mechanism** | `turn/interrupt` RPC → `process_group::interrupt()` | Hierarchical `AbortController` tree | `AbortController.signal` → `killProcessTree()` | Double Ctrl+C (2s window) | `AbortSignal` → SIGTERM → 1s → SIGKILL | `killTerminalProcess()` → SIGTERM → 5s → SIGKILL | `AtomicBool` (declared, unwired) |
| **Signal sequence** | `process_group::interrupt()` (SIGINT to PGID) | AbortController tree propagation | SIGINT → `abortAll()` → process group SIGKILL | First Ctrl+C = warning, second = `sys.exit()` | SIGTERM → 1s → SIGKILL | SIGTERM → 5s → SIGKILL | None |
| **Process group kill** | Yes (negative PGID via `kill(pid, SIGINT)`) | Yes (through `shellCommand` abort) | Yes (negative PID `process.kill()`) | No | Yes (SIGTERM → SIGKILL) | No (single process `process.kill()`) | N/A |
| **Timeout handling** | Per-call timeout parameter | Per-command timeout + auto-background on timeout | Per-command timeout + session-level timeout | No (infinite wait) | 30s default + configurable | 120s default (`DEFAULT_TOOL_TIMEOUT_MS`) | N/A |
| **Double-interrupt protection** | No | No | Yes (2nd press = force exit) | Yes (2nd press = exit) | No | No | N/A |

### 1.4 Subprocess Management

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Shell invocation** | No shell (direct exec) | `bash -c` or `zsh` | `/bin/bash -c` or `powershell` | `$SHELL -i -c` or `sh` | `sh -c` | `$SHELL -l -c` | N/A |
| **Process isolation** | OS sandbox (Seatbelt/bwrap/Landlock) | Sandbox adapter (optional) | None | None (git revert safety) | Landlock/Seatbelt in official | None | None |
| **Process tree management** | Child-terminator kills descendant processes | `scrubBareGitRepoFiles()` post-sandbox | Negative PID + SIGKILL | None | Background process map + SIGTERM/SIGKILL | Detached + unref for background | N/A |
| **Stdin writing** | `process/write` RPC with idempotency IDs | Generator input channel | stdio `pipe` | `pexpect.send()` | `child.stdin.write()` | `child.stdin.write()` | N/A |
| **Reattachment** | No (process-bound session) | Yes (background task IDs) | No | No | Yes (bg task IDs + file state) | No | N/A |
| **IPC protocol** | JSON-RPC on stdio | MCP JSON-RPC for extensions | JSON-RPC (MCP) + file-based approval | None | ACP (JSON-RPC on stdio) | JSON-RPC (MCP) | None |

### 1.5 Working Directory & Environment

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **CWD model** | Static per-session, set at start | Persistent per-session, `preventCwdChanges` flag | Static per-session | Git root (fixed per-repo) | **Stateful** — `cd` changes `BashTool.cwd` | Static per-command (via `cwd` param) | None |
| **CWD passthrough** | `command.current_dir(cwd)` | Per-shell invocation | `cwd: config.cwd` | `cwd=self.root` (linter only) | `cwd: this.cwd` (stateful) | `cwd: resolvedCwd` param | N/A |
| **Worktree support** | No | `EnterWorktreeTool/ExitWorktreeTool` | No | `--subtree-only` | No | No | N/A |
| **Env var model** | `env_clear()` then explicit set with `EnvPolicy` (inherit/set/exclude/include_only) | Inherit + selective override + sandbox limits | `{...process.env, ...config.env}` | Inherit + `.env` file + git attribution | Inherit + `FORCE_COLOR=0` | Inherit + color overrides | `HashMap` field (always empty) |
| **Process hardening** | Pre-main: strip `LD_*`/`DYLD_*`, disable ptrace, core dumps | Strip unsafe vars from permission matching | None | None | None | None | None |

### 1.6 Permissions

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Permission model** | OS sandbox (deny by default, allow specific paths) | Rule-based (exact/prefix/wildcard + path constraints + classifier) | Policy-based (`enabled` + `autoApprove`) | No permission system (git revert = safety) | Sandbox profiles + tool-level rules | Pattern-matching (glob-style) | `ApprovedScope` for filesystem only |
| **Approval modes** | Approval per tool call | `default` / `acceptEdits` / `plan` / `bypassPermissions` / `auto` | Per-tool policy override | N/A | `ask` / `always-approve` / `dontAsk` / `plan` | `alwaysAllow` / `allowThisSession` / `allowOnce` / `deny` / `ask` | None |
| **Sandbox types** | Seatbelt (macOS), Bwrap+Landlock (Linux), AppContainer (Windows) | Plugable sandbox adapter | Subprocess sandbox (plugin IPC) | None | Landlock/Seatbelt (official), Shuru microVM (community) | None | None |
| **Read-only mode** | Via sandbox policy (no-write mounts) | `plan` mode (no execution) | Tool-level disable | N/A (`read-only` sandbox profile) | `plan` mode (block writes + readonly commands) | Tool-level deny | N/A |
| **Audit log** | Sandbox denial flag in output | Full log of permission decisions | Tool event log | Git history | Sandbox violation logging | Process tracking map | N/A |

### 1.7 Cleanup & Recovery

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Process cleanup** | `Drop` terminates session; `terminate()` kills child + abort tasks | Bare git repo scrubbing + AbortController tree | `cleanup()` removes signal handlers, unsubscribes | None (sync processes terminate naturally) | `cleanup()` kills bg processes + removes temp dirs | `removeRunningProcess()` removes from tracking map | None |
| **Post-exit retention** | 30s retention (process entry), late output still captured | No retention | No retention | N/A | No retention | No retention | N/A |
| **Crash recovery** | Idempotent stdin writes, output retention after exit, event replay buffer | Session persists in `.claude/` | Abort-in-progress protection (suppress unhandled rejections) | Daemon threads only, no state recovery | Session persisted to `~/.grok/sessions/` | No crash recovery for in-flight processes | DB-backed state (pane/message persistence) |
| **Cleanup on error** | Process entry removed after retention | AbortController cascade to children | Try/catch per stage, errors logged | `finally` block stops spinner + watchers | `Promise.allSettled` for parallel cleanup | `clearTimeout` on close/error | Error emitted via Tauri event |

### 1.8 Performance

| Dimension | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue | OpenCode (current) |
|---|---|---|---|---|---|---|---|
| **Read buffer** | 8KB chunks | Chunk-level (Node.js default) | Data event level | 1 character | Data event level | Data event level | 4KB (HTTP SSE) |
| **Output retention limit** | 1MB per process | 64MB total per command | 48K chars per command | Unlimited | 10MB per command | Unlimited | DB-stored |
| **Channel capacity** | 128 (mpsc) / 256 (broadcast) / 256 (RPC notify) | N/A | N/A | N/A | N/A | N/A | Unbounded (`mpsc::unbounded_channel`) |
| **Progress throttle** | Immediate (8KB chunks) | 2s threshold before progress UI shown | Immediate | Immediate | Immediate | Immediate | Immediate |
| **Default timeout** | Per-call configurable | 30s (auto-background after) | 30s per command, 60s per tool wrap | None | 30s | 120s | None |
| **Process pooling** | No (fresh per command) | No | No | No | No | No | N/A |

---

## 2. Common Process Architecture

### 2.1 Universal Execution Flow

Every system follows this pattern with minor variations:

```
1. PARSE TOOL CALL
   └─ Extract command string + arguments + options (timeout, cwd, env)

2. VALIDATE & AUTHORIZE
   ├─ Schema validation (Zod, serde, etc.)
   ├─ Permission check (auto-approve? ask? deny?)
   └─ Sandbox decision (sandboxed? direct?)

3. SPAWN PROCESS
   ├─ Resolve shell (bash/zsh/powershell/direct exec)
   ├─ Set working directory
   ├─ Build environment (inherit + overrides + sandbox limits)
   └─ Call spawn() / exec()

4. STREAM OUTPUT
   ├─ Attach stdout/stderr listeners
   ├─ Accumulate output (buffer, file, or collector)
   ├─ Yield progress to caller (generator, callback, or channel)
   └─ Check for timeout

5. COLLECT RESULT
   ├─ Wait for process exit
   ├─ Capture exit code
   ├─ Merge stdout/stderr (if configured)
   └─ Truncate large output if needed

6. RETURN TO LLM
   ├─ Format result as tool_result (text or persisted file reference)
   └─ Include metadata (exit code, truncated flag, timing)
```

### 2.2 Shared Architectural Primitives

These abstractions appear in every system:

| Primitive | Codex CLI | Claude Code | Cline | Aider | Grok Build | Continue |
|---|---|---|---|---|---|---|
| **Process handle** | `ProcessHandle` (PID + session) | Shell command promise | `ChildProcess` | `pexpect.spawn` / `Popen` | `ChildProcess` | `ChildProcess` |
| **Output accumulator** | Retained buffer + event log | `EndTruncatingAccumulator` | `RollingCollector` | `BytesIO` | Buffer + log file | Terminal output string |
| **Timeout controller** | Per-command timeout param | `timeout` option on tool call | `setTimeout` on promise | None | `timeout` on `exec()` | Two-phase timeout (120s + 5s SIGKILL) |
| **Cancellation signal** | `turn/interrupt` RPC | AbortController `signal` | `AbortSignal` listener | Keyboard handler | `AbortSignal` | `killTerminalProcess()` |
| **Permission gate** | Sandbox policy + approval | Permission rule engine | Tool policy + approval | Git revert | Sandbox profile + approval | Tool policy pattern match |
| **Process map** | `HashMap<ProcessId, ProcessEntry>` | In-memory tool state | Agent runtime state | N/A | `Map<number, BackgroundProcess>` | `Map<string, ProcessInfo>` |
| **Structured result** | `ReadResponse` (chunks, exit_code, closed, sandbox_denied) | Tool result object (stdout, stderr, interrupted) | Command exit error or output string | Exit status + output | `{success, output, error}` | Terminal output string |
| **Kill strategy** | SIGINT→PGID, then `terminate()` kills all | SIGTERM via abort | SIGKILL on process group | N/A | SIGTERM→1s→SIGKILL | SIGTERM→5s→SIGKILL |

### 2.3 Two Paths: Pipe vs. PTY

All systems split into two subprocess modes:

| Mode | When Used | stdout/stderr | stdin | Pros | Cons |
|---|---|---|---|---|---|
| **Pipe** (`Stdio::piped()`) | Non-interactive commands, automation | Separate or merged streams | Write via `child.stdin.write()` | Full output control, structured capture | No TTY, some programs behave differently (colors, progress bars) |
| **PTY** (`portable_pty`, `pexpect.spawn()`) | Interactive commands, editors, REPLs | Combined stream via PTY master | Write via PTY master fd | Programs think they're in a terminal, full interactive support | Blocking reads, more overhead, harder to parse structured output |

**Codex CLI** is the only system implementing both — it selects based on a `tty` parameter. All others default to pipe mode with optional PTY.

### 2.4 The Process Object (Common Schema)

Across all systems, the unit of process management is a structure containing:

```typescript
interface ManagedProcess {
  id: string | number;           // Unique process identifier
  pid: number;                   // OS process ID
  handle: ChildProcess | PopenHandle; // OS-level process handle
  startedAt: number;             // Timestamp (ms)
  
  // Output streams
  stdout: ReadableStream<Uint8Array | string>;
  stderr: ReadableStream<Uint8Array | string>;
  
  // State
  state: 'starting' | 'running' | 'exiting' | 'exited' | 'failed';
  exitCode: number | null;
  killed: boolean;
  
  // Bound resources
  timeoutId: TimerHandle | null;
  abortController: AbortController | null;
  cleanupFns: Array<() => void>;
}
```

### 2.5 Kill Strategy (Universal)

```
╔═══════════════════════════════════════════════════════════╗
║                  KILL STRATEGY                            ║
╠═══════════════════════════════════════════════════════════╣
║                                                           ║
║  1. CANCEL REQUEST (AbortController, Ctrl+C, timeout)     ║
║     │                                                      ║
║     ▼                                                      ║
║  2. SIGTERM (or equivalent graceful shutdown)              ║
║     ├─ Codex CLI: process_group::interrupt() (SIGINT PG)  ║
║     ├─ Cline: process.kill(-childPid, 'SIGKILL')          ║
║     ├─ Grok Build/Continue: process.kill('SIGTERM')       ║
║     │                                                      ║
║     ▼  (after grace window: 1s-5s)                        ║
║  3. SIGKILL (force kill)                                  ║
║     ├─ Codex CLI: session.terminate() → ChildTerminator   ║
║     ├─ Cline: already sent SIGKILL in step 2              ║
║     ├─ Grok Build: SIGKILL after 1s grace                 ║
║     └─ Continue: SIGKILL after 5s grace                   ║
║                                                           ║
║     ▼                                                      ║
║  4. CLEANUP                                               ║
║     ├─ Remove from process tracking map                   ║
║     ├─ Clear timeout timers                               ║
║     ├─ Close log file handles (if any)                    ║
║     └─ Reject/complete pending promise                    ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
```

---

## 3. Best Practices

### 3.1 Two-Phase Kill with Configurable Grace Window

- **Practice:** SIGTERM first, wait a configurable grace period (default 3s), then SIGKILL.
- **Evidence:** Every system does this. The grace window varies: Codex CLI uses process_group interrupt, Cline uses `SIGKILL` directly (no SIGTERM), Grok Build uses 1s, Continue uses 5s.
- **Why:** SIGTERM gives processes a chance to clean up (temp files, child processes). SIGKILL ensures no orphans.
- **Recommendation:** 3s default, configurable per-execution profile. Use process group kill (negative PID) on Unix.

### 3.2 Separate stdout/stderr with Optional Merge

- **Practice:** Keep stdout and stderr as separate streams by default. Provide a merge option.
- **Evidence:** Codex CLI, Cline, Continue keep them separate. Claude Code merges them. Aider merges via `STDOUT`.
- **Why:** Separate streams enable semantic filtering (errors vs. output), structured parsing (JSON on stdout, diagnostics on stderr), and selective display.
- **Recommendation:** Store both separately. Expose a `combineOutput` option that appends stderr to stdout with a `[stderr]` marker.

### 3.3 Bounded Output Buffers with Middle Truncation

- **Practice:** Cap output at a configurable limit (default 48K-1MB). When exceeded, truncate the middle with a notice.
- **Evidence:** Cline's `RollingCollector` (48K chars, middle-truncate) is the most elegant. Codex CLI (1MB, FIFO eviction) and Grok Build (10MB) use simpler models.
- **Why:** Prevents OOM from runaway processes. Middle truncation preserves both the start (context) and end (result) of output.
- **Recommendation:** Use Cline's rolling collector pattern: keep first N/2 chars, add truncation notice, keep last N/2 chars.

### 3.4 Capability-Based Tool Visibility

- **Practice:** Show the LLM only the tools/permissions relevant to the current task.
- **Evidence:** Claude Code's tool classification, Continue's context providers.
- **Why:** Reduces token usage. Improves tool selection accuracy by narrowing the decision space.
- **Recommendation:** Integrate with Phase 9A Skills engine — Skill definitions declare their tool requirements.

### 3.5 Hierarchical AbortController for Clean Cancellation

- **Practice:** Link tool cancelation to parent session cancellation. Propagate cancellation to all child processes.
- **Evidence:** Claude Code's hierarchical AbortController tree (child aborts when parent aborts, but not vice versa).
- **Why:** Single cancel call at the session level cascades to all in-flight operations. Prevents orphaned processes.
- **Recommendation:** Implement a tree of AbortController-like signals. Each tool call gets its own signal linked to the pane's signal.

### 3.6 Output Retention After Process Exit

- **Practice:** Keep process output available for a short window (5-30s) after the process exits.
- **Evidence:** Codex CLI's 30s retention with late-output capture. Prevents race between "process exited" and "last output bytes".
- **Why:** Output can arrive after the OS reports process exit (especially in sandboxed environments).
- **Recommendation:** 5s post-exit retention with an event log that supports replay.

### 3.7 Process Group Management

- **Practice:** Launch subprocesses in their own process group (`setpgid` on Unix). Kill the entire group, not just the parent.
- **Evidence:** Codex CLI uses `process_group::interrupt_process_group()`. Cline uses `process.kill(-childPid)`. Both kill descendant processes.
- **Why:** Shell commands spawn child processes (pipelines, background jobs). Killing only the parent leaves orphans.
- **Recommendation:** Use `detached: false` but set process group. Kill group on cancel/timeout.

### 3.8 Progress Throttle

- **Practice:** Don't stream output to the LLM more often than ~1-2 seconds. Batch small chunks.
- **Evidence:** Claude Code's `PROGRESS_THRESHOLD_MS = 2000` — progress UI only shown after 2s.
- **Why:** Prevents token waste from rapid small updates. LLMs don't need millisecond granularity.
- **Recommendation:** 500ms throttle for UI, 2s throttle for feeding back to the LLM as context.

### 3.9 Permission Caching with Session Scope

- **Practice:** Cache permission decisions for the session duration. Allow "always allow", "allow this session", "allow once".
- **Evidence:** Cline's `allowThisSession` mode. Continue's permission manager. Claude Code's rule-based auto-approve.
- **Why:** Reduces friction for repetitive operations. User trusts the tool within a session.
- **Recommendation:** Cache at pane level. Invalidate on pane close or project switch.

### 3.10 Structured Result with Metadata

- **Practice:** Return a structured result object, not just a string.
- **Evidence:** Codex CLI's `ReadResponse` (chunks, next_seq, exited, exit_code, closed, sandbox_denied). Claude Code's tool result with `interrupted`, `returnCodeInterpretation`, `persistedOutputPath`.
- **Why:** Enables the LLM and UI to make decisions based on exit codes, truncation, and sandbox denials.
- **Recommendation:** Return `{ stdout, stderr, exitCode, interrupted, truncated, sandboxDenied, timingMs, persistedPath }`.

---

## 4. Anti-Patterns

### 4.1 Output Buffer Overflow (No Truncation)
- **What:** Accumulating output in memory without limits.
- **Where:** Aider (unlimited), Continue (unlimited).
- **Fix:** Bounded buffer with middle truncation. Cline's approach is ideal.

### 4.2 Synchronous Subprocess Execution in Async Context
- **What:** Blocking the async runtime with `process.wait()` or `execSync()`.
- **Where:** Aider (synchronous `Popen.wait()`), Continue (some `execSync()` calls).
- **Fix:** Always use `tokio::process::Command` or equivalent async spawn. Use `spawn_blocking` only if unavoidable.

### 4.3 Single-Process Kill (Not Process Group)
- **What:** Killing only the parent PID, leaving child processes running.
- **Where:** Continue (`process.kill()` on single PID), Grok Build (SIGTERM on child only).
- **Fix:** Launch in own process group, kill group with negative PID.

### 4.4 Hard-Coded She'll Path
- **What:** Using `/bin/bash` without checking user's `$SHELL`.
- **Where:** Cline (`/bin/bash`), Grok Build (`sh`), Aider (`/bin/sh` fallback).
- **Fix:** Respect `$SHELL` env var, fall back to `/bin/sh`, then `bash`, then `sh`.

### 4.5 No Process Lifecycle Tracking
- **What:** Spawning a process and relying on the OS to clean it up.
- **Where:** Aider (no process map, no cleanup).
- **Fix:** Implement a process tracking map with Starting/Running/Exited states. Clean up on drop or explicit termination.

### 4.6 Ignoring Stderr (Silent Failures)
- **What:** Discarding stderr or merging it into stdout without markers.
- **Where:** Claude Code (merged streams, no separation), Aider (stdout redirect).
- **Fix:** Capture stderr separately. Expose it alongside stdout. Use `[stderr]` markers if merging.

### 4.7 Infinite Default Timeout
- **What:** No timeout on process execution.
- **Where:** Aider (no timeout), OpenCode (no timeout).
- **Fix:** Default 30s timeout. Expose as configurable parameter per execution profile.

### 4.8 Unbounded Process Concurrency
- **What:** Allowing unlimited concurrent subprocesses.
- **Where:** Continue (uncapped).
- **Fix:** Cap at a reasonable limit (8-10). Queue or reject beyond cap.

### 4.9 Permission Bypass via Environment Variable Injection
- **What:** Allowing env vars that change binary resolution (`LD_PRELOAD`, `PATH`, `NODE_OPTIONS`).
- **Where:** Most systems (inherit `process.env` unconditionally).
- **Fix:** Strip or restrict dangerous env vars. Codex CLI's process hardening is the gold standard.

### 4.10 Blocking First Token with Process Orchestration
- **What:** Starting subprocess orchestration before the first LLM token is delivered to the UI.
- **Where:** OpenCode (filesystem enrichment blocks streaming start).
- **Fix:** Defer all process orchestration to a background task. Stream the LLM response immediately.

---

## 5. Recommended CLIExecutionEngine Design

### 5.1 What Should CLIExecutionEngine Own?

The `CLIExecutionEngine` should be a **shared infrastructure component** with these responsibilities:

| Responsibility | Rationale |
|---|---|
| **Process lifecycle** (spawn → track → cleanup → reap) | Universal across all tool types |
| **Output streaming** (stdout/stderr capture, buffering, truncation) | Same algorithm for all commands |
| **Cancellation** (hierarchical AbortController, signal propagation, process group kill) | Must be consistent across all tools |
| **Timeout management** (configurable defaults, per-call overrides, grace window) | Shared safety mechanism |
| **Permission gating** (tool policy evaluation, sandbox enforcement) | Centralized security boundary |
| **Environment construction** (inherit policy, safe-strip dangerous vars, merge overrides) | Single source of truth |
| **Working directory management** (resolve, validate, pass to process) | Consistency across all commands |
| **Process group management** (create group, kill group on cancel) | Prevents orphaned processes |
| **Output retention** (post-exit retention window, replay buffer) | Covers race conditions |
| **Structured result formatting** (exit code, truncated flag, metadata) | Consistent LLM interface |
| **Resource limits** (max concurrency, max output size, max runtime) | System-wide guardrails |

### 5.2 What Should Individual Engines Own?

| Responsibility | Owner | Rationale |
|---|---|---|
| **Command construction** (what to run, arguments) | Individual tool/skill | Unique to each tool |
| **Shell selection** (bash, python, node, direct exec) | Individual engine | Depends on tool requirements |
| **Input preparation** (stdin content, arg files) | Individual engine | Tool-specific |
| **Result interpretation** (exit code semantics, output parsing) | Individual engine | Tool-specific meaning |
| **Error recovery strategy** (retry, fallback, failure message) | Individual engine | Depends on tool semantics |
| **Permission policy** (what requires approval) | Skill manifest + user config | Configured per-skill |
| **Progress formatting** (how to display to user) | UI layer | Presentation concern |

### 5.3 Reusable Process-Management Abstractions

These abstractions should be implemented once in `CLIExecutionEngine` and reused by all tool/skill engines:

```rust
// ──── PROCESS HANDLE ────
// A managed handle to a running subprocess.
// Owns: OS child handle, stdout/stderr readers, timeout, cleanup.
// Behavior: pipes output through bounded channels, kills on drop.
pub struct ManagedProcess {
    id: ProcessId,
    pid: u32,
    stdout_rx: mpsc::Receiver<OutputChunk>,
    stderr_rx: mpsc::Receiver<OutputChunk>,
    exit_rx: oneshot::Receiver<ProcessExit>,
    kill: Box<dyn FnOnce() + Send>,
    cancellation_token: CancellationToken,
}

// ──── OUTPUT ACCUMULATOR ────
// Bounded rolling output buffer with middle truncation.
// Capacity-based eviction (not line-based).
pub struct RollingOutputAccumulator {
    head: Vec<u8>,     // First half of budget
    tail: Vec<u8>,     // Last half of budget
    total: usize,      // Total bytes received
    capacity: usize,   // Max bytes to retain
    truncated: bool,   // Was output truncated?
}
// Strategy: push all to head until head reaches capacity/2,
// then drain head to tail (keeping last capacity/2).
// On finalize: head + "[... truncated N bytes ...]" + tail

// ──── PROCESS TRACKER ────
// Concurrent map of running processes with lifecycle states.
pub struct ProcessTracker {
    processes: DashMap<ProcessId, ProcessState>,
    max_concurrency: usize,
}

enum ProcessState {
    Starting(CancellationToken),
    Running(Arc<ManagedProcess>),
    Draining { /* post-exit retention */ exited: ProcessExit, output: RollingOutputAccumulator },
}

// ──── KILL STRATEGY ────
pub enum KillStrategy {
    SigtermThenSigkill { grace_ms: u64 },  // default: 3000ms
    Sigkill,                                // immediate
    ProcessGroup { grace_ms: u64 },         // kill entire PGID
}

// ──── CANCELLATION TOKEN ────
// Hierarchical: child tokens are linked to parent.
// Cancelling parent cascades to all children.
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
    children: Vec<Arc<CancellationToken>>,
}

// ──── STDIO CONFIG ────
pub struct StdioConfig {
    pub combine_output: bool,         // Merge stderr into stdout?
    pub max_output_chars: usize,      // Rolling collector capacity (default 48K)
    pub input_encoding: String,       // stdin encoding
    pub output_encoding: String,      // stdout/stderr encoding
}

// ──── ENV POLICY ────
pub struct EnvPolicy {
    pub inherit: InheritMode,         // None | SetOnly | Full
    pub exclude: Vec<String>,          // Keep these out
    pub include_only: Option<Vec<String>>, // Only these (if set)
    pub set: HashMap<String, String>,  // Overrides
    pub safe_strip: Vec<String>,       // Dangerous vars to strip (LD_PRELOAD, etc.)
}

enum InheritMode {
    None,          // Start with empty env
    SetOnly,       // Only inherit vars explicitly listed
    Full,          // Inherit all, then filter
}

// ──── EXIT RESULT ────
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub interrupted: bool,
    pub truncated: bool,
    pub sandbox_denied: bool,
    pub timing_ms: u64,
    pub persisted_output_path: Option<String>,
    pub exit_signal: Option<String>,
}
```

### 5.4 How BuilderBoard Should Normalize

#### stdout / stderr

| Concern | Normalization |
|---|---|
| **Separation** | Keep separate by default. Provide `combine_output: bool` option. |
| **Encoding** | Always capture as bytes. Convert to string using platform encoding (UTF-8, detect charset). |
| **Binary detection** | Check for null bytes. If binary, hex-encode or skip with warning. |
| **Buffering** | Bounded rolling accumulator (48K default, configurable per-execution profile). |
| **Stderr in combined mode** | Append `\n[stderr]\n` before stderr content. |
| **Large output** | Persist to temp file. Return `<persisted-output>` reference in result. |

#### exit codes

| Concern | Normalization |
|---|---|
| **0** | Success. `exit_code = 0`, `error = null`. |
| **Non-zero** | `exit_code = N`, `error = format("Command exited with code {N}")`. |
| **Signal termination** | Detect via `WIFSIGNALED`. Store `exit_signal` name (e.g., "SIGKILL", "SIGTERM"). |
| **Timeout** | Synthetic exit code 124 (timeout convention). `interrupted = true`. |
| **Cancellation** | Synthetic exit code 130 (SIGINT convention). `interrupted = true`. |
| **Sandbox denial** | Detect via output patterns. `sandbox_denied = true`. |

#### cancellation / signals

| Concern | Normalization |
|---|---|
| **User cancel (Ctrl+C)** | `CancellationToken.abort()` → cascade to children → SIGTERM → 3s → SIGKILL. |
| **Timeout** | Same kill strategy as user cancel. Include timeout duration in result metadata. |
| **Parent abort** | Pane abort cascades to all active tool executions. |
| **Double cancel** | Second cancel within 1s: skip grace window, SIGKILL immediately. |
| **Background conversion** | Support converting foreground process to background (detach + unref). |

#### JSON events

| Concern | Normalization |
|---|---|
| **Process lifecycle events** | Emit typed events: `process.started`, `process.output`, `process.exited`, `process.failed`, `process.cancelled`. |
| **Delta vs. snapshot** | `process.output` events are deltas. Result includes snapshot. |
| **Format** | Tagged union: `{ type: "process.started", pid, command, cwd }`. |
| **Stream binding** | Each process gets a `process_id` bound to the tool call `tool_call_id`. |
| **Rate limiting** | Throttle `process.output` events to 100ms intervals. Aggregate chunks. |

#### progress

| Concern | Normalization |
|---|---|
| **Initial threshold** | Don't emit progress in first 500ms (avoid noise for fast commands). |
| **Throttle** | Emit at most every 200ms for UI, every 2s for LLM context. |
| **Content** | Include: elapsed time, total bytes, truncation status. |
| **Completion** | Final event includes full result, exit code, timing. |
| **Background tasks** | Progress emits via task ID, not tool call ID. |

### 5.5 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                      CLIExecutionEngine                              │
│                                                                      │
│  ┌─────────────────────┐   ┌──────────────────────────────────────┐ │
│  │  ProcessTracker      │   │  PermissionGate                      │ │
│  │  ┌─────────────────┐ │   │  ┌─────────────────────┐             │ │
│  │  │ ProcessEntry 1  │ │   │  │ PatternMatcher      │             │ │
│  │  │ - ManagedProcess│ │   │  │ - exact/prefix/wild │             │ │
│  │  │ - RollingOutput │ │   │  │ - path constraints  │             │ │
│  │  │ - KillStrategy  │ │   │  │ - env var stripping │             │ │
│  │  │ - Timeout       │ │   │  └─────────┬───────────┘             │ │
│  │  └─────────────────┘ │   │            │                          │ │
│  │  ┌─────────────────┐ │   │            ▼                          │ │
│  │  │ ProcessEntry 2  │ │   │  ┌─────────────────────┐             │ │
│  │  │ ...             │ │   │  │ Decision (allow/     │             │ │
│  │  └─────────────────┘ │   │  │  deny/ask)          │             │ │
│  │  Max: 8 concurrent   │   │  └─────────────────────┘             │ │
│  └──────────┬───────────┘   └──────────────────────────────────────┘ │
│             │                                                         │
│             ▼                                                         │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                 Process Launcher                              │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐ │   │
│  │  │ Pipe     │  │ PTY      │  │ Detached │  │ Sandbox      │ │   │
│  │  │ Launcher │  │ Launcher │  │ Launcher │  │ Wrapper      │ │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────────┘ │   │
│  └──────────────────────────────────────────────────────────────┘   │
│             │                                                         │
│             ▼                                                         │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                 Event Emitter                                  │   │
│  │  process.started → process.output → process.exited           │   │
│  │  + progress throttle (200ms UI / 2s LLM)                     │   │
│  │  + output accumulated in RollingOutputAccumulator             │   │
│  └──────────────────────────────────────────────────────────────┘   │
│             │                                                         │
│             ▼                                                         │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Structured Result Builder                        │   │
│  │  CommandResult { stdout, stderr, exit_code, interrupted,     │   │
│  │    truncated, sandbox_denied, timing_ms, persisted_path }    │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
             │                    ▲
             │                    │
             ▼                    │
┌────────────────────────┐   ┌──────────────────────────────┐
│ Individual Skill/Tool  │   │  Event Bus / UI              │
│ Engine                  │   │  (Tauri events, SSE)        │
│ - constructs command   │   │  - processes events          │
│ - interprets result    │   │  - displays progress          │
│ - selects kill strategy│   │  - feeds to LLM as context   │
└────────────────────────┘   └──────────────────────────────┘
```

### 5.6 Ownership Boundaries

```
CLIExecutionEngine owns:                  Individual Engines own:
─────────────────────────                ─────────────────────
Process lifecycle                        Command construction
Output streaming                         Shell selection
Cancellation propagation                 Input preparation
Timeout management                       Result interpretation
Permission gating                        Error recovery strategy
Environment construction                 Permission policy (via skill manifest)
Working directory mgmt
Process group mgmt
Output retention
Structured result formatting
Resource limits
```

### 5.7 Integration with Existing Architecture

The `CLIExecutionEngine` plugs into the existing `ExecutionEngine` trait:

```rust
// New execution engine variant for CLI/command execution
pub struct CLIExecutionEngine {
    tracker: ProcessTracker,
    permission_gate: PermissionGate,
    event_tx: mpsc::UnboundedSender<ExecutionEvent>,
    max_concurrency: usize,
}

impl ExecutionEngine for CLIExecutionEngine {
    fn execute(&self, context: ExecutionContext, event_callback: ...) -> ... {
        // 1. Validate command against permission gate
        // 2. Spawn process via ProcessTracker
        // 3. Stream output via event_callback
        // 4. Collect result
        // 5. Return structured CommandResult
    }
}
```

This replaces the current empty `ExecutionPolicy` and unwired `AtomicBool` cancellation with real process management. The `ExecutionEngine` registry (in `execution/engine.rs`) can have both `OpenAIExecutionEngine` (for LLM calls) and `CLIExecutionEngine` (for subprocess execution) registered alongside each other.

---

## 6. Grok-Specific Considerations

### 6.1 Stateful CWD is Dangerous

Grok Build's `BashTool` maintains a **stateful CWD** that persists across tool calls via `cd` interception:

```typescript
// Grok Build (simplified from bash.ts)
if (command.startsWith("cd ")) {
    this.cwd = path.resolve(this.cwd, dir); // ← state mutation
    return { success: true, output: `Changed directory to: ${this.cwd}` };
}
```

**Danger:** This means a single `cd` in one turn permanently changes the working directory for all subsequent turns. The LLM model can "forget" its current location across turns, leading to unexpected behavior.

**Recommendation for BuilderBoard:** Do NOT implement stateful CWD. Each tool call should receive the CWD as a parameter. If the user wants to change directory, the frontend should update the pane's project scope, not let the model mutate execution state.

### 6.2 `exec()` vs `spawn()` Pattern

Grok Build uses `exec()` for foreground (collects all output into a buffer) and `spawn()` for background (streams to file). This is a reasonable simplification, but the `exec()` approach means:
- **No streaming output** for foreground commands
- **10MB hard buffer limit** — uncontrolled commands can hit this
- **No intermediate progress** — the LLM sees nothing until the command completes

**Recommendation:** Always use `spawn()` with streaming output. Use `exec()` only for very short commands (< 1s expected runtime).

### 6.3 Subagent Delegation Architecture

Grok Build spawns subagents as entirely new Node.js processes:
```typescript
const child = spawn(process.execPath, ["--background-task-file", jobPath], {
    detached: true,
    stdio: "ignore",
    env: { ...process.env, GROK_BACKGROUND_CHILD: "1" },
});
child.unref();
```

This is an elegant pattern for isolation — subagents can't affect the parent's state, survive parent crashes, and are independently monitorable. However:
- **Process overhead:** Each subagent is a full Node.js runtime (~40MB+ RAM)
- **File-based IPC:** State is passed through JSON files on disk (latency, no streaming)
- **No stdout/stderr:** `stdio: "ignore"` means all output goes to files

**Recommendation for BuilderBoard:** Reuse this pattern only for true isolation needs (Phase 9C+). For Phase 9B, use in-process Skills with shared but isolated execution contexts. When we do need subagent isolation, use the same detached-process pattern but with pipe-based IPC for streaming.

### 6.4 Sandbox Profiles

Grok Build's sandbox profiles (`workspace`, `devbox`, `read-only`, `strict`) are a clean abstraction:

| Profile | Filesystem | Network | Use Case |
|---|---|---|---|
| `off` | Full access | Full | Trusted commands |
| `workspace` | CWD + temp + config dirs | Full | Normal development |
| `devbox` | Full except `/data` | Full | Controlled environment |
| `read-only` | Read everywhere, write temp+config | Blocked | Code review |
| `strict` | CWD + system paths only | Blocked | Untrusted execution |

All profiles block: `~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.config/gcloud`, `~/.azure`

**Recommendation:** Adopt this profile model. Start with `workspace` and `read-only` in Phase 9B. Add `strict` when sandboxing lands.

### 6.5 ACP (Agent Client Protocol)

Grok Build's ACP protocol is a JSON-RPC 2.0 protocol over stdin/stdout for headless execution:
- `session/create` → `session/update` (streaming events) → `session/cancel` → `session/destroy`
- Events: `messageChunk`, `thoughtChunk`, `toolCall`, `toolCallUpdate`, `mediaContent`

**Recommendation:** BuilderBoard should avoid defining a new protocol for internal use. Instead, the existing `ExecutionEvent` enum + Tauri events are sufficient. If an external protocol is needed in the future (Phase 9C+ for multi-process orchestration), adopt ACP or MCP rather than inventing a new one.

---

## 7. Claude/OpenCode Considerations

### 7.1 Claude Code Permission System (Gold Standard)

Claude Code's permission system is the most sophisticated across all researched systems. Key features to adopt:

| Feature | Why It Matters | Priority |
|---|---|---|
| **Four permission modes** (`default`/`acceptEdits`/`plan`/`bypassPermissions`/`auto`) | Covers the full spectrum from safety to speed | High |
| **Hierarchical rule matching** (exact → prefix → wildcard → path → sed → classifier) | Granular control without verbosity | High |
| **SAFE_ENV_VARS stripping before matching** | Prevents permission bypass via env vars | Medium |
| **read-only mode with classifier** | Auto-detection of safe commands | Medium |
| **Hook system for pre/post tool** | Extensibility without engine changes | Low (Phase 9C) |

**Recommendation:** Implement the four permission modes and hierarchical rule matching in Phase 9B. The rule syntax should match Claude Code's for user familiarity:
```json
{
  "allow": ["Bash(git *)", "Edit(src/**)"],
  "deny": ["Bash(rm *)", "Read(.env*)"]
}
```

### 7.2 Claude Code's Concurrency Classification

Claude Code classifies tools as **concurrency-safe** (read-only) or **non-concurrency-safe** (write):

| Category | Examples | Behavior |
|---|---|---|
| Concurrency-safe | `FileReadTool`, `GrepTool`, `GlobTool` | Can execute simultaneously |
| Non-concurrency-safe | `FileEditTool`, `BashTool` | Execute exclusively, queue behind others |

**Recommendation:** Integrate this with the Skills engine. Skill manifests should declare `concurrency_safe: bool` in their tool definitions. The `CLIExecutionEngine`'s `ProcessTracker` should enforce this with a queue.

### 7.3 Claude Code's AbortController Hierarchy

Claude Code's linked AbortController tree is the cleanest cancellation model:
- **Parent** → **Child** (parent abort cascades to child)
- **Child** → **Parent** (child abort does NOT affect parent)
- Each tool call gets its own child controller linked to the session controller

**Recommendation:** Implement this pattern in `CancellationToken`. The pane runtime owns the root token. Each skill execution creates a child. Each tool call within that execution creates a grandchild.

### 7.4 OpenCode's Current State (Baseline)

| Area | Current State | Gap vs. Best Practice |
|---|---|---|
| **Subprocess execution** | None (no `std::process::Command` infrastructure) | Full gap — needs construction |
| **Cancellation** | `AtomicBool` declared, unwired; `Cancelled` event variant exists, never emitted | Needs real wiring to process lifecycle |
| **Working directory** | None (filesystem uses absolute paths via `ApprovedScope`) | Needs `cwd` parameter in execution context |
| **Environment variables** | `HashMap<String, String>` field, always empty | Needs policy-based construction |
| **Permissions** | `ExecutionPolicy` struct, always default (false/None) | Needs rule engine + approval flow |
| **Process tracking** | None | Needs `ProcessTracker` component |
| **Output streaming** | HTTP SSE only (for LLM responses) | Needs pipe-based subprocess streaming |
| **Event model** | `ExecutionEvent` enum with `serde(tag = "type")` | Already well-structured — extend with process event variants |
| **Background workers** | `StreamPersistenceService` (mpsc channel + thread) | Good pattern — reuse for process management |

**Key insight:** The existing `ExecutionEngine` trait, `ExecutionEvent` enum, and event streaming infrastructure are already well-designed for extension. The `CLIExecutionEngine` is a natural addition — it implements the same trait, emits the same event types (plus new process-specific ones), and integrates with the same `StreamWriteBuffer` + Tauri event system.

### 7.5 Recommended Phase 9B Integration

```
Phase 9B Implementation Order:

Week 1: Foundation
├─ CancellationToken (hierarchical, linked to pane)
├─ CLIExecutionEngine struct + trait implementation
├─ ProcessTracker (concurrent map, lifecycle states)
└─ RollingOutputAccumulator (bounded, middle-truncation)

Week 2: Core Execution
├─ PipeProcessLauncher (tokio::process::Command)
├─ KillStrategy (SIGTERM → grace → SIGKILL)
├─ EnvPolicy (inherit + safe-strip + overrides)
└─ ProgressEmitter (throttle: 200ms UI / 2s LLM)

Week 3: Permissions & Safety
├─ PermissionGate (4 modes + rule matching)
├─ Permission rule parser (JSON config)
├─ Approval flow (ask/deny/allow this session)
└─ Sandbox profile selection (workspace/read-only)

Week 4: Integration
├─ Wire CLIExecutionEngine into stream_execution.rs
├─ Replace unwired AtomicBool with CancellationToken
├─ Wire PermissionGate into ExecutionContext
└─ Extend ExecutionEvent with process variants

Week 5: Testing & Polish
├─ Unit tests: rolling collector, kill strategy, env policy
├─ Integration tests: spawn → stream → cancel → cleanup
├─ Performance benchmarks: latency, memory, concurrency
└─ Documentation: CLIExecutionEngine API reference
```

---

## 8. Confidence Score Rationale

**Score: 90/100**

| Factor | Score | Reasoning |
|---|---|---|
| Research coverage | 95/100 | All 7 major CLI coding systems analyzed in depth. Source-level analysis of Codex CLI, Cline, Aider, and Grok Build. Detailed second-hand analysis of Claude Code (leaked source), Continue (open source), and OpenCode (own codebase). |
| Pattern confidence | 92/100 | Common patterns are extremely consistent across all systems. The kill strategy, output buffering, process tracking, and permission systems follow nearly identical patterns. Low risk of missing something fundamental. |
| Architectural fit | 88/100 | Recommended architecture aligns with existing OpenCode patterns (ExecutionEngine trait, ExecutionEvent enum, streaming pipeline). Requires building from scratch for subprocess support but reuses existing event infrastructure. |
| Risk assessment | 87/100 | Main risk: building a PTY launcher is complex (interactive process support). Mitigation: start with pipe-only in Phase 9B, add PTY in Phase 9C if needed. |
| Normalization design | 90/100 | stdout/stderr/exit code/signal/cancellation normalization is well-supported by existing patterns across systems. The rolling output collector has three proven implementations to draw from. |
| Alignment with Phase 9A | 90/100 | CLIExecutionEngine cleanly maps onto the Skills architecture as a shared capability. Skill manifests can declare `requires_cli: bool` to enable it. Permission rules integrate with the trust tier model. |

**Key uncertainties lowering the score:**
- No system implements a fully generalized CLIExecutionEngine — they all embed process management in tool-specific code. This means the abstraction boundaries are unproven at scale. Mitigation: keep the design modular; individual engines can bypass the shared engine if needed.
- PTY support (for interactive commands) is significantly more complex than pipe support. Most systems don't need it. BuilderBoard may never need it either. Score will increase if we confirm PTY is out of scope for Phase 9B.
- Permission rule interaction with Skills trust tiers is undefined. The current design assumes rules are evaluated independently. If trust tiers and rules conflict, precedence needs definition.

---

## Next Steps

1. **Review this architecture document** — Decision: proceed to Phase 9B CLIExecutionEngine implementation.
2. **Phase 9B CLI implementation** (estimated 5 weeks):
   - Weeks 1-2: Foundation (CancellationToken, ProcessTracker, RollingOutputAccumulator, Pipe launcher, KillStrategy)
   - Week 3: Permissions (PermissionGate, rule parser, approval flow)
   - Week 4: Integration (wire into stream_execution, replace unwired cancellations, extend events)
   - Week 5: Testing (unit, integration, performance)
3. **Phase 9C** (multi-pane orchestration, Builder system, cross-pane event bus, detached subagents).
