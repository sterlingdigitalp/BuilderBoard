# Provider Model

The provider model defines a stable abstraction for future LLM integrations. Phase 2C adds registry-backed provider resolution without implementing real API behavior.

## Core Trait

`LLMProvider` is the provider-facing contract.

```rust
pub trait LLMProvider {
    fn send(&self, request: ProviderRequest) -> ProviderResult<ProviderResponse>;
    fn stream(&self, request: ProviderRequest) -> ProviderResult<ProviderStream>;
    fn list_models(&self) -> ProviderResult<Vec<Model>>;
}
```

## Provider Stubs

- `AnthropicProvider`: Placeholder for Anthropic integrations.
- `OpenAIProvider`: Placeholder for OpenAI integrations.
- `GoogleProvider`: Placeholder for Google integrations.

The `send` and `stream` methods return `ProviderError::NotImplemented` today. The `list_models` method returns static placeholder model identifiers and performs no external calls.

## Registry Resolution

Provider registry rows are loaded from SQLite via the storage provider repository. Resolution uses `provider_type`, not display name, to select an `LLMProvider` implementation.

| provider_type | Resolution |
|---------------|------------|
| `anthropic` | `AnthropicProvider` |
| `openai` | `OpenAIProvider` |
| `google` | `GoogleProvider` |

The Phase 2C resolver intentionally rejects non-MVP providers including `openrouter`, `ollama`, and `lmstudio`. Unsupported rows return a structured `ProviderResolutionError` with code `unsupported_provider`, preserving the provider id and provider type for callers.

`provider_list` returns enabled registry rows for UI selection. It does not instantiate providers, read credentials, or call provider APIs.

## Shared Types

- `Provider`: Provider identifier enum with `Anthropic`, `OpenAI`, and `Google` variants.
- `Model`: Provider-neutral model enum with placeholder variants for Claude, GPT, Gemini, and custom model identifiers.
- `Message`: Provider-neutral chat message with a role and content.
- `Conversation`: Provider-neutral conversation containing an id, selected model, and message history.
- `ProviderRequest`: Wrapper for provider input.
- `ProviderResponse`: Normalized provider output.
- `StreamChunk`: Normalized streaming delta placeholder.
- `ProviderResolutionError`: Structured resolver error for unsupported, missing, or storage-backed provider resolution failures.

## Boundary Rules

- Provider implementations must not expose provider-specific request or response schemas outside the `providers` module boundary.
- Provider implementations must normalize all output into shared `models` types before returning to callers.
- Authentication and token storage must remain outside provider implementations until the `auth` boundary is implemented.
- Provider code must not directly persist conversations; persistence belongs to the `storage` boundary.
- Streaming must return normalized `StreamChunk` values rather than provider-native events.
- Registry resolution must not add networking, OAuth, keychain access, account handling, streaming, or API-key logic.
