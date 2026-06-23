## 2026-06-23

### Frontend Framework

Decision:
React + TypeScript + Vite

Reason:
Fast iteration, Tauri compatibility

Status:
Accepted

---

### Phase 4A OpenAI Endpoint

Decision:
Use `POST https://api.openai.com/v1/chat/completions` with JSON body and `gpt-4o-mini` default model

Reason:
Matches the existing `LLMProvider` chat-shaped contract and supports both non-streaming and SSE streaming without adding tools or multimodal features

Status:
Accepted

---

### Phase 4A Execution Resolver

Decision:
Keep non-execution provider resolution unchanged and add a separate execution resolver that reads API keys through `CredentialService`

Reason:
UI/status flows can keep using credential handles while execution receives the secret only at the last responsible boundary

Status:
Accepted

---

### Desktop Framework

Decision:
Tauri 2.x

Reason:
Native desktop app with Rust backend

Status:
Accepted

---

### Persistence

Decision:
SQLite

Reason:
Local-first architecture

Status:
Accepted

---

### Secret Storage

Decision:
OS Keychain

Reason:
No secrets in SQLite

Status:
Accepted

---

### Provider Abstraction

Decision:
LLMProvider trait

Reason:
Provider-independent architecture

Status:
Accepted

---

### Phase Discipline

Decision:
No feature implementation outside assigned phase

Reason:
Prevent scope creep

Status:
Accepted

---

### Phase 2A Storage Layout

Decision:
Single `storage` module with embedded migrations via `include_str!`

Reason:
Reliable migration loading in dev and production bundles

Status:
Accepted

---

### Phase 2A Provider Seeds

Decision:
Seed only anthropic, openai, google in `0001_initial_schema`

Reason:
Match Phase 2A scope; additional providers deferred

Status:
Accepted

---

### Phase 3A Credential Store

Decision:
`keyring` crate with in-memory store for tests

Reason:
macOS Keychain integration with testable `CredentialStore` trait

Status:
Accepted

---

### Phase 3A Google API Keys

Decision:
Allow API-key accounts for `google` in Phase 3A despite OAuth-oriented seed metadata

Reason:
Phase 3A scope is API-key only; OAuth deferred to Phase 3B

Status:
Accepted

---

### Phase 3A Default Accounts

Decision:
`is_default` column per provider; one default per provider_id

Reason:
Pane resolution needs a stable default account per provider

Status:
Accepted

---

### Phase 3B Google OAuth Client ID

Decision:
Load `client_id` from `BUILDERBOARD_GOOGLE_CLIENT_ID` and `client_secret` from `BUILDERBOARD_GOOGLE_CLIENT_SECRET` (dev)

Reason:
Google Desktop App OAuth requires `client_secret` in token exchange even with PKCE; secret stays in backend env/keychain config only, never in SQLite or IPC

Status:
Accepted

---

### Phase 3B Loopback Callback

Decision:
Bind ephemeral port on `127.0.0.1` only; 5-minute timeout; one pending session per provider

Reason:
RFC 8252 compliant; localhost-only reduces hijack surface

Status:
Accepted

---

### Phase 3B System Browser

Decision:
Open authorization URL via macOS `open` command (no Tauri opener plugin)

Reason:
Avoid embedded WebView credential phishing; system browser is security requirement

Status:
Accepted

---

### Phase 3B Google OAuth Scopes

Decision:
Request `openid` and `email` only for account linking; omit Gemini API scopes

Reason:
`https://www.googleapis.com/auth/generative-language` is not a valid Google OAuth scope (returns `invalid_scope`). Valid Gemini scopes (`cloud-platform`, `generative-language.retriever`) are for API access, not identity, and belong in Phase 4

Status:
Accepted

---

### Phase 3B OAuth Credential Resolution

Decision:
Resolve OAuth accounts using the same `CredentialHandle` path as API-key accounts

Reason:
Provider adapters need one account resolution contract while `LLMProvider` remains unchanged

Status:
Accepted

---

### Phase 4A Message Lifecycle Commands

Decision:
Add `message_create`, `message_stream_update`, `message_complete`, and `message_error` as storage-layer commands; keep `message_append` for legacy complete inserts

Reason:
Streaming chat needs mutable assistant rows with explicit status transitions; provider execution (Phase 4B) will call these commands without changing the repository boundary

Status:
Accepted

---

### Phase 4A Assistant Placeholder

Decision:
`message_create` inserts user message (`complete`) and assistant placeholder (`pending`, empty content) atomically; assistant `parent_id` references the user message

Reason:
UI and provider layer need a stable assistant row ID before the first stream chunk arrives

Status:
Accepted

---

### Phase 4A Stream Updates

Decision:
`message_stream_update` appends `delta` to existing assistant `content` via SQL concatenation; first update transitions `pending` → `streaming`

Reason:
Incremental persistence matches provider chunk delivery without replacing prior content

Status:
Accepted
