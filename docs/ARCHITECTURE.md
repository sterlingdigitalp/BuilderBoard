# BuilderBoard Backend Architecture

BuilderBoard's backend is organized as a Rust crate under `src-tauri`. Phase 3B extends account-aware provider resolution to OAuth credentials while keeping provider execution, model API networking, streaming, and model execution out of scope.

## Module Boundaries

- `auth`: Authentication and credential boundary. It owns `CredentialHandle`, API-key and OAuth credential payloads, and credential-store abstractions.
- `providers`: LLM provider abstraction boundary. It owns provider traits, provider identifiers, request/response envelopes, stream chunk types, provider-specific stubs, and registry-entry resolution for MVP providers.
- `models`: Shared domain model boundary. It owns provider-neutral model identifiers, messages, roles, and conversations.
- `chat`: Conversation orchestration boundary. It resolves `panes.provider_id` and `panes.account_id` into `Provider -> Account -> CredentialHandle -> LLMProvider`. It does not execute models.
- `storage`: Persistence boundary. It owns SQLite initialization, migrations, repository modules, and Tauri commands backed by the local database.
- `sidecar`: External process boundary. It exposes process contracts only; no spawning, IPC, or lifecycle management exists in this phase.

## Current Guarantees

- The backend modules compile independently of any UI code.
- Provider stubs make no network calls.
- No OAuth flow is implemented.
- Provider registry rows load from the `providers` table.
- `provider_list` returns enabled providers for the UI picker.
- `provider_type` values `anthropic`, `openai`, and `google` resolve to MVP provider stubs.
- Unsupported provider types return structured `unsupported_provider` errors.
- Explicit `pane.account_id` takes precedence over provider defaults.
- Panes without `account_id` use the provider's active default account.
- Missing, inactive, and expired accounts return structured `no_account`, `inactive_account`, and `expired_account` errors.
- `CredentialHandle` supports both `api_key` and `oauth` account types without exposing raw secrets to providers.

## Planned Flow

1. UI calls `provider_list` to read enabled provider registry rows from SQLite.
2. Chat orchestration reads the pane from `storage` and uses `panes.provider_id` to load the enabled provider row.
3. If `panes.account_id` exists, chat resolves that account; otherwise it resolves the provider default account.
4. Chat rejects expired account status or expired OAuth `token_expires_at` before provider construction.
5. Chat converts account metadata into a `CredentialHandle` containing `auth_type`, opaque `credential_ref`, and optional token expiry metadata.
6. The provider resolver maps the row's `provider_type` to an `LLMProvider` implementation for `anthropic`, `openai`, or `google` and returns it with the credential handle.
7. Unsupported provider rows such as `openrouter`, `ollama`, or `lmstudio` fail with a structured error instead of falling back silently.

## Non-Goals For This Phase

- No frontend or UI work.
- No provider API networking.
- No provider API calls or credential validation calls.
- No streaming.
- No model execution.
- No sidecar process execution.
