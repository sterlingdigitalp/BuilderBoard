# BuilderBoard Backend Architecture

BuilderBoard's backend is organized as a Rust crate under `src-tauri`. Phase 2C adds provider registry persistence and provider resolution while keeping provider execution, OAuth, API keys, and account handling out of scope.

## Module Boundaries

- `auth`: Authentication session boundary. It exposes traits for representing authenticated subjects only. OAuth and credential exchange are intentionally out of scope.
- `providers`: LLM provider abstraction boundary. It owns provider traits, provider identifiers, request/response envelopes, stream chunk types, provider-specific stubs, and registry-entry resolution for MVP providers.
- `models`: Shared domain model boundary. It owns provider-neutral model identifiers, messages, roles, and conversations.
- `chat`: Conversation orchestration boundary. In Phase 2C it resolves a pane's persisted `provider_id` to an `LLMProvider` stub via the provider registry. It does not execute models.
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

## Planned Flow

1. UI calls `provider_list` to read enabled provider registry rows from SQLite.
2. Chat orchestration reads the pane from `storage` and uses `panes.provider_id` to load the enabled provider row.
3. The provider resolver maps the row's `provider_type` to an `LLMProvider` implementation for `anthropic`, `openai`, or `google`.
4. Unsupported provider rows such as `openrouter`, `ollama`, or `lmstudio` fail with a structured error instead of falling back silently.
5. Future phases will add credential resolution and model execution after this boundary.

## Non-Goals For This Phase

- No frontend or UI work.
- No real provider networking.
- No OAuth or token management.
- No API key or account handling.
- No model execution.
- No persistence behavior beyond loading seeded provider registry rows for selection and resolution.
- No sidecar process execution.
