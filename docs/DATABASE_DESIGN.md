# BuilderBoard Database Design

## Scope

This document proposes the SQLite schema for BuilderBoard future phases. It is a design artifact only — no migration files, Rust code, or SQL scripts are created in this pass.

### In Scope

- Table definitions: `workspaces`, `providers`, `accounts`, `panes`, `messages`
- Supporting table: `schema_migrations`
- Indexes, constraints, and foreign keys
- Migration strategy (additive, idempotent, versioned)
- Future support for `workspace_id`, `metadata_json`, provider switching, OAuth accounts, API-key accounts

### Out of Scope

- Implementation code
- Provider adapter logic (Builder B)
- UI state management (Builder A)

## Design Principles

1. **Additive migrations only** — No `DROP TABLE`, no column rewrites. New columns use `ALTER TABLE` with defaults.
2. **Idempotent startup** — `CREATE TABLE IF NOT EXISTS`, guarded `ALTER TABLE`, seed inserts with `INSERT OR IGNORE`.
3. **Secrets stay out of SQLite** — `accounts.credential_ref` points to a keychain entry; no token or API key columns.
4. **Pane identity is stable** — Provider switching updates `panes.provider_id` / `panes.account_id`; messages retain historical provider context in `metadata_json`.
5. **Builder B compatibility** — `providers` registry columns align with the provider resolution contract Builder B will expose (provider id, type, capabilities, configuration JSON).

## Entity Relationship

```text
workspaces (1) ──< panes (N)
workspaces (1) ──< messages (N)   [denormalized for query efficiency]

providers (1) ──< accounts (N)
providers (1) ──< panes (N)       [active binding]

accounts (1) ──< panes (N)        [active binding, nullable]

panes (1) ──< messages (N)
```

## Database Location

```
~/Library/Application Support/com.builderboard.app/builderboard.db
```

Single SQLite file. WAL mode enabled at connection open. `PRAGMA foreign_keys = ON`.

---

## Table: `schema_migrations`

Tracks applied schema versions for safe upgrade paths.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `version` | TEXT | PRIMARY KEY | Migration identifier, e.g. `0001_initial_schema` |
| `applied_at` | TEXT | NOT NULL | ISO 8601 UTC timestamp |

---

## Table: `workspaces`

Supports multi-workspace future phase. Every pane and message carries `workspace_id`.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | UUID v4 |
| `name` | TEXT | NOT NULL | Display name |
| `slug` | TEXT | UNIQUE | URL-safe identifier for deep links |
| `is_default` | INTEGER | NOT NULL DEFAULT 0 | 1 if this is the default workspace |
| `layout_json` | TEXT | | Serialized grid layout (Builder A contract) |
| `metadata_json` | TEXT | | Extensible workspace settings |
| `created_at` | TEXT | NOT NULL | ISO 8601 UTC |
| `updated_at` | TEXT | NOT NULL | ISO 8601 UTC |
| `archived_at` | TEXT | | Soft-delete timestamp; NULL if active |

**Indexes:**

- `idx_workspaces_slug` ON `slug`
- `idx_workspaces_active` ON `archived_at` WHERE `archived_at IS NULL`

**Seed:** One default workspace inserted at `0001_initial_schema` with `is_default = 1`.

---

## Table: `providers`

Canonical provider registry. Builder B adapters resolve against `providers.id`.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | Stable slug, e.g. `openai`, `anthropic` |
| `provider_type` | TEXT | NOT NULL | Adapter discriminator matching Builder B enum |
| `display_name` | TEXT | NOT NULL | Human-readable label |
| `enabled` | INTEGER | NOT NULL DEFAULT 1 | 0 = hidden from picker |
| `auth_mode` | TEXT | NOT NULL | `oauth`, `api_key`, `none`, `local` |
| `supports_chat` | INTEGER | NOT NULL DEFAULT 1 | |
| `supports_streaming` | INTEGER | NOT NULL DEFAULT 1 | |
| `supports_tool_use` | INTEGER | NOT NULL DEFAULT 0 | |
| `supports_vision` | INTEGER | NOT NULL DEFAULT 0 | |
| `context_window` | INTEGER | | Default context window tokens |
| `locality` | TEXT | NOT NULL DEFAULT `remote` | `local` or `remote` |
| `oauth_config_json` | TEXT | | OAuth endpoints, scopes (no secrets) |
| `configuration_json` | TEXT | | Base URL, default model, adapter hints |
| `created_at` | TEXT | NOT NULL | |
| `updated_at` | TEXT | NOT NULL | |

**Check constraints:**

- `auth_mode IN ('oauth', 'api_key', 'none', 'local')`
- `locality IN ('local', 'remote')`

**Indexes:**

- `idx_providers_type_enabled` ON (`provider_type`, `enabled`)

**Seed providers (idempotent):**

| id | provider_type | auth_mode |
|----|---------------|-----------|
| `openai` | `openai` | `api_key` |
| `anthropic` | `anthropic` | `api_key` |
| `google` | `google` | `oauth` |
| `openrouter` | `openrouter` | `api_key` |
| `ollama` | `ollama` | `none` |
| `lmstudio` | `lmstudio` | `none` |

OAuth-enabled providers store public OAuth metadata in `oauth_config_json` (authorization URL, token URL, scopes). Client secrets are injected at runtime from environment or build config, never from SQLite.

---

## Table: `accounts`

User-linked authentication records. Supports both OAuth and API-key accounts.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | UUID v4 |
| `provider_id` | TEXT | NOT NULL, FK → `providers.id` | |
| `label` | TEXT | NOT NULL | User-facing name, e.g. "Work OpenAI" |
| `auth_type` | TEXT | NOT NULL | `oauth` or `api_key` |
| `credential_ref` | TEXT | NOT NULL | Keychain service key; opaque to UI |
| `external_account_id` | TEXT | | Provider-side account/subject id (OAuth) |
| `external_email` | TEXT | | Display only; nullable |
| `token_expires_at` | TEXT | | OAuth access token expiry; NULL for API keys |
| `scopes_json` | TEXT | | Granted OAuth scopes |
| `status` | TEXT | NOT NULL DEFAULT `active` | `active`, `expired`, `revoked`, `error` |
| `last_used_at` | TEXT | | |
| `metadata_json` | TEXT | | Provider-specific non-secret metadata |
| `created_at` | TEXT | NOT NULL | |
| `updated_at` | TEXT | NOT NULL | |

**Check constraints:**

- `auth_type IN ('oauth', 'api_key')`
- `status IN ('active', 'expired', 'revoked', 'error')`

**Foreign keys:**

- `provider_id` → `providers(id)` ON DELETE RESTRICT

**Indexes:**

- `idx_accounts_provider` ON (`provider_id`, `status`)
- `idx_accounts_credential_ref` ON `credential_ref` UNIQUE

**Credential storage contract:**

| auth_type | Keychain payload | SQLite stores |
|-----------|------------------|---------------|
| `api_key` | `{ "api_key": "sk-..." }` | `credential_ref` only |
| `oauth` | `{ "access_token": "...", "refresh_token": "...", "token_type": "Bearer", "expires_at": "ISO8601" }` | `credential_ref`, `token_expires_at`, `scopes_json` |

---

## Table: `panes`

Pane layout and active provider/account binding. Aligns with Builder A `Pane` / `PaneGrid` components.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | UUID v4; stable across provider switches |
| `workspace_id` | TEXT | NOT NULL, FK → `workspaces.id` | |
| `title` | TEXT | | User-editable pane title |
| `role_label` | TEXT | | Optional role badge (Builder A) |
| `sort_order` | INTEGER | NOT NULL DEFAULT 0 | Grid position |
| `width_ratio` | REAL | | Grid column weight |
| `height_ratio` | REAL | | Grid row weight |
| `provider_id` | TEXT | FK → `providers.id` | Active provider; nullable until configured |
| `account_id` | TEXT | FK → `accounts.id` | Active account; nullable |
| `model_id` | TEXT | | Active model override |
| `system_prompt` | TEXT | | Per-pane system prompt |
| `status` | TEXT | NOT NULL DEFAULT `idle` | `idle`, `streaming`, `error` |
| `layout_json` | TEXT | | Pane-specific UI state (scroll position, etc.) |
| `metadata_json` | TEXT | | Extensible pane metadata |
| `created_at` | TEXT | NOT NULL | |
| `updated_at` | TEXT | NOT NULL | |
| `closed_at` | TEXT | | Soft-close; NULL if open |

**Check constraints:**

- `status IN ('idle', 'streaming', 'error')`

**Foreign keys:**

- `workspace_id` → `workspaces(id)` ON DELETE CASCADE
- `provider_id` → `providers(id)` ON DELETE SET NULL
- `account_id` → `accounts(id)` ON DELETE SET NULL

**Indexes:**

- `idx_panes_workspace_order` ON (`workspace_id`, `sort_order`) WHERE `closed_at IS NULL`
- `idx_panes_provider` ON (`provider_id`, `account_id`)

### Provider Switching Behavior

When a user switches provider on a pane:

```sql
UPDATE panes
SET provider_id = :new_provider_id,
    account_id = :new_account_id,
    model_id = :new_model_id,
    updated_at = :now
WHERE id = :pane_id;
```

Existing `messages` rows are untouched. New messages record the active provider in `metadata_json` at insert time.

---

## Table: `messages`

Conversation history per pane with extensible metadata.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | UUID v4 |
| `workspace_id` | TEXT | NOT NULL, FK → `workspaces.id` | Denormalized for workspace-scoped queries |
| `pane_id` | TEXT | NOT NULL, FK → `panes.id` | |
| `parent_id` | TEXT | FK → `messages.id` | Thread branching; NULL for root |
| `role` | TEXT | NOT NULL | `user`, `assistant`, `system`, `tool` |
| `content` | TEXT | NOT NULL | Message body (markdown) |
| `content_type` | TEXT | NOT NULL DEFAULT `text` | `text`, `markdown`, `json` |
| `status` | TEXT | NOT NULL DEFAULT `complete` | `pending`, `streaming`, `complete`, `error` |
| `provider_id` | TEXT | FK → `providers.id` | Provider at message creation |
| `account_id` | TEXT | FK → `accounts.id` | Account at message creation |
| `model_id` | TEXT | | Model at message creation |
| `token_count_input` | INTEGER | | |
| `token_count_output` | INTEGER | | |
| `error_code` | TEXT | | Provider error code if `status = error` |
| `error_message` | TEXT | | Human-readable error |
| `metadata_json` | TEXT | NOT NULL DEFAULT `'{}'` | Extensible metadata (see below) |
| `created_at` | TEXT | NOT NULL | |
| `updated_at` | TEXT | NOT NULL | |
| `completed_at` | TEXT | | When streaming finished |

**Check constraints:**

- `role IN ('user', 'assistant', 'system', 'tool')`
- `status IN ('pending', 'streaming', 'complete', 'error')`
- `content_type IN ('text', 'markdown', 'json')`

**Foreign keys:**

- `workspace_id` → `workspaces(id)` ON DELETE CASCADE
- `pane_id` → `panes(id)` ON DELETE CASCADE
- `parent_id` → `messages(id)` ON DELETE SET NULL
- `provider_id` → `providers(id)` ON DELETE SET NULL
- `account_id` → `accounts(id)` ON DELETE SET NULL

**Indexes:**

- `idx_messages_pane_created` ON (`pane_id`, `created_at`)
- `idx_messages_workspace_created` ON (`workspace_id`, `created_at`)
- `idx_messages_status` ON (`pane_id`, `status`) WHERE `status IN ('pending', 'streaming')`

### `metadata_json` Schema

Flexible JSON object. Documented keys (all optional):

```json
{
  "provider_id": "openai",
  "model_id": "gpt-4o",
  "account_id": "uuid",
  "finish_reason": "stop",
  "tool_calls": [],
  "attachments": [],
  "latency_ms": 1234,
  "request_id": "provider-side-id",
  "stream_chunks": 42,
  "edited": false,
  "regenerated_from": "message-uuid"
}
```

Provider adapters (Builder B) may write adapter-specific keys under a namespaced prefix, e.g. `"openai": { "response_id": "..." }`. The persistence layer does not validate JSON shape beyond valid JSON.

---

## Migration Strategy

### Versioning

Migrations are numbered SQL files applied in order at app startup:

```
migrations/
  0001_initial_schema.sql
  0002_add_message_parent_id.sql
  ...
```

Each file is recorded in `schema_migrations` after successful application. Already-applied versions are skipped.

### Migration Rules

| Rule | Rationale |
|------|-----------|
| `CREATE TABLE IF NOT EXISTS` | Safe re-run on partial failure |
| `INSERT OR IGNORE` for seeds | Idempotent provider/workspace seeds |
| Guarded `ALTER TABLE` via `PRAGMA table_info` check | Absorb partially-upgraded databases |
| No `DROP` or `DELETE` of user data | Migration safety |
| Foreign keys enabled before migration batch | Referential integrity |
| Backup before first migration on existing DB | User data protection |

### Migration Sequence (Proposed)

| Version | Description |
|---------|-------------|
| `0001_initial_schema` | Create all tables, indexes, seeds (workspaces, providers); copy backup if DB already exists |
| `0002_pane_layout_columns` | Add `width_ratio`, `height_ratio`, `layout_json` if missing (guarded) |
| `0003_message_metadata_defaults` | Ensure `metadata_json` defaults to `'{}'` on existing rows |
| `0004_account_status_index` | Add `idx_accounts_provider` if missing |

Future migrations follow the same additive pattern.

### Startup Migration Flow

```text
1. Open SQLite connection (WAL mode, foreign_keys ON)
2. Ensure schema_migrations table exists
3. For each migration file in order:
   a. Skip if version in schema_migrations
   b. BEGIN TRANSACTION
   c. Execute migration SQL
   d. INSERT INTO schema_migrations (version, applied_at)
   e. COMMIT
4. Run convergence checks (guarded ALTER for any drift)
5. Return connection to app state
```

### Rollback Policy

- **No automatic down-migrations.** If a migration fails, the transaction rolls back and the app reports the error.
- Users can restore from automatic pre-migration backup (`backups/builderboard.db.{timestamp}.bak`).
- Schema version mismatch does not block read-only access to messages (future graceful degradation).

---

## Builder B Compatibility

The `providers` table is the persistence backing for Builder B's provider registry. Builder B's current `Provider` enum (`Anthropic`, `OpenAI`, `Google`) maps to registry rows by slug:

| `providers.id` | `providers.provider_type` | Builder B `Provider` enum |
|----------------|---------------------------|---------------------------|
| `anthropic` | `anthropic` | `Provider::Anthropic` |
| `openai` | `openai` | `Provider::OpenAI` |
| `google` | `google` | `Provider::Google` |
| `openrouter` | `openrouter` | *(future adapter)* |
| `ollama` | `ollama` | *(future adapter)* |
| `lmstudio` | `lmstudio` | *(future adapter)* |

**Unknown `provider_type` handling:** If `provider_type` has no registered `LLMProvider` implementation, the `chat` boundary returns a structured error before invoking the adapter. The pane remains configured; the UI prompts the user to select a supported provider. Disabled providers (`enabled = 0`) are rejected at resolution time with the same pattern.

**`model_id` mapping:** Database stores `model_id` as TEXT (e.g. `gpt-4o`). Builder B maps to `models::Model::Custom(model_id)` when no enum variant matches. Provider stubs may continue returning enum variants until live adapters ship.

**`MessageRole` mapping:** Database `messages.role` includes `tool` for future tool-use support. The `storage` boundary maps DB roles to `models::MessageRole` as follows: `system` → `System`, `user` → `User`, `assistant` → `Assistant`, `tool` → stored in DB but excluded from `Conversation` until `MessageRole::Tool` is added to `models` (Phase 4). Tool messages remain queryable via `message_list` for UI display.

Resolution contract (implemented in `chat` + `storage` boundaries, not inside provider stubs):

```text
Input:  pane.provider_id, pane.account_id, pane.model_id
Lookup: providers row (enabled, provider_type, configuration_json)
        accounts row (auth_type, credential_ref, status)
Build:  models::Conversation from messages table (via storage boundary)
Credential (Phase 3+): chat boundary resolves CredentialHandle from accounts.credential_ref, then instantiates the adapter with credentials bound at construction time (e.g. OpenAIProvider::with_credentials(handle)). The LLMProvider trait signature is unchanged per PROVIDER_MODEL.md.
Invoke: adapter.stream(ProviderRequest) or adapter.send(...)
Output: ProviderResponse / StreamChunk → normalized models::Message
```

Builder B provider implementations must not read `messages` or `panes` directly. The `storage` boundary loads a `Conversation`; the `chat` boundary selects the `LLMProvider` stub/adapter matching `provider_type`.

Column mapping:

| Builder B concept | DB column |
|-------------------|-----------|
| `providers::Provider` variant | `providers.provider_type` |
| `models::Model` | `panes.model_id` / `messages.model_id` (TEXT, e.g. `gpt-4o`) |
| `models::Conversation.id` | `panes.id` (one conversation thread per pane) |
| `models::Message` | `messages` row (`role`, `content`) |
| Capabilities | `providers.supports_*` columns |
| Config | `providers.configuration_json` |
| Credential | `accounts.credential_ref` → keychain (via `auth` boundary, Phase 3) |

---

## Builder A Compatibility

Pane DTO returned to UI:

```typescript
interface PaneDto {
  id: string;
  workspaceId: string;
  title: string | null;
  roleLabel: string | null;
  sortOrder: number;
  widthRatio: number | null;
  heightRatio: number | null;
  providerId: string | null;
  accountId: string | null;
  modelId: string | null;
  status: 'idle' | 'streaming' | 'error';
  layoutJson: string | null;
  metadataJson: string | null;
}
```

Builder A components consume DTOs via Tauri commands. They do not query SQLite directly.

---

## Query Patterns

| Use Case | Query |
|----------|-------|
| Load workspace panes | `SELECT * FROM panes WHERE workspace_id = ? AND closed_at IS NULL ORDER BY sort_order` |
| Load pane messages | `SELECT * FROM messages WHERE pane_id = ? ORDER BY created_at` |
| List active accounts for provider | `SELECT * FROM accounts WHERE provider_id = ? AND status = 'active'` |
| List enabled providers | `SELECT * FROM providers WHERE enabled = 1 ORDER BY display_name` |
| Streaming in-flight | `SELECT * FROM messages WHERE pane_id = ? AND status = 'streaming'` |

---

## Risks

| Risk | Mitigation |
|------|------------|
| `metadata_json` unbounded growth | Consider pruning `stream_chunks` after completion; document size guidance |
| Denormalized `workspace_id` on messages drifts | Repository always sets both `pane_id` and `workspace_id` from pane row |
| Account deletion with active pane binding | `ON DELETE SET NULL` on `panes.account_id`; UI prompts re-selection |
| OAuth token expiry | `accounts.status = expired`; refresh flow in OAUTH_DESIGN.md |
| Builder B trait adds required provider fields | Add columns via guarded migration; never rewrite registry ids |