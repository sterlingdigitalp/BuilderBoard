# Phase 4A OpenAI Provider Execution

## Status

Implemented for OpenAI API-key accounts only. Anthropic execution, Google execution, OAuth execution, tools, images, function calling, and file uploads are out of scope.

## Request Contract

- Endpoint: `POST https://api.openai.com/v1/chat/completions`
- Content-Type: `application/json`
- Authorization: `Bearer <api key>`
- Default model: `gpt-4o-mini`
- Non-stream body includes `"stream": false`
- Stream body includes `"stream": true`

```json
{
  "model": "gpt-4o-mini",
  "messages": [
    { "role": "user", "content": "Hello" }
  ],
  "stream": true
}
```

## Execution Flow

1. Pane resolves provider and account through existing storage repositories.
2. `CredentialHandle` identifies the API-key account and opaque `credential_ref`.
3. Execution resolver reads the API key through `CredentialService`.
4. The API key is bound to `OpenAIProvider` internally.
5. `OpenAIProvider::send` or `OpenAIProvider::stream` calls Chat Completions.
6. Streaming chunks are normalized into `StreamChunk` values.
7. `ChatExecutionService` persists the user message, assistant placeholder, stream deltas, and completion status.

## Error Handling

- Missing API key: `ProviderError::MissingCredentials`.
- HTTP failure: `ProviderError::Http { status, message }`.
- Malformed OpenAI response: `ProviderError::InvalidResponse`.
- Missing account: `ProviderResolutionError { code: "no_account" }` before execution.
- Non-OpenAI provider: `ProviderResolutionError { code: "unsupported_provider" }` before execution.

## Streaming Summary

OpenAI streaming is parsed from Server-Sent Events. Each `data: { ... }` event is decoded and `choices[0].delta.content` is emitted as a `StreamChunk`. `data: [DONE]` emits a completion chunk and ends the iterator.
