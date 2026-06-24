# Phase 5B — OAuth & Pane Settings Architecture Validation

**Date:** 2026-06-23  
**Scope:** Audit OpenAI OAuth readiness, pane-level model/reasoning persistence, workspace interaction, and account isolation. No implementation code.

## Executive Summary

| Area | Verdict | Notes |
|------|---------|-------|
| OpenAI OAuth lifecycle | **Not implemented** | OpenAI uses API-key auth by design in v1 |
| OAuth infrastructure (Google) | **Pass** | PKCE, callback, Keychain storage, refresh validated by unit tests |
| Per-pane model persistence | **Partial** | `panes.model_id` exists; UI selection is in-memory; no `pane_update` command |
| Per-pane reasoning persistence | **Not implemented** | Types defined in `paneSettings.ts` only |
| Workspace pane-settings isolation | **Pass (architecture)** | Settings live on per-workspace pane rows; Phase 5A isolation holds |
| Account isolation | **Pass** | Accounts/credentials global; pane bindings are per-pane, not per-workspace |

## Validation Questions

| Question | Expected | Result |
|----------|----------|--------|
| Can user log in via OpenAI OAuth? | Flow works end-to-end | **N/A** — OpenAI `auth_mode = api_key`; `oauth_start("openai")` rejected |
| Do pane models persist independently (GPT-5.5 / 5.4 mini / Codex Spark)? | Yes, no leakage | **Fail** — types exist; no persistence command or UI wiring |
| Do reasoning levels persist independently (Low/Medium/High/XHigh)? | Yes | **Fail** — no schema field, command, or UI |
| Do workspace A/B pane settings contaminate each other? | No | **Pass (architecture)** — `panes.workspace_id` scopes rows; no shared settings store |
| Does workspace switching alter accounts/credentials? | No | **Pass** — confirmed in Phase 5A |

---

## Subagent A — OpenAI OAuth Audit

**Scope note:** BuilderBoard v1 implements OAuth for **Google only**. OpenAI OAuth is explicitly out of scope per [OAUTH_DESIGN.md](./OAUTH_DESIGN.md) and [PHASE3B_IMPLEMENTATION.md](./PHASE3B_IMPLEMENTATION.md). This audit validates the **OAuth architecture** against the requested lifecycle checks and documents the OpenAI gap.

### OpenAI Provider Configuration

| Check | Status | Evidence |
|-------|--------|----------|
| OpenAI `auth_mode` | `api_key` | `migrations/0001_initial_schema.sql` provider seed |
| OpenAI `oauth_config_json` | NULL | No OAuth endpoints configured |
| `OAUTH_SUPPORTED_PROVIDERS` | `["google"]` only | `accounts.rs` |
| UI OAuth connect for OpenAI | Absent | `AccountProviderSection` — Google button only |
| OpenAI account path | API key | `account_create_api_key` via `AccountCreateForm` |

**`oauth_start("openai")` behavior:** `OAuthService::validate_oauth_provider` returns `InvalidInput: provider openai does not support OAuth in Phase 3B`.

### OAuth Lifecycle Audit (Google — Reference Implementation)

The following validates the generic OAuth lifecycle that would apply if OpenAI OAuth were added later.

| Lifecycle step | Status | Implementation | Test |
|----------------|--------|----------------|------|
| **PKCE** | Pass | S256 challenge from verifier; verifier sent at token exchange | `pkce_challenge_uses_s256` |
| **Callback** | Pass | Loopback `127.0.0.1:<ephemeral>/callback`; state + code parsed from HTTP GET | `google_oauth_flow_completes_with_callback` |
| **State validation** | Pass | Callback `state` must match pending session | `oauth_rejects_state_mismatch` |
| **Token storage** | Pass | Keychain JSON via `CredentialService::store_oauth_credential`; SQLite holds `credential_ref` only | Flow test + `oauth_disconnect_removes_keychain_entry` |
| **Refresh flow** | Pass | `refresh_oauth_access_token` when `expires_at < now + 5min`; preserves refresh token | `oauth_refresh_updates_keychain_and_account` |
| **Cancel** | Pass | `oauth_cancel` sets cancel flag | `oauth_cancel_emits_cancelled_error` |
| **System browser** | Pass | macOS `open` — no embedded WebView | Documented in PHASE3B |

### OpenAI Login Path (Actual v1 Flow)

```text
User → AccountCreateForm (OpenAI) → account_create_api_key
     → CredentialService::store_api_key (Keychain)
     → AccountRepository::create_api_key_account (SQLite metadata)
```

Validated by existing tests: `create_openai_anthropic_and_google_accounts`, `api_key_is_stored_in_keychain_not_sqlite`.

### Subagent A Verdict

**OpenAI OAuth: Not present (by design).** Generic OAuth lifecycle **validated** via Google implementation and test suite. No code changes required for this audit.

---

## Subagent B — Model Persistence Audit

**Requested scenario:**

| Pane | Model |
|------|-------|
| A | GPT-5.5 |
| B | GPT-5.4 mini |
| C | GPT-5.3 Codex Spark |

### Type Definitions (Present)

`src/types/paneSettings.ts` defines:

```typescript
export type OpenAiModelId = "gpt-5.5" | "gpt-5.4-mini" | "gpt-5.3-codex-spark";
```

### Persistence Layer (Partial)

| Layer | Model storage | Status |
|-------|---------------|--------|
| Schema | `panes.model_id TEXT` | Present |
| `PaneDto.model_id` | Serialized in `pane_list` | Present |
| `pane_create` | Does not set `model_id` | NULL on create |
| `pane_update_provider` / `pane_update_settings` | — | **Not implemented** |
| `stream_chat` | `UPDATE panes SET model_id = ?` at send time | Writes only during chat |
| Message snapshot | `messages.model_id` from pane at insert | Present |

### UI Layer (Gap)

| Component | Behavior | Gap |
|-----------|----------|-----|
| `ChatControls` | Single option: `OpenAIGpt` | Does not expose GPT-5.x models |
| `usePaneChat` | `selectedModelId` in React state | Initialized from `pane.modelId`; **not persisted on change** |
| `usePersistentPanes` | Loads panes via `paneList()` | No model-update command on selector change |

### Leakage Analysis

| Vector | Leakage? | Reason |
|--------|----------|--------|
| Pane A model visible in Pane B via `pane_list` | No | Each pane row has own `model_id` |
| Model change in UI affects other panes | No | State is per `usePaneChat(pane)` instance |
| Model survives restart without send | **No** | Selector changes never written to SQLite |
| `stream_chat` model overwrites pane binding | Per-pane only | `WHERE id = ?5` on single pane |

### Restart Scenario (Model)

| Step | Current behavior |
|------|------------------|
| User selects GPT-5.5 on Pane A | Lost on reload — UI resets to `pane.modelId ?? "OpenAIGpt"` |
| User sends message with GPT-5.5 | `stream_chat` persists `model_id` on pane row |
| App restart | `pane_list` returns persisted `model_id` **only if** a prior send occurred |

### Subagent B Verdict

**Fail for requested validation scenario.** Schema supports independent per-pane `model_id`, but no dedicated persistence path for UI model selection. Types in `paneSettings.ts` are unused.

**Recommended architecture (not implemented):**

- Store model in `panes.model_id` (canonical)
- Add `pane_update_settings(pane_id, model_id?, metadata_json?)` command
- Wire `usePaneChat.setSelectedModelId` to persist on change

---

## Subagent C — Reasoning Persistence Audit

**Requested levels:** Low, Medium, High, XHigh — persist independently per pane.

### Current State

| Layer | Reasoning support | Status |
|-------|-------------------|--------|
| `paneSettings.ts` | `ReasoningLevel = "low" \| "medium" \| "high" \| "xhigh"` | Types only |
| `panes.metadata_json` | Extensible TEXT column | No `reasoning_level` convention enforced |
| `panes` repository | No reasoning read/write | Not implemented |
| UI | No reasoning selector | Not implemented |
| `stream_chat` / OpenAI adapter | No reasoning parameter | Not implemented |

### Independence Analysis

Because reasoning is not persisted anywhere, there is no cross-pane leakage — but also **no persistence to validate**.

### Restart Scenario (Reasoning)

All panes would lose reasoning level on restart (or never have it set).

### Subagent C Verdict

**Fail.** Reasoning is a UI-type stub only. Recommended storage: `panes.metadata_json` with key `"reasoning_level"` (validated JSON, max 64 KB per SECURITY_MODEL).

---

## Subagent D — Workspace Interaction Audit

**Requested scenario:**

| Workspace | Pane | Model | Reasoning |
|-----------|------|-------|-----------|
| A | 1 | GPT-5.5 | High |
| A | 2 | GPT-5.4 mini | Medium |
| B | 1 | GPT-5.3 Codex Spark | Low |

### Architecture (Pass)

Pane settings are properties of the `panes` row, which carries `workspace_id`:

```text
workspaces (A) ──< panes (model_id, metadata_json) ──< messages
workspaces (B) ──< panes (model_id, metadata_json) ──< messages
```

| Check | Status | Evidence |
|-------|--------|----------|
| Workspace A pane list excludes B panes | Pass | Phase 5A `pane_list_is_workspace_scoped` |
| Model binding is per-pane, not global | Pass | `panes.model_id` column |
| Reasoning would be per-pane via `metadata_json` | Pass (design) | No shared settings table |
| Workspace switch alters accounts | No | Phase 5A `accounts_and_credentials_unaffected` |
| Same account bound in A and B panes | Allowed | By design — accounts are global |

### UI Gap

`usePersistentPanes` calls `paneList()` without `workspace_id` — always loads default workspace. Multi-workspace pane settings UI is not wired (Phase 5A implementation remaining).

### Cross-Workspace Contamination (Hypothetical)

If `pane_update_settings` existed:

- Updates scoped by `pane_id` (UUID) — no workspace parameter needed if pane IDs are globally unique
- `pane_list(workspace_id)` ensures UI only surfaces panes for active workspace
- No architectural path for Workspace A to read Workspace B `model_id` without knowing foreign pane UUID

### Subagent D Verdict

**Pass (architecture).** Workspace isolation for pane rows is sound. End-to-end validation of the requested scenario is blocked by missing pane-settings persistence (Subagents B & C).

---

## Account Isolation Audit

| Check | Status | Notes |
|-------|--------|-------|
| Accounts table has no `workspace_id` | Pass | Shared pool by design |
| API keys in Keychain, not SQLite | Pass | Phase 3A tests |
| OAuth tokens in Keychain, not SQLite | Pass | Phase 3B tests |
| Pane `account_id` binding is per-pane | Pass | Independent across panes/workspaces |
| `account_list` unaffected by workspace pane ops | Pass | Phase 5A test |
| OpenAI accounts are `auth_type = api_key` only | Pass | No OpenAI OAuth accounts possible |
| Google OAuth accounts isolated by `credential_ref` | Pass | Unique index on `accounts.credential_ref` |

**Expected answer:** Switching workspaces does not alter accounts or credentials. **Confirmed.**

---

## Validation Scenarios — Executability Matrix

| Scenario | Executable today? | Result |
|----------|-------------------|--------|
| Login via OpenAI OAuth | No | Use API-key path instead |
| Workspace A: 3 panes with distinct models | Partial | Only via direct SQL or post-send `stream_chat` |
| Workspace B: 1 pane with distinct model | Partial | Same |
| Reasoning levels per pane | No | Types only |
| Restart restores all settings | No | Model/reasoning not persisted on UI change |
| No cross-workspace settings leakage | Yes (architecture) | Phase 5A + per-pane row design |

---

## Test Coverage Referenced (No New Tests — Audit Only)

| Area | Tests |
|------|-------|
| OAuth PKCE | `pkce_challenge_uses_s256` |
| OAuth callback + exchange | `google_oauth_flow_completes_with_callback` |
| OAuth state | `oauth_rejects_state_mismatch` |
| OAuth refresh | `oauth_refresh_updates_keychain_and_account` |
| OAuth Keychain | `oauth_disconnect_removes_keychain_entry` |
| API-key isolation | `api_key_is_stored_in_keychain_not_sqlite` |
| Workspace pane isolation | `workspace_isolation.rs` (6 tests) |
| Pane restart (single workspace) | `pane_and_message_persistence_survives_reopen` |

---

## Remaining Risks

| Risk | Severity | Mitigation path |
|------|----------|-----------------|
| OpenAI OAuth not available | Medium | Future phase: add `oauth_config_json` + provider registration |
| Model selector not persisted | High | `pane_update_settings` + UI wiring |
| Reasoning not persisted | High | `metadata_json.reasoning_level` + UI + adapter passthrough |
| `paneSettings.ts` types unused | Low | Wire to persistence layer or remove until implemented |
| Multi-workspace UI not active | Medium | Phase 5A `workspace_switch` + `paneList(workspaceId)` |
| Model IDs in UI (`OpenAIGpt`) differ from pane types (`gpt-5.5`) | Medium | Unify identifier scheme before persistence |

---

## Architecture Recommendations (Documentation Only)

### Pane Settings Canonical Storage

| Setting | Storage | Example |
|---------|---------|---------|
| Model | `panes.model_id` | `gpt-5.5` |
| Reasoning | `panes.metadata_json` | `{"reasoning_level":"high"}` |
| Provider/account | `panes.provider_id`, `panes.account_id` | Existing |

### Commands (Future)

| Command | Purpose |
|---------|---------|
| `pane_update_settings` | Persist model + reasoning without sending a message |
| `oauth_start` (OpenAI) | Requires provider OAuth config + desktop client registration |

### Restart Contract

On `Database::initialize_at` reopen:

1. `pane_list(workspace_id)` returns each pane's `model_id` and `metadata_json`
2. UI hydrates selectors from pane row — no cross-pane defaults
3. Accounts reload globally via `account_list` — unchanged by workspace

---

## Verdict

**Phase 5B architecture validation: COMPLETE with documented gaps.**

- **OAuth:** Google lifecycle validated; OpenAI OAuth absent by design.
- **Pane settings:** Schema-ready; persistence and UI not implemented for GPT-5.x models or reasoning levels.
- **Workspace:** Per-pane settings architecture is isolation-safe; Phase 5A tests confirm no cross-workspace pane leakage.
- **Accounts:** Global and isolation-safe relative to workspace operations.

Implementation of pane-settings persistence and OpenAI OAuth are out of scope for this audit pass.