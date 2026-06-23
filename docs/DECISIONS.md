## 2026-06-23

### Frontend Framework

Decision:
React + TypeScript + Vite

Reason:
Fast iteration, Tauri compatibility

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
Load `client_id` from `BUILDERBOARD_GOOGLE_CLIENT_ID` env var (dev); no client secret (PKCE public client)

Reason:
Desktop-safe OAuth per OAUTH_DESIGN; secrets not embedded in distributed binary

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

### Phase 3B OAuth Credential Resolution

Decision:
Resolve OAuth accounts using the same `CredentialHandle` path as API-key accounts

Reason:
Provider adapters need one account resolution contract while `LLMProvider` remains unchanged

Status:
Accepted
