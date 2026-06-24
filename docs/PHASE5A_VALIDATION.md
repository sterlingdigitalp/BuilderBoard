# Phase 5A — Workspace Architecture Validation

**Date:** 2026-06-23  
**Scope:** Audit and validate workspace isolation in the existing persistence layer. No new features (no `workspace_create`, `workspace_switch`, OAuth, provider execution, or exports).

## Validation Questions

| Question | Expected | Result |
|----------|----------|--------|
| Can Workspace A see Workspace B panes via `pane_list`? | No | **Pass** — `PaneRepository::list_open` filters `WHERE workspace_id = ?1` |
| Can Workspace A see Workspace B messages via normal workspace flow? | No | **Pass** — messages are loaded per `pane_id`; workspace B pane IDs never appear in Workspace A `pane_list` |
| Does switching workspace queries alter accounts? | No | **Pass** — `accounts` has no `workspace_id`; global table |
| Does switching workspace queries alter credentials? | No | **Pass** — Keychain entries keyed by `credential_ref`, independent of workspace |
| Does switching workspace queries alter provider bindings globally? | No | **Pass** — bindings stored per pane row; each workspace owns its pane set |

## Subagent Reviews

### Subagent A — Workspace Schema Review

**Files:** `migrations/0001_initial_schema.sql`, `src-tauri/src/storage/repositories/workspaces.rs`

| Check | Status | Notes |
|-------|--------|-------|
| `workspaces` table with `is_default`, `archived_at` | Pass | Default workspace seeded at `00000000-0000-4000-8000-000000000001` |
| `panes.workspace_id` FK → `workspaces(id)` ON DELETE CASCADE | Pass | Pane lifecycle tied to workspace |
| `messages.workspace_id` FK → `workspaces(id)` ON DELETE CASCADE | Pass | Denormalized for workspace-scoped indexes |
| `accounts` global (no `workspace_id`) | Pass | By design — accounts shared across workspaces |
| Indexes support workspace-scoped pane/message queries | Pass | `idx_panes_workspace_order`, `idx_messages_workspace_created` |

**Finding:** Schema supports multi-workspace isolation at the data layer. Management commands (`workspace_create`, `workspace_list`, `workspace_switch`) are Phase 5 deliverables not yet implemented.

### Subagent B — Workspace Query Isolation Review

**Files:** `panes.rs`, `messages.rs`, `commands.rs`

| Command / Query | Workspace guard | Status |
|-----------------|-----------------|--------|
| `pane_list(workspace_id?)` | Yes — resolves and filters by `workspace_id` | Pass |
| `pane_create(workspace_id?)` | Yes — resolves workspace before insert | Pass |
| `message_list(pane_id)` | Indirect — requires open pane; no cross-workspace pane discovery | Pass (with caveat) |
| `message_append` / `message_create` | Yes — `workspace_id` copied from pane row | Pass |
| `pane_close(pane_id)` | No workspace param | Acceptable — pane IDs are globally unique UUIDs |

**Caveat:** `message_list` accepts any valid open `pane_id` without re-checking caller workspace context. Cross-workspace message read is possible only if a caller already knows another workspace's pane UUID. The UI path does not expose foreign pane IDs when `pane_list` is workspace-scoped.

### Subagent C — Restart Persistence Review

**Files:** `db.rs`, `storage/mod.rs` integration tests, `tests/workspace_isolation.rs`

| Check | Status | Notes |
|-------|--------|-------|
| Default workspace survives `Database::initialize_at` reopen | Pass | `verify_seeds` + migration `INSERT OR IGNORE` |
| Per-workspace pane counts survive reopen | Pass | 3 panes in A, 2 in B after reopen |
| Per-pane messages survive reopen | Pass | One message per pane verified |
| Active workspace preference persisted | N/A | `workspace_switch` not implemented; UI always calls `pane_list` without `workspace_id` (defaults to seeded workspace) |

### Subagent D — Cross-Workspace Contamination Review

**Scenarios exercised in `tests/workspace_isolation.rs`:**

1. Workspace A: 3 panes + messages; Workspace B: 2 panes + messages
2. Alternate `pane_list` queries 20 times — stable, disjoint pane sets
3. Close pane in B — A pane count unchanged (3)
4. Create shared account, bind to panes in both workspaces — account list unchanged, credentials intact
5. Reopen database file — all workspace data restored

**Contamination vectors assessed:**

| Vector | Leakage? |
|--------|----------|
| `pane_list` default vs explicit workspace | No |
| Message content via workspace-scoped pane discovery | No |
| Pane close in B affecting A open list | No |
| Account/credential mutation on workspace query | No |
| Direct `pane_id` message read (out-of-band UUID) | Theoretical — not reachable from workspace-scoped UI |

## Validation Scenarios (Executed)

```
Workspace A (default): 3 panes, 1 message each
Workspace B (inserted in test): 2 panes, 1 message each
→ Alternate pane_list(A) / pane_list(B): PASS
→ Close B pane: A unaffected: PASS
→ Reopen DB: counts + messages preserved: PASS
→ Shared account across workspaces: PASS
```

## Frontend Gap (Documented, Not Fixed in 5A)

`src/hooks/usePersistentPanes.ts` calls `paneList()` without `workspace_id`, so production UI always loads the default workspace. Multi-workspace UI is Phase 5A implementation work, outside this validation pass.

## Remaining Risks

| Risk | Severity | Mitigation path |
|------|----------|-----------------|
| No `workspace_switch` / active workspace persistence | Medium | Phase 5A implementation |
| `message_list` lacks explicit workspace guard | Low | Add optional `workspace_id` check when switch UI lands |
| UI hardcoded to default workspace | Medium | Pass `workspace_id` from persisted active workspace |
| Direct pane UUID access bypasses workspace context | Low | Document; add workspace membership check on sensitive commands |

## Test Coverage

Integration tests: `src-tauri/tests/workspace_isolation.rs`

- `pane_list_is_workspace_scoped`
- `messages_do_not_leak_across_workspaces`
- `repeated_workspace_queries_remain_stable`
- `accounts_and_credentials_unaffected_by_workspace_operations`
- `pane_close_in_one_workspace_does_not_affect_other`
- `workspace_data_persists_across_database_reopen`

Existing restart tests in `src-tauri/src/storage/mod.rs` remain valid for single-workspace persistence.

## Verdict

**Phase 5A architecture validation: PASS**

The persistence layer enforces workspace isolation for pane listing and message creation. Accounts and credentials are correctly global and unaffected by workspace-scoped operations. Restart persistence holds for per-workspace pane and message data. Management commands and UI workspace switching remain future implementation work.