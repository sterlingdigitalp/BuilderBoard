# BuilderBoard Phase Breakdown

## Overview

BuilderBoard is built in parallel tracks by three builders. This document defines phases, dependencies, deliverables, and acceptance criteria for each.

**Current state:** Project scaffold exists (`src/components/{Pane,PaneGrid,Sidebar}`, `src-tauri/`). No implementation code yet. Infrastructure documentation (this pass) is complete.

## Builder Ownership (Standing)

| Builder | Responsibility | Paths |
|---------|----------------|-------|
| **A** | UI components, styles, UI types | `src/components/**`, `src/styles/**`, UI `src/types/**` |
| **B** | Provider trait, adapters, streaming | `src-tauri/src/providers/**` (proposed) |
| **C** | Architecture docs, schema design, security/OAuth design | `docs/**` |

Cross-builder integration occurs at Tauri command boundaries and shared DTO types agreed in Phase 2.

---

## Phase 1 — Foundation (Current)

### 1A: UI Shell (Builder A)

**Goal:** Render an empty multi-pane grid with sidebar navigation.

| Deliverable | Description |
|-------------|-------------|
| `PaneGrid` | CSS grid layout, add/close pane controls |
| `Pane` | Pane chrome (title, role label, close button) |
| `Sidebar` | Workspace nav placeholder, settings link |
| `src/types/pane.ts` | UI pane state types |
| In-memory state | No persistence; panes exist in React state only |

**Acceptance criteria:**

- App opens to a grid with one default pane
- User can add and close panes
- Closing a pane does not affect others
- No Tauri persistence commands required

### 1B: Provider Abstraction (Builder B) ✅

**Goal:** Define the provider contract without live API calls.

| Deliverable | Status | Description |
|-------------|--------|-------------|
| `LLMProvider` trait | ✅ | `send`, `stream`, `list_models` per [PROVIDER_MODEL.md](./PROVIDER_MODEL.md) |
| Adapter stubs | ✅ | `AnthropicProvider`, `OpenAIProvider`, `GoogleProvider` |
| `models` module | ✅ | `Message`, `Conversation`, `Model`, `MessageRole` |
| `storage` trait | ✅ | `ConversationStore` placeholder |
| `auth` trait | ✅ | `AuthSessionStore` placeholder (app subject only) |

**Acceptance criteria:**

- Trait compiles and stub adapters return `NotImplemented` for send/stream
- `list_models` returns static placeholders, no network calls
- No SQLite, no OAuth, no keychain
- Provider enum variants map to `providers.id` seeds: `anthropic`, `openai`, `google`

### 1C: Infrastructure Documentation (Builder C) ✅

**Goal:** Architecture docs for future phases.

| Deliverable | Status |
|-------------|--------|
| `docs/BUILD_PLAN.md` | ✅ Complete |
| `docs/DATABASE_DESIGN.md` | ✅ Complete |
| `docs/OAUTH_DESIGN.md` | ✅ Complete |
| `docs/SECURITY_MODEL.md` | ✅ Complete |
| `docs/PHASE_BREAKDOWN.md` | ✅ Complete |

**Acceptance criteria:**

- All infrastructure documents exist (C-owned set complete; B-owned `ARCHITECTURE.md` and `PROVIDER_MODEL.md` companion docs present)
- Schema covers `panes`, `messages`, `providers`, `accounts`
- `workspace_id`, `metadata_json`, provider switching, OAuth, API-key documented
- No implementation code introduced

---

## Phase 2 — Persistence

**Depends on:** Phase 1A (pane types), Phase 1C (schema design)

### 2A: SQLite Layer (Shared / Backend) ✅

| Deliverable | Status | Description |
|-------------|--------|-------------|
| `migrations/0001_initial_schema.sql` | ✅ | Tables per DATABASE_DESIGN |
| Migration runner | ✅ | Startup-applied, `schema_migrations` ledger |
| Repository modules | ✅ | `workspaces`, `panes`, `messages`, `providers` (`accounts` schema only) |
| Tauri commands | ✅ | `pane_list`, `pane_create`, `pane_close`, `message_list`, `message_append` |

See [PHASE2A_IMPLEMENTATION.md](./PHASE2A_IMPLEMENTATION.md).

**Acceptance criteria:**

- Fresh install creates DB with seeded providers and default workspace
- Pane create/close/list persists across app restart
- Messages append and reload per pane
- Migrations are idempotent (safe to re-run)
- Pre-migration backup created when upgrading an existing database file
- No secret columns in schema

### 2B: UI Persistence Integration (Builder A)

| Deliverable | Description |
|-------------|-------------|
| Replace in-memory state | Load panes from `pane_list` on mount |
| Pane lifecycle | `pane_create` / `pane_close` on user action |
| Message display | Render `message_list` per active pane |
| Loading states | Skeleton UI during DB load |

**Acceptance criteria:**

- Pane grid restores on restart
- Message history visible per pane
- Builder A does not import SQLite directly

### 2C: Provider Registry Persistence (Builder B)

| Deliverable | Description |
|-------------|-------------|
| Registry loads from `providers` table | ✅ Map `provider_type` → `LLMProvider` implementation for `anthropic`, `openai`, and `google` |
| `provider_list` command | ✅ Returns enabled providers for UI picker |
| `chat` boundary wiring | ✅ Selects provider by `panes.provider_id` without model execution |

**Acceptance criteria:**

- Provider list matches seeded rows
- `chat` selects correct `LLMProvider` stub for pane's `provider_id`
- Builder B does not modify `providers` schema or `LLMProvider` trait signature
- Unsupported provider types return structured errors
- No networking, OAuth, API keys, account handling, or streaming is introduced

---

## Phase 3 — Accounts & Authentication

**Depends on:** Phase 2 (persistence layer), Phase 1B (provider trait)

### 3A: Credential Service (Backend) ✅

| Deliverable | Status | Description |
|-------------|--------|-------------|
| Keychain integration | ✅ | `auth/credential_service.rs` |
| `account_create_api_key` | ✅ | API key path with rollback on DB failure |
| `account_list` / `account_disconnect` / `account_get_status` | ✅ | Account management commands |
| `accounts` repository | ✅ | SQLite CRUD + `is_default` behavior |
| Migration `0002_accounts_is_default` | ✅ | Default account column |
| Account-aware provider resolution | ✅ | Explicit pane account or provider default account resolves to `CredentialHandle` + `LLMProvider` |

See [PHASE3A_IMPLEMENTATION.md](./PHASE3A_IMPLEMENTATION.md).

**Acceptance criteria:**

- API key stored in Keychain, `credential_ref` in SQLite
- API key never returned to frontend after creation
- Disconnect removes keychain entry and sets `status = revoked`
- Explicit pane account selection is honored before provider defaults
- Missing, inactive, and unsupported-provider resolution failures return structured errors
- No OAuth, networking, streaming, or model execution is introduced

### 3B: OAuth Flow (Backend)

| Deliverable | Description |
|-------------|-------------|
| PKCE flow | Per OAUTH_DESIGN |
| Loopback callback server | Ephemeral port, 5-min timeout |
| Token refresh | Proactive refresh before expiry |
| `oauth_start` command | Opens system browser |

**Acceptance criteria:**

- Google OAuth completes end-to-end in dev environment
- Tokens in Keychain only
- Expired account marked `status = expired`
- Builder B adapters receive credentials via Credential Service

### 3C: Account UI (Builder A)

| Deliverable | Description |
|-------------|-------------|
| Settings → Accounts view | List accounts by provider |
| API key input form | For `auth_mode = api_key` providers |
| OAuth connect button | For `auth_mode = oauth` providers |
| Account status badges | active, expired, error |

**Acceptance criteria:**

- User can add API key account for OpenAI
- User can OAuth connect Google account
- User can disconnect account
- No secrets displayed in UI

---

## Phase 4 — Live Provider Streaming

**Depends on:** Phase 3 (accounts), Phase 2B (message UI), Phase 1B (provider trait)

### 4A: Provider Execution (Builder B)

| Deliverable | Description |
|-------------|-------------|
| Live adapters | OpenAI, Anthropic HTTPS streaming |
| `stream_chat` implementation | SSE/chunked response parsing |
| Tauri events | `message_stream_chunk`, `message_stream_complete`, `message_stream_error` |
| Token counting | Populate `token_count_input/output` |

**Acceptance criteria:**

- Real provider call with valid API key succeeds
- Streaming chunks arrive in UI in order
- Error surfaces as `messages.status = error`
- No credentials in logs or events

### 4B: Chat UI (Builder A)

| Deliverable | Description |
|-------------|-------------|
| Message input | Send on Enter |
| Streaming display | Append chunks to active assistant message |
| Error display | Show provider errors inline |
| Model selector | Per-pane model dropdown |

**Acceptance criteria:**

- User sends message, receives streamed response
- Pane shows streaming indicator during response
- Failed sends show actionable error

### 4C: Provider Switching (Builder A + Backend)

| Deliverable | Description |
|-------------|-------------|
| `pane_update_provider` command | Updates pane binding |
| Provider picker in pane chrome | Provider + account + model selection |
| Historical context preserved | Old messages retain original `metadata_json` |

**Acceptance criteria:**

- Switch provider on pane; next message uses new provider
- Previous messages unchanged
- Pane without account shows connect prompt

---

## Phase 5 — Multi-Workspace & Polish

**Depends on:** Phase 4 (working chat)

### 5A: Workspace Management (Builder A + Backend)

| Deliverable | Description |
|-------------|-------------|
| `workspace_create` / `workspace_list` / `workspace_switch` | Multi-workspace commands |
| Sidebar workspace list | Switch between workspaces |
| Per-workspace pane sets | `workspace_id` scoping |

**Acceptance criteria:**

- User creates second workspace with independent pane set
- Switching workspace loads correct panes and messages
- Default workspace exists on fresh install

### 5B: Message Metadata Enrichment (Builder B + Backend)

| Deliverable | Description |
|-------------|-------------|
| Full `metadata_json` population | Provider request IDs, latency, finish_reason |
| Message regeneration | `regenerated_from` in metadata |
| Export | Workspace export to JSON (no secrets) |

**Acceptance criteria:**

- Metadata visible in message inspector (dev mode or settings)
- Export file contains messages, not credentials

### 5C: Migration Hardening (Backend)

| Deliverable | Description |
|-------------|-------------|
| Pre-migration backup | Automatic `backups/` copy |
| Convergence checks | Guarded ALTER for column drift |
| Migration integration test | Upgrade from 0001 → latest |

**Acceptance criteria:**

- Upgrade from Phase 2 schema to Phase 5 schema without data loss
- Failed migration rolls back and preserves backup

---

## Dependency Graph

```text
Phase 1A (UI Shell)  ──────────────────────────┐
Phase 1B (Provider)  ─────────────────────┐    │
Phase 1C (Docs)      ────────────────┐    │    │
                                     v    v    v
                              Phase 2 (Persistence)
                                     │
                        ┌────────────┼────────────┐
                        v            v            v
                   Phase 3A     Phase 3B     Phase 3C
                  (API Key)     (OAuth)      (Account UI)
                        └────────────┼────────────┘
                                     v
                              Phase 4 (Streaming)
                                     │
                        ┌────────────┼────────────┐
                        v            v            v
                   Phase 5A     Phase 5B     Phase 5C
                (Workspaces)  (Metadata)   (Migrations)
```

## Recommended Phase 2 Sequence

When Phase 1 completes, implement Phase 2 in this order:

1. **Migration runner + `0001_initial_schema`** — Foundation for all persistence
2. **Provider seed + `provider_list`** — Unblocks provider picker UI
3. **Pane repository + commands** — Unblocks Builder A persistence integration
4. **Message repository + commands** — Enables history display
5. **Default workspace seed + verification** — End-to-end restart test
6. **Builder A integration** — Wire UI to commands
7. **Builder B registry DB load** — Replace in-memory provider seed

## Cross-Phase Validation Scenarios

| Scenario | Phases | Validates |
|----------|--------|-----------|
| Pane survives restart | 2 | Persistence, migration safety |
| Provider list matches docs | 2 | Schema seed, Builder B compatibility |
| API key account CRUD | 3 | Security model, no secrets in SQLite |
| OAuth connect Google | 3 | OAuth design, keychain storage |
| Send message to OpenAI | 4 | Provider abstraction, streaming |
| Switch provider mid-pane | 4 | Provider switching, metadata_json |
| Two workspaces isolated | 5 | workspace_id scoping |
| Schema upgrade 0001→latest | 5 | Migration safety |

## Architectural Concerns

| Concern | Impact | Owner | Resolution Phase |
|---------|--------|-------|------------------|
| Shared DTO types between builders | Integration friction | Agreement in Phase 2 kickoff | 2 |
| Provider trait stability | Adapter rework | Builder B | Ongoing |
| Keychain access in dev | OAuth blocked in CI | Backend | 3 |
| Large message history performance | Slow pane load | Backend | 5 (pagination) |
| SQLite file corruption | Data loss | Backend | 5 (backups) |
| Builder A/B parallel edits | Merge conflicts | Process | Per-phase |

## Document Cross-References

| Phase | Primary Docs |
|-------|-------------|
| 1C | All docs in `docs/` |
| 2 | DATABASE_DESIGN.md |
| 3 | OAUTH_DESIGN.md, SECURITY_MODEL.md |
| 4 | BUILD_PLAN.md (data flow), DATABASE_DESIGN.md (metadata_json) |
| 5 | DATABASE_DESIGN.md (workspaces), PHASE_BREAKDOWN.md |
