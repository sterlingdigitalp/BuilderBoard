# Phase 3B — Google OAuth Implementation

**Status:** Complete  
**Scope:** Google OAuth only (PKCE, loopback callback, token exchange, keychain storage, refresh)

## Deliverables

| Component | Path | Description |
|-----------|------|-------------|
| Google OAuth config migration | `migrations/0003_google_oauth_config.sql`, `0004_google_oauth_scopes_fix.sql` | Seeds `oauth_config_json`; scopes `openid` + `email` only |
| OAuth service | `src-tauri/src/auth/oauth_service.rs` | PKCE, loopback server, token exchange, userinfo, refresh |
| OAuth commands | `src-tauri/src/auth/commands.rs` | `oauth_start`, `oauth_cancel` |
| Credential extensions | `src-tauri/src/auth/credential_service.rs` | OAuth token payload storage in Keychain |
| Accounts repository | `src-tauri/src/storage/repositories/accounts.rs` | `create_oauth_account`, token metadata updates |
| Providers repository | `src-tauri/src/storage/repositories/providers.rs` | `get_oauth_config` |
| Event DTOs | `src-tauri/src/storage/models.rs` | `OAuthStartResult`, `OAuthCompleteEvent`, `OAuthErrorEvent` |
| Account-aware provider resolution | `src-tauri/src/chat/mod.rs` | Explicit/default OAuth accounts resolve to `CredentialHandle` + `LLMProvider` |

## Commands

| Command | Input | Output |
|---------|-------|--------|
| `oauth_start` | `provider_id` | `{ authUrl }` — opens system browser |
| `oauth_cancel` | `provider_id` | void — cancels pending flow |

## Events

| Event | Payload |
|-------|---------|
| `oauth_complete` | `{ accountId, providerId, label }` |
| `oauth_error` | `{ providerId, errorCode, message }` |

## Security

- **PKCE S256** — `code_challenge = BASE64URL(SHA256(code_verifier))`
- **State validation** — CSRF protection on callback
- **Loopback only** — binds `127.0.0.1` ephemeral port (49152–65535)
- **System browser** — macOS `open` command (no embedded WebView)
- **Secrets in Keychain** — OAuth tokens never in SQLite or IPC

## Configuration

Set the Google OAuth Desktop App credentials from the downloaded `client_secret.json`:

```bash
export BUILDERBOARD_GOOGLE_CLIENT_ID="your-client-id.apps.googleusercontent.com"
export BUILDERBOARD_GOOGLE_CLIENT_SECRET="your-client-secret"
```

Google Desktop App token exchange requires `client_secret` in the POST body alongside PKCE `code_verifier`.

Redirect URI registered with Google must allow loopback: `http://127.0.0.1:<port>/callback`

## Token Lifecycle

1. **Exchange** — authorization code + PKCE verifier → access/refresh tokens
2. **Storage** — Keychain JSON: `{ access_token, refresh_token, token_type, expires_at }`
3. **Account row** — `auth_type = oauth`, `credential_ref` → Keychain, metadata in SQLite
4. **Refresh** — proactive refresh when `expires_at < now + 5min` via `OAuthService::refresh_oauth_access_token`
5. **Disconnect** — deletes Keychain entry, sets `status = revoked`, clears pane bindings

## Provider Resolution Integration

Resolution remains `Provider -> Account -> CredentialHandle -> LLMProvider`.

- Explicit `panes.account_id` is used when present.
- Otherwise the provider default account is used.
- `auth_type = 'oauth'` creates the same `CredentialHandle` shape as `api_key` accounts.
- OAuth handles include `token_expires_at` metadata when present.
- `status = 'expired'` returns `expired_account`.
- Active OAuth accounts with expired `token_expires_at` return `expired_account`.
- Other non-active accounts return `inactive_account`.
- Missing account paths return `no_account`.
- Unsupported providers return `unsupported_provider`.

## Out of Scope (Phase 3B)

- OpenAI / Anthropic OAuth
- Provider execution / streaming
- Model execution
- Provider API networking
- Windows / Linux OAuth

## Validation Scenarios

| Scenario | Test |
|----------|------|
| Start OAuth flow | `google_oauth_flow_completes_with_callback` |
| Browser launches | Mock browser records opened URL |
| Callback received | Loopback HTTP GET simulation |
| State validated | `oauth_rejects_state_mismatch` |
| Token exchanged | Mock HTTP client records exchange |
| Keychain entry created | `credential_exists` after flow |
| Account row created | `create_oauth_account` via flow |
| Refresh path | `oauth_refresh_updates_keychain_and_account` |
| Disconnect | `oauth_disconnect_removes_keychain_entry` |
| Cancel flow | `oauth_cancel_emits_cancelled_error` |
| PKCE S256 | `pkce_challenge_uses_s256` |
| Explicit Google OAuth resolution | `resolves_google_account` |
| Default Google OAuth resolution | `resolves_google_default_oauth_account` |
| Expired OAuth rejected | `expired_oauth_token_is_rejected` |

## Dependencies Added

- `reqwest` (blocking, rustls)
- `sha2`, `base64`, `rand`, `urlencoding`
