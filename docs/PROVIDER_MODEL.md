# Provider Model

The provider model defines a stable abstraction for future LLM integrations. Phase 3B resolves provider stubs with API-key or OAuth credential handles without implementing real provider API behavior.

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
- Registry and account resolution must not add provider API networking, streaming, or model execution.
