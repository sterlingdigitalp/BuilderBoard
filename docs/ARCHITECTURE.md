# BuilderBoard Backend Architecture

BuilderBoard's backend is organized as a Rust crate under `src-tauri`. This phase only defines module boundaries and compile-time contracts for later implementation work.

## Module Boundaries

- `auth`: Authentication session boundary. It exposes traits for representing authenticated subjects only. OAuth and credential exchange are intentionally out of scope.
- `providers`: LLM provider abstraction boundary. It owns provider traits, provider identifiers, request/response envelopes, stream chunk types, and provider-specific stubs.
- `models`: Shared domain model boundary. It owns provider-neutral model identifiers, messages, roles, and conversations.
- `chat`: Conversation orchestration boundary. It will coordinate messages, providers, and storage in later phases.
- `storage`: Persistence boundary. It exposes storage traits only; no database, file, or key-value implementation exists in this phase.
- `sidecar`: External process boundary. It exposes process contracts only; no spawning, IPC, or lifecycle management exists in this phase.

## Current Guarantees

- The backend modules compile independently of any UI code.
- Provider stubs make no network calls.
- No OAuth flow is implemented.
- No persistence implementation is included.
- Public traits are placeholders intended to stabilize future integration points.

## Planned Flow

1. UI or command handlers will pass chat requests into the `chat` boundary.
2. `chat` will validate the conversation and choose an `LLMProvider` implementation.
3. Provider implementations will transform BuilderBoard models into provider-specific API requests.
4. Provider responses will be normalized back into `Message` and `Conversation` values.
5. `storage` will persist conversations once an implementation is selected.

## Non-Goals For This Phase

- No frontend or UI work.
- No real provider networking.
- No OAuth or token management.
- No persistence backend.
- No sidecar process execution.
