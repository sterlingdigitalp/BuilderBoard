# Provider Model

The provider model defines a stable abstraction for future LLM integrations. Phase 4A keeps `LLMProvider` unchanged and adds real OpenAI execution for API-key accounts only.

## Core Trait

`LLMProvider` is the provider-facing contract.

```rust
pub trait LLMProvider {
    fn send(&self, request: ProviderRequest) -> ProviderResult<ProviderResponse>;
    fn stream(&self, request: ProviderRequest) -> ProviderResult<ProviderStream>;
    fn list_models(&self) -> ProviderResult<Vec<Model>>;
}
```

## Provider Implementations

- `AnthropicProvider`: Placeholder for Anthropic integrations.
- `OpenAIProvider`: OpenAI Chat Completions implementation for API-key accounts.
- `GoogleProvider`: Placeholder for Google integrations.

Anthropic and Google `send` and `stream` return `ProviderError::NotImplemented`. OpenAI `send` and `stream` execute real HTTPS requests when constructed through the execution resolver with an API key.

## OpenAI Request Contract

- Endpoint: `POST https://api.openai.com/v1/chat/completions`
- Content-Type: `application/json`
- Authorization: `Bearer <api key>`
- Default model for `Model::OpenAIGpt`: `gpt-4o-mini`
- Request body:

```json
{
  "model": "gpt-4o-mini",
  "messages": [
    { "role": "user", "content": "Hello" }
  ],
  "stream": false
}
```

Streaming uses the same endpoint with `"stream": true` and parses Server-Sent Events of the form `data: {...}` until `data: [DONE]`. Each `choices[0].delta.content` value becomes a normalized `StreamChunk`.

## Registry And Account Resolution

Provider registry rows are loaded from SQLite via the storage provider repository. Resolution uses `provider_type`, not display name, to select an `LLMProvider` implementation.

| provider_type | Resolution |
|---------------|------------|
| `anthropic` | `AnthropicProvider` |
| `openai` | `OpenAIProvider` |
| `google` | `GoogleProvider` |

The Phase 3B resolver intentionally rejects non-MVP providers including `openrouter`, `ollama`, and `lmstudio`. Unsupported rows return a structured `ProviderResolutionError` with code `unsupported_provider`, preserving the provider id and provider type for callers.

`provider_list` returns enabled registry rows for UI selection. It does not instantiate providers or call provider APIs.

Account-aware resolution follows this order:

1. Load the pane and enabled provider registry row.
2. Use `pane.account_id` if present.
3. Otherwise use `AccountRepository::get_default_for_provider`.
4. Reject missing accounts with `no_account`.
5. Reject expired accounts with `expired_account`.
6. Reject other non-active accounts with `inactive_account`.
7. Create a `CredentialHandle` from account metadata and return it with the selected `LLMProvider` stub.

`CredentialHandle` carries account metadata, `auth_type`, the opaque `credential_ref`, and optional OAuth expiry metadata. Provider stubs still do not receive raw API keys or OAuth tokens.

OpenAI execution uses a separate resolver from the non-execution path. The execution resolver reads the API key from `CredentialService` and binds it to `OpenAIProvider`; raw keys are not returned in DTOs, events, logs, or frontend-facing data.

## Shared Types

- `Provider`: Provider identifier enum with `Anthropic`, `OpenAI`, and `Google` variants.
- `Model`: Provider-neutral model enum with placeholder variants for Claude, GPT, Gemini, and custom model identifiers.
- `Message`: Provider-neutral chat message with a role and content.
- `Conversation`: Provider-neutral conversation containing an id, selected model, and message history.
- `ProviderRequest`: Wrapper for provider input.
- `ProviderResponse`: Normalized provider output.
- `StreamChunk`: Normalized streaming delta placeholder.
- `ProviderResolutionError`: Structured resolver error for unsupported, missing, inactive, expired, or storage-backed provider resolution failures.
- `ResolvedProvider`: Provider stub plus `CredentialHandle` returned by account-aware resolution.

## Boundary Rules

- Provider implementations must not expose provider-specific request or response schemas outside the `providers` module boundary.
- Provider implementations must normalize all output into shared `models` types before returning to callers.
- Authentication and token storage remain outside provider implementations; providers receive only the resolved credential handle at construction/resolution boundaries.
- Provider code must not directly persist conversations; persistence belongs to the `storage` boundary.
- Streaming must return normalized `StreamChunk` values rather than provider-native events.
- Anthropic and Google execution remain out of scope.
- OpenAI execution must not add tools, images, function calling, or file uploads.
