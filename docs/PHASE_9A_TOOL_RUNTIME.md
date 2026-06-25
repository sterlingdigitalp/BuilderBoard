# Phase 9A — Tool Runtime v1

**Status**: DELIVERED
**Confidence**: 90/100
**Lines of code**: ~1250 (8 tool modules + infrastructure)
**Tests**: All 104 existing pass (no regressions)
**TypeScript**: Clean `tsc --noEmit`

## Architecture

```
ExecutionRequest
  → ExecutionManager (resolve → engine + policy)
    → ExecutionEngine (agentic loop)
      → ToolRegistry.get("tool_id")
        → Tool::execute(ctx, args, on_event)
          → ExecutionEvent (ToolStarted, ToolOutput, ToolFinished, ToolFailed, ReviewItemCreated, TimelineEntry, PermissionCheck)
            → frontend / review queue / timeline
```

### Key Design Decisions

1. **Tools are engine-independent** — the `Tool` trait has no reference to any engine type. Engines invoke tools, not vice versa.
2. **Everything flows through ExecutionManager** — tool requests are resolved via the existing `resolve()` so policy checks are applied before any tool runs.
3. **Permissions are declared per-tool** — each tool lists its required `ToolPermission` variants. `ExecutionPolicy` (extended with `allow_read`, `allow_write`, `allow_delete`, `allow_git`, `allow_packages`, `allow_processes`) gates execution at runtime.
4. **Mutating tools generate ReviewItems** — shell, write, edit, delete, commit, kill, package install/uninstall, directory create all emit `ReviewItemCreated` events.
5. **No engine redesign** — `ExecutionEngine` trait, `ExecutionManager`, and `BuilderRegistry` are unchanged (extended only `ExecutionPolicy` with new fields and `ExecutionEvent` with new variants).

## Modules

| Module | File | Tools | Permissions |
|--------|------|-------|-------------|
| `execution/tools/` | `mod.rs` | Module root, re-exports | — |
| `traits.rs` | Core Tool trait + ToolId | — |
| `registry.rs` | ToolRegistry, `register_default_tools()`, `global_tool_registry()` | — |
| `permissions.rs` | `ToolPermission` enum (8 categories) + `PermissionLevel` | — |
| `context.rs` | `ToolContext` (exec_id, pane_id, scope, cwd, env, cancellation, policy flags) | — |
| `results.rs` | `ToolResult`, `ToolOutput`, `ToolArtifact`, `ReviewItem`, `ToolDescriptor` | — |
| `shell.rs` | `ShellTool` | Shell |
| `filesystem.rs` | `ReadTool`, `WriteTool`, `EditTool`, `DeleteTool` | ReadFiles, WriteFiles, DeleteFiles |
| `directory.rs` | `ListTool`, `CreateTool` | ReadFiles, WriteFiles |
| `package.rs` | `InstallTool`, `UninstallTool`, `ListTool` | Packages |
| `git.rs` | `StatusTool`, `DiffTool`, `CommitTool`, `LogTool` | Git |
| `process.rs` | `ListTool`, `KillTool` | Processes |
| `search.rs` | `GrepTool`, `GlobTool` | ReadFiles |
| `diagnostics.rs` | `HealthTool`, `EnvTool` | (none) |

## Event Extensions

New `ExecutionEvent` variants added to `event.rs`:

- `ToolStarted { tool_id, execution_id, args }` — tool begins execution
- `ToolOutput { tool_id, execution_id, channel, content }` — incremental output (stdout/stderr/result)
- `ToolFinished { tool_id, execution_id, summary }` — clean completion
- `ToolFailed { tool_id, execution_id, code, message }` — failed execution
- `PermissionCheck { tool_id, permission, allowed, reason }` — permission gate event
- `ReviewItemCreated { tool_id, execution_id, action, summary, details }` — review queue entry
- `TimelineEntry { execution_id, phase, tool_id, summary }` — timeline entry

## Policy Extensions

New fields on `ExecutionPolicy` in `context.rs`:
- `allow_read: bool`
- `allow_write: bool`
- `allow_delete: bool`
- `allow_git: bool`
- `allow_packages: bool`
- `allow_processes: bool`

## Registration

`register_default_tools()` is called during app startup in `storage/mod.rs` setup. Registers 20 tools across 8 categories.

## Usage Example

```rust
let registry = global_tool_registry();
let registry = registry.read().unwrap();
if let Some(tool) = registry.get("filesystem.read") {
    let ctx = ToolContext::local("exec-123");
    let args = serde_json::json!({"path": "./src/main.rs"});
    let result = tool.execute(ctx, args, &|event| {
        match event {
            ExecutionEvent::ToolOutput { content, .. } => println!("{}", content),
            ExecutionEvent::ToolFinished { .. } => println!("Done!"),
            _ => {}
        }
    });
}
```

## Next Steps (Phase 9B)

- Skills framework: each Skill describes its tools via BUILDER.yaml, the ToolRegistry maps intent → tool
- Autonomous execution: ExecutionClass::Autonomous for background tool chains
- ReviewQueue panel in the frontend to approve/reject ReviewItems
- Timeline visualization of tool execution history
