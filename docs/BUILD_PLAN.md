# BuilderBoard Build Plan

## Purpose

BuilderBoard is a macOS-native Tauri desktop application for running multiple AI chat panes in a single workspace. Each pane can target a different provider and account, with conversation history persisted locally.

This document defines the overall build plan, ownership boundaries, and architectural principles. It is design-only; no implementation is specified here beyond what future phases require.

## Project Goals

1. **Multi-pane workspace** — Users open a grid of chat panes, add or close panes independently, and restore layout on restart.
2. **Provider diversity** — Each pane resolves through a provider registry and can switch providers without losing pane identity.
3. **Account flexibility** — Users authenticate via OAuth or API key, per provider, with multiple accounts per provider when supported.
4. **Local-first persistence** — Pane layout, messages, provider configuration, and account metadata persist in SQLite; secrets stay in the OS keychain.
5. **Clean builder boundaries** — UI, provider abstractions, and infrastructure evolve in parallel without cross-cutting edits.

## Ownership Boundaries

| Builder | Owns | Must Not Modify |
|---------|------|-----------------|
| **Builder A** | `src/components/**`, `src/styles/**`, UI-facing `src/types/**` | Provider abstractions, database layer, OAuth flows |
| **Builder B** | `LLMProvider` trait, provider stubs, `models`/`providers` modules (see [PROVIDER_MODEL.md](./PROVIDER_MODEL.md), [ARCHITECTURE.md](./ARCHITECTURE.md)) | UI components, database schema, OAuth token storage |
| **Builder C (Infrastructure)** | `docs/**`, schema proposals, security/OAuth design | UI files, provider abstractions, executable code |

### Integration Contract

```
┌─────────────────────────────────────────────────────────────┐
│  Builder A — React UI                                       │
│  PaneGrid · Pane · Sidebar                                  │
│  Consumes: pane DTOs, message DTOs, provider/account labels │
└──────────────────────────┬──────────────────────────────────┘
                           │ Tauri commands + events
┌──────────────────────────▼──────────────────────────────────┐
│  Builder B — Provider Layer (`src-tauri/src/providers`)     │
│  LLMProvider trait · send/stream/list_models · StreamChunk  │
│  Consumes: CredentialHandle (Phase 3), provider registry rows │
└──────────────────────────┬──────────────────────────────────┘
                           │ repository + credential service
┌──────────────────────────▼──────────────────────────────────┐
│  Future Persistence Layer (Phase 2+)                        │
│  SQLite: workspaces, panes, messages, providers, accounts   │
│  Keychain: OAuth tokens, API keys                           │
└─────────────────────────────────────────────────────────────┘
```

Builder A renders state; Builder B executes provider calls; the persistence layer (future) owns durable records. None of these layers embed provider-specific logic in the others.

The `chat` boundary (see [ARCHITECTURE.md](./ARCHITECTURE.md)) orchestrates calls:

1. Load `Conversation` from `storage`.
2. Resolve `CredentialHandle` from `accounts` (Phase 3+).
3. Select provider by `provider_type` and construct adapter with credentials bound at instantiation (trait signature unchanged).
4. Invoke `adapter.send(ProviderRequest)` or `adapter.stream(ProviderRequest)`.

`ProviderRequest` stays credential-free per [PROVIDER_MODEL.md](./PROVIDER_MODEL.md). Builder B may add `with_credentials()` constructors in Phase 3 without changing the `LLMProvider` trait.

## Canonical Data Flow

```text
Workspace
  -> Pane (layout + provider/account binding)
    -> Message thread (per pane)
      -> Provider resolution (Builder B)
        -> Account credential (keychain-backed)
          -> Provider adapter
            -> Model response (streamed)
```

### Provider Switching

Provider switching is a **pane-level** operation:

1. User selects a different provider or account in a pane.
2. UI sends `pane_update_provider` with `pane_id`, `provider_id`, `account_id`, optional `model_id`.
3. Persistence updates the pane row; existing messages remain attached to the pane (not the provider).
4. Next send uses Builder B resolution against the new provider/account pair.
5. Message metadata records `provider_id` and `model_id` at send time for auditability.

Historical messages are never rewritten when the provider changes. Each message stores the provider context active at creation time in `metadata_json`.

## Technology Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Shell | Tauri 2.x | Native macOS desktop, Rust backend, React frontend |
| Frontend | React + TypeScript | Aligns with existing `src/` scaffold |
| Persistence | SQLite (via `rusqlite` or `sqlx`) | Local-first, single-file DB, proven in sibling projects |
| Secrets | macOS Keychain (`security-framework` / `keyring` crate) | OAuth tokens and API keys never stored in SQLite |
| Migrations | Versioned SQL + `schema_migrations` table | Idempotent, additive, startup-applied |

## Database Overview

Four core tables (see [DATABASE_DESIGN.md](./DATABASE_DESIGN.md)):

| Table | Role |
|-------|------|
| `providers` | Canonical provider registry (OpenAI, Anthropic, etc.) |
| `accounts` | User-linked auth records (OAuth or API key), keychain references only |
| `panes` | Pane layout, workspace binding, active provider/account |
| `messages` | Conversation history with `metadata_json` |

Supporting tables: `workspaces`, `schema_migrations`.

## Security Overview

See [SECURITY_MODEL.md](./SECURITY_MODEL.md) and [OAUTH_DESIGN.md](./OAUTH_DESIGN.md).

Principles:

- **No secrets in SQLite** — `accounts` stores `credential_ref` (keychain key), never raw tokens or API keys.
- **Least privilege** — Tauri capabilities restrict filesystem, network, and shell access.
- **OAuth via PKCE** — Desktop-safe authorization with loopback or custom URL scheme callback.
- **Provider isolation** — Adapters receive credentials through a credential service, not direct DB reads from UI.

## Phase Summary

| Phase | Focus | Primary Builder |
|-------|-------|-----------------|
| 1 | Pane grid UI shell | A |
| 1 | Provider trait + stub adapters | B |
| 1 | Architecture docs (this pass) | C |
| 2 | SQLite schema + migrations | C / shared |
| 2 | Pane/message persistence commands | Backend |
| 3 | OAuth flows + account linking | Backend + B |
| 3 | API key account management | Backend |
| 4 | Live provider streaming | B |
| 4 | Provider switching UX | A |
| 5 | Multi-workspace support | A + Backend |
| 5 | Message metadata enrichment | B + Backend |

Detailed breakdown: [PHASE_BREAKDOWN.md](./PHASE_BREAKDOWN.md).

## Non-Goals (Current Scope)

- Cloud sync or multi-device account sharing
- Windows/Linux support (macOS only for v1)
- Provider billing or usage metering
- Plugin/extension marketplace
- Real-time collaboration between users

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Builder A UI types drift from DB DTOs | Shared `src/types/api.ts` contract owned by integration phase; DB layer maps to DTOs |
| Provider trait changes break adapters | Builder B owns semver on trait; infrastructure docs define stable `providers` registry columns |
| OAuth redirect handling on macOS | Documented PKCE + loopback pattern in OAUTH_DESIGN.md |
| Migration failures on upgrade | Additive-only migrations, `schema_migrations` ledger, pre-migration backup hook |
| Secret leakage via logs | Security model prohibits logging credential_ref resolution payloads |

## Document Index

| Document | Contents | Owner |
|----------|----------|-------|
| [BUILD_PLAN.md](./BUILD_PLAN.md) | Goals, boundaries, data flow | C |
| [DATABASE_DESIGN.md](./DATABASE_DESIGN.md) | SQLite schema, indexes, migration strategy | C |
| [OAUTH_DESIGN.md](./OAUTH_DESIGN.md) | OAuth flows, token lifecycle, provider-specific notes | C |
| [SECURITY_MODEL.md](./SECURITY_MODEL.md) | Threat model, credential storage, capability boundaries | C |
| [PHASE_BREAKDOWN.md](./PHASE_BREAKDOWN.md) | Per-phase deliverables, dependencies, acceptance criteria | C |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Backend module boundaries (`auth`, `providers`, `storage`, etc.) | B |
| [PROVIDER_MODEL.md](./PROVIDER_MODEL.md) | `LLMProvider` trait contract and stub providers | B |

## Acceptance Criteria (Documentation Pass)

- [x] All infrastructure documents exist under `docs/` (five C-owned + two B-owned companion docs)
- [x] Schema proposal covers `panes`, `messages`, `providers`, `accounts`
- [x] `workspace_id`, `metadata_json`, provider switching, OAuth, and API-key accounts are addressed
- [x] Builder A and Builder B ownership boundaries are explicit
- [x] No UI, provider, database, or OAuth implementation code is introduced