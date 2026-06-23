# Phase 3A Implementation Report

## Status

**Complete** — API-key credential service, account management, and account-aware provider resolution implemented. OAuth is out of scope.

## Deliverables

| Item | Location | Status |
|------|----------|--------|
| Credential service | `src-tauri/src/auth/credential_service.rs` | Done |
| Accounts repository | `src-tauri/src/storage/repositories/accounts.rs` | Done |
| `is_default` migration | `migrations/0002_accounts_is_default.sql` | Done |
| `account_create_api_key` | `src-tauri/src/storage/commands.rs` | Done |
| `account_list` | `src-tauri/src/storage/commands.rs` | Done |
| `account_disconnect` | `src-tauri/src/storage/commands.rs` | Done |
| `account_get_status` | `src-tauri/src/storage/commands.rs` | Done |
| Account-aware provider resolution | `src-tauri/src/chat/mod.rs` | Done |
| `CredentialHandle` | `src-tauri/src/auth/mod.rs` | Done |

## Keychain Design

| Property | Value |
|----------|-------|
| Service | `com.builderboard.app` |
| Account key | `credential_ref` (UUID) |
| Payload | `{"api_key":"..."}` JSON |
| Production store | macOS Keychain via `keyring` crate |
| Test store | In-memory `MemoryCredentialStore` |

Secrets never appear in SQLite or IPC responses. `AccountDto` excludes `credential_ref`, `api_key`, and tokens.

## Account Repository

| Method | Purpose |
|--------|---------|
| `create_api_key_account` | Insert active account; auto-default if first for provider |
| `list_active` | List non-revoked accounts, optional provider filter |
| `get_by_id` / `get_status` | Metadata reads |
| `set_default` | One default per provider |
| `revoke` | Mark revoked, clear pane bindings, reassign default |
| `credential_ref` | Internal keychain lookup (not exposed to UI) |

Supported API-key providers: `openai`, `anthropic`, `google`.

## Default Account Behavior

- First active account for a provider becomes default automatically.
- `account_create_api_key(is_default: true)` sets default explicitly.
- `AccountRepository::set_default` clears other defaults for the same provider.
- Disconnecting the default promotes the next active account if one exists.

## Provider Resolution Behavior

Resolution follows `Provider -> Account -> CredentialHandle -> LLMProvider`:

- If `panes.account_id` exists, that account is used.
- If `panes.account_id` is `NULL`, the provider's active default account is used.
- Missing explicit accounts and providers with no default account return `ProviderResolutionError { code: "no_account" }`.
- Non-active accounts return `ProviderResolutionError { code: "inactive_account" }`.
- Unsupported provider types return `ProviderResolutionError { code: "unsupported_provider" }`.
- Provider stubs are still non-executing; `send` and `stream` return `NotImplemented`.

## Out of Scope

- OAuth flows
- Provider execution / streaming
- Provider API networking
- Model execution
- Account UI (Phase 3C)
