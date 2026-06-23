# BuilderBoard OAuth Design

## Scope

This document defines OAuth authentication flows for BuilderBoard provider accounts. It is design-only — no OAuth implementation code, redirect handlers, or token storage code is created in this pass.

### Relationship to Other Documents

| Document | Relevance |
|----------|-----------|
| [DATABASE_DESIGN.md](./DATABASE_DESIGN.md) | `accounts` table, `providers.oauth_config_json` |
| [SECURITY_MODEL.md](./SECURITY_MODEL.md) | Keychain storage, threat model, capability boundaries |
| [BUILD_PLAN.md](./BUILD_PLAN.md) | Builder B owns adapters; this doc defines the auth contract they consume |

## Goals

1. Support OAuth-based provider accounts alongside API-key accounts.
2. Use desktop-safe OAuth 2.0 with PKCE (no client secret in distributed app).
3. Store tokens exclusively in macOS Keychain, referenced by `accounts.credential_ref`.
4. Allow multiple OAuth accounts per provider (e.g. personal and work Google accounts).
5. Integrate with Builder B provider resolution without embedding OAuth logic in adapters.

## Non-Goals

- Web-based OAuth (this is a desktop app)
- Shared/cloud credential sync
- OAuth for providers that only support API keys in v1 (OpenAI, Anthropic use API key path)
- Windows/Linux OAuth (macOS only for v1)

## Supported Auth Modes by Provider

| Provider | v1 Auth Mode | OAuth in v1? |
|----------|--------------|--------------|
| OpenAI | API key | No |
| Anthropic | API key | No |
| Google (Gemini) | OAuth | Yes |
| OpenRouter | API key | No |
| Ollama | None (local) | No |
| LM Studio | None (local) | No |

OAuth design is general enough to add more providers (e.g. GitHub Copilot, Microsoft) in later phases.

## Architecture Overview

```text
┌──────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Builder A   │     │  OAuth Service   │     │  macOS Keychain │
│  (Settings/  │────>│  (Tauri backend) │────>│  (token store)  │
│   Account UI)│     │                  │     └────────┬────────┘
└──────────────┘     └────────┬─────────┘              │
                              │                          │
                              v                          v
                     ┌────────────────┐          ┌───────────────┐
                     │ accounts table │<─────────│ credential_ref│
                     │ (metadata only)│          └───────────────┘
                     └────────┬───────┘
                              │
                              v
                     ┌────────────────┐
                     │ Builder B      │
                     │ CredentialSvc  │──> Provider Adapter
                     └────────────────┘
```

### Component Responsibilities

| Component | Owns |
|-----------|------|
| OAuth Service | Authorization URL construction, PKCE, callback capture, token exchange, refresh |
| Credential Service | Keychain read/write, `credential_ref` generation, token expiry checks |
| `accounts` repository | SQLite CRUD for account metadata |
| Builder B `LLMProvider` adapters | Consume resolved `CredentialHandle` with access token; no OAuth awareness |
| `auth` module | Expands in Phase 3; current `AuthSessionStore` is app-subject only, separate from provider accounts |

## OAuth 2.0 Flow (Authorization Code + PKCE)

### Step-by-Step

```text
1. User clicks "Connect Google" in account settings (Builder A UI)
2. UI invokes Tauri command: oauth_start(provider_id)
3. OAuth Service:
   a. Reads providers.oauth_config_json for authorization_url, scopes
   b. Generates code_verifier + code_challenge (S256)
   c. Generates state token (CSRF protection)
   d. Stores pending auth session in memory: { state, code_verifier, provider_id }
   e. Opens system browser to authorization URL with:
      - response_type=code
      - client_id (from build config / env)
      - redirect_uri
      - scope
      - state
      - code_challenge + code_challenge_method=S256
4. User authenticates in browser
5. Provider redirects to redirect_uri with ?code=...&state=...
6. Callback handler (loopback server or custom URL scheme):
   a. Validates state matches pending session
   b. Exchanges code + code_verifier for tokens at token_url
   c. Credential Service stores tokens in Keychain → credential_ref
   d. Repository inserts accounts row:
      - auth_type = 'oauth'
      - credential_ref = keychain key
      - external_account_id = provider subject/id
      - token_expires_at = computed from expires_in
      - scopes_json = granted scopes
   e. Emits Tauri event: oauth_complete { account_id, provider_id }
7. Builder A updates account list
```

### Redirect URI Options

| Method | URI Pattern | Pros | Cons |
|--------|-------------|------|------|
| **Loopback (recommended)** | `http://127.0.0.1:{port}/callback` | RFC 8252 compliant, no app registration for scheme | Must bind ephemeral port |
| **Custom URL scheme** | `builderboard://oauth/callback` | Simple registration | Less standard, macOS scheme handling |

**v1 recommendation:** Loopback server on `127.0.0.1` with ephemeral port. Port and `state` are communicated through the pending auth session. The loopback server shuts down after callback or timeout (5 minutes).

### PKCE Parameters

```
code_verifier:  43-128 char random URL-safe string
code_challenge: BASE64URL(SHA256(code_verifier))
method:         S256
```

## Token Lifecycle

### Storage Format (Keychain)

Service: `com.builderboard.app`
Account: `{credential_ref}` (matches `accounts.credential_ref`)

Payload (JSON, encrypted by Keychain):

```json
{
  "access_token": "ya29...",
  "refresh_token": "1//...",
  "token_type": "Bearer",
  "expires_at": "2026-06-23T16:00:00Z"
}
```

### Token Refresh

```text
1. Builder B requests credential for account_id
2. Credential Service reads keychain payload
3. If expires_at < now + 5min buffer:
   a. POST to token_url with grant_type=refresh_token
   b. Update keychain payload with new access_token, expires_at
   c. Update accounts.token_expires_at, accounts.updated_at
   d. On failure: set accounts.status = 'expired', return error
4. Return access_token to adapter
```

Refresh is synchronous before provider calls. Adapters never handle refresh directly.

### Token Revocation

| Trigger | Action |
|---------|--------|
| User disconnects account | Delete keychain entry, set `accounts.status = 'revoked'`, clear pane bindings |
| Refresh fails permanently | `accounts.status = 'expired'`, notify UI |
| Provider returns 401 | Attempt one refresh; on failure mark expired |

### Account Disconnection

```text
1. User clicks "Disconnect" on account
2. Optional: call provider revocation endpoint
3. Delete keychain entry for credential_ref
4. UPDATE accounts SET status = 'revoked', updated_at = now
5. UPDATE panes SET account_id = NULL WHERE account_id = :id
6. Emit account_disconnected event
```

## Provider-Specific Configuration

Stored in `providers.oauth_config_json` (public metadata only):

### Google (Gemini)

```json
{
  "authorization_url": "https://accounts.google.com/o/oauth2/v2/auth",
  "token_url": "https://oauth2.googleapis.com/token",
  "revocation_url": "https://oauth2.googleapis.com/revoke",
  "scopes": [
    "openid",
    "email"
  ],
  "userinfo_url": "https://www.googleapis.com/oauth2/v3/userinfo"
}
```

`client_id` and `client_secret` are loaded from:

1. `BUILDERBOARD_GOOGLE_CLIENT_ID` environment variable (development)
2. `BUILDERBOARD_GOOGLE_CLIENT_SECRET` environment variable (development)
3. Build-time embedded config (production)

Google **Desktop App** OAuth clients require `client_secret` in the token exchange POST body even when using PKCE ([native app docs](https://developers.google.com/identity/protocols/oauth2/native-app)). The secret is distributed with the desktop binary and is not sent to the frontend.

Phase 3B requests identity scopes only (`openid`, `email`). The scope `https://www.googleapis.com/auth/generative-language` is not a valid Google OAuth scope and causes `invalid_scope` errors. Gemini API access scopes (`cloud-platform`, `generative-language.retriever`) are deferred to Phase 4 provider execution.

### Future Provider Template

```json
{
  "authorization_url": "https://provider.com/oauth/authorize",
  "token_url": "https://provider.com/oauth/token",
  "revocation_url": "https://provider.com/oauth/revoke",
  "scopes": ["scope1", "scope2"],
  "userinfo_url": "https://provider.com/oauth/userinfo",
  "extra_auth_params": {}
}
```

## API Key Accounts (Non-OAuth Path)

For providers using `auth_mode = 'api_key'`:

```text
1. User enters API key in settings UI
2. UI invokes: account_create_api_key(provider_id, label, api_key)
3. Credential Service:
   a. Generates credential_ref (UUID)
   b. Stores { "api_key": "..." } in Keychain
4. Repository inserts accounts row:
   - auth_type = 'api_key'
   - credential_ref = keychain key
   - token_expires_at = NULL
   - status = 'active'
5. API key is never returned to UI after creation
```

API key validation (optional, Phase 3): lightweight provider ping via Builder B adapter before marking account active.

## Tauri Commands (Proposed)

| Command | Input | Output | Phase |
|---------|-------|--------|-------|
| `oauth_start` | `provider_id` | `{ auth_url }` | 3 |
| `oauth_cancel` | `provider_id` | `void` | 3 |
| `account_create_api_key` | `provider_id, label, api_key` | `AccountDto` | 3 |
| `account_list` | `provider_id?` | `AccountDto[]` | 3 |
| `account_disconnect` | `account_id` | `void` | 3 |
| `account_get_status` | `account_id` | `{ status, expires_at }` | 3 |

OAuth callback is internal (loopback handler), not a Tauri command.

## Events (Proposed)

| Event | Payload | When |
|-------|---------|------|
| `oauth_complete` | `{ account_id, provider_id, label }` | Successful OAuth |
| `oauth_error` | `{ provider_id, error_code, message }` | OAuth failure |
| `account_status_changed` | `{ account_id, status }` | Expiry, revocation |
| `account_created` | `AccountDto` | API key or OAuth account created |

## AccountDto (Frontend Contract)

Returned by `account_list`, `account_create_api_key`, and `oauth_complete`. Must never include secrets.

```typescript
interface AccountDto {
  id: string;
  providerId: string;
  label: string;
  authType: 'oauth' | 'api_key';
  externalEmail: string | null;
  status: 'active' | 'expired' | 'revoked' | 'error';
  tokenExpiresAt: string | null;
  lastUsedAt: string | null;
  isDefault: boolean;
}
```

Excluded fields: `credential_ref`, `access_token`, `refresh_token`, `api_key`, `scopes_json` (internal only).

## UI Integration (Builder A Contract)

Builder A account settings UI (future) should:

1. List accounts from `account_list` grouped by provider.
2. Show OAuth "Connect" button when `providers.auth_mode = 'oauth'`.
3. Show API key input when `providers.auth_mode = 'api_key'`.
4. Display `external_email` or `label` for account identity.
5. Show `status` badge (active, expired, error).
6. Never display raw tokens or API keys.

Pane provider picker passes `account_id` to `pane_update_provider` when switching.

## Error Handling

| Error | User Message | System Action |
|-------|--------------|---------------|
| State mismatch | "Authentication failed. Please try again." | Discard pending session |
| Token exchange failed | "Could not connect account." | Log error code, no keychain write |
| Refresh failed | "Session expired. Reconnect your account." | `status = expired` |
| Loopback timeout (5 min) | "Authentication timed out." | Shutdown loopback server |
| Keychain write failed | "Could not save credentials securely." | Rollback accounts insert |

## Security Considerations

- `state` parameter prevents CSRF on callback.
- PKCE prevents authorization code interception.
- Pending auth sessions are in-memory only, never persisted.
- Loopback server binds to `127.0.0.1` only (not `0.0.0.0`).
- Browser is system default (not embedded WebView) to avoid credential phishing surface.
- See [SECURITY_MODEL.md](./SECURITY_MODEL.md) for full threat model.

## Builder B Integration

Per [PROVIDER_MODEL.md](./PROVIDER_MODEL.md), authentication must remain outside provider implementations. OAuth account linking extends the `auth` module; provider adapters receive credentials only through a `CredentialHandle` at call time:

Adapters receive credentials through a `CredentialHandle`:

```text
CredentialHandle {
  provider_id: String,
  account_id: String,
  auth_type: oauth | api_key,
  credential_ref: String,          // opaque Keychain reference
  token_expires_at: Option<String> // OAuth expiry metadata, no token value
}
```

Builder B adapters must not:

- Open browsers or handle redirects
- Read Keychain directly
- Store tokens in memory beyond request scope

**Credential delivery pattern:** The `chat` boundary resolves `CredentialHandle`, then constructs the provider adapter with credentials bound at instantiation (e.g. `OpenAIProvider::with_credentials(handle)`). The `LLMProvider` trait methods (`send`, `stream`, `list_models`) remain unchanged; credentials are not fields on `ProviderRequest`.

Phase 3B credential integration ensures explicit and default OAuth accounts resolve through the same `Provider -> Account -> CredentialHandle -> LLMProvider` path as API-key accounts. Expired OAuth account rows or expired `token_expires_at` metadata return structured `expired_account` errors before any provider execution.

## Migration / Schema Compatibility

OAuth accounts use the `accounts` table defined in [DATABASE_DESIGN.md](./DATABASE_DESIGN.md). No separate OAuth table is needed.

Adding OAuth to a new provider:

1. Update `providers.auth_mode` to `oauth` (migration seed or upsert).
2. Add `oauth_config_json` for the provider.
3. No schema migration required.

## Testing Strategy (Future Implementation)

| Test | Validates |
|------|-----------|
| PKCE challenge generation | Correct S256 encoding |
| State validation | Rejects mismatched callback |
| Token exchange mock | Correct POST body to token_url |
| Keychain round-trip | Store, read, delete credential |
| Refresh before expiry | Proactive refresh within 5min buffer |
| Account disconnect | Keychain deleted, panes unbound |
| Expired account | Provider call returns actionable error to UI |

## Risks

| Risk | Mitigation |
|------|------------|
| Google OAuth client registration | Document dev setup; use env var for client_id |
| Loopback port conflict | Retry on next available port; include port in redirect_uri |
| Keychain access denied | Clear error message; link to macOS Keychain Access permissions |
| Multiple concurrent OAuth flows | One pending session per provider_id; cancel previous on new start |
| Token refresh race | Mutex on credential refresh per account_id |
