# Provider Model

The provider model defines a stable abstraction for future LLM integrations without implementing real API behavior in this phase.

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

## Shared Types

- `Provider`: Provider identifier enum with `Anthropic`, `OpenAI`, and `Google` variants.
- `Model`: Provider-neutral model enum with placeholder variants for Claude, GPT, Gemini, and custom model identifiers.
- `Message`: Provider-neutral chat message with a role and content.
- `Conversation`: Provider-neutral conversation containing an id, selected model, and message history.
- `ProviderRequest`: Wrapper for provider input.
- `ProviderResponse`: Normalized provider output.
- `StreamChunk`: Normalized streaming delta placeholder.

## Boundary Rules

- Provider implementations must not expose provider-specific request or response schemas outside the `providers` module boundary.
- Provider implementations must normalize all output into shared `models` types before returning to callers.
- Authentication and token storage must remain outside provider implementations until the `auth` boundary is implemented.
- Provider code must not directly persist conversations; persistence belongs to the `storage` boundary.
- Streaming must return normalized `StreamChunk` values rather than provider-native events.
