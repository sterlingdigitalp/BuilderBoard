# BuilderBoard Security Model

## Scope

This document defines the security architecture for BuilderBoard. It covers threat modeling, credential storage, data classification, Tauri capability boundaries, and security requirements for future implementation phases.

No security implementation code is created in this pass.

## Security Principles

1. **Secrets never in SQLite** — API keys, OAuth tokens, and refresh tokens live exclusively in macOS Keychain.
2. **Defense in depth** — Capability restrictions, input validation, and least-privilege access at each layer.
3. **Local-first trust boundary** — The app trusts the local user; threats are primarily credential leakage, supply chain, and local malware.
4. **Provider isolation** — Provider adapters receive credentials through a credential service; UI and persistence layers cannot read raw secrets.
5. **Auditability without exposure** — Message `metadata_json` records provider context but never stores credentials.

## Threat Model

### Assets

| Asset | Sensitivity | Storage |
|-------|-------------|---------|
| OAuth access/refresh tokens | Critical | Keychain only |
| API keys | Critical | Keychain only |
| Conversation messages | High | SQLite |
| Pane layout / workspace config | Medium | SQLite |
| Provider registry metadata | Low | SQLite (seeded) |
| Account labels / emails | Medium | SQLite |

### Threat Actors

| Actor | Capability | Primary Risk |
|-------|------------|--------------|
| Local malware | Read app files, memory, keychain (if unlocked) | Credential theft |
| Malicious provider response | Network injection, prompt injection | Data exfil via model |
| Compromised dependency | Supply chain | Arbitrary code execution |
| User error | Paste API key in chat, screenshot | Accidental exposure |
| Other local users | Access shared Mac account | Read SQLite (no keychain without password) |

### Out-of-Scope Threats (v1)

- Remote attacker without local access
- Multi-tenant server-side data breach (no server)
- Nation-state adversaries
- Physical device theft with FileVault disabled

## Data Classification

| Class | Examples | Handling |
|-------|----------|----------|
| **Secret** | API keys, OAuth tokens | Keychain, never logged, never in SQLite, never in IPC to frontend |
| **Sensitive** | Message content, system prompts | SQLite, local filesystem, not transmitted except to chosen provider |
| **Internal** | `credential_ref`, account status | SQLite, safe for logs (opaque identifier) |
| **Public** | Provider names, model names, OAuth authorization URLs | SQLite seeds, safe for UI display |

## Credential Storage Architecture

```text
┌─────────────────────────────────────────────────────────┐
│                    Frontend (Builder A)                  │
│  Never receives: api_key, access_token, refresh_token  │
│  Receives: account label, status, provider display name │
└────────────────────────┬────────────────────────────────┘
                         │ Tauri IPC (DTOs only)
┌────────────────────────▼────────────────────────────────┐
│                  Credential Service (Rust)               │
│  - generate_credential_ref()                             │
│  - store_credential(ref, payload)                        │
│  - resolve_credential(ref) → SecretString                │
│  - delete_credential(ref)                                │
│  - refresh_if_needed(account_id)                         │
└────────────┬───────────────────────┬────────────────────┘
             │                       │
             v                       v
┌────────────────────┐    ┌──────────────────────────┐
│  macOS Keychain    │    │  accounts table (SQLite)  │
│  Service:          │    │  credential_ref (opaque)  │
│  com.builderboard  │    │  NO secret columns        │
│  .app              │    └──────────────────────────┘
└────────────────────┘
```

### Keychain Entry Format

| Field | Value |
|-------|-------|
| Service | `com.builderboard.app` |
| Account | `{credential_ref}` (UUID) |
| Label | `BuilderBoard:{provider_id}:{label}` |
| Accessible | `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` |

`ThisDeviceOnly` prevents iCloud Keychain sync of credentials.

### Credential Reference Lifecycle

```text
Create:  UUID → keychain write → accounts.credential_ref = UUID
Resolve: accounts.credential_ref → keychain read → SecretString (zeroized on drop)
Delete:  keychain delete → accounts.status = 'revoked'
Rotate:  new keychain entry → update credential_ref → delete old entry
```

## Layer Security Boundaries

### Builder A (Frontend)

| Allowed | Prohibited |
|---------|------------|
| Display account labels, status, provider names | Store secrets in localStorage/sessionStorage |
| Send pane/message commands with account_id | Receive raw credentials in command responses |
| Render message content | Log message content to external services |
| Trigger OAuth via `oauth_start` command | Handle OAuth callback directly |

### Builder B (Provider Layer — `providers` module)

Per [PROVIDER_MODEL.md](./PROVIDER_MODEL.md), provider implementations normalize all output into `models` types and must not handle auth or persistence directly.

| Allowed | Prohibited |
|---------|------------|
| Receive opaque credential handles at adapter construction (Phase 3+; `chat` resolves CredentialHandle, binds to adapter instance before `send`/`stream`) | Persist raw credentials across requests |
| Make HTTPS calls to provider APIs (Phase 4) | Log request headers containing auth |
| Return normalized `StreamChunk` / `ProviderResponse` | Write credentials to SQLite |
| Report `ProviderError` (sanitized) | Execute shell commands from provider responses |
| Expose provider-specific schemas only inside `providers/` | Read Keychain or `accounts` table directly |

### Persistence Layer

| Allowed | Prohibited |
|---------|------------|
| Store messages, pane layout, account metadata | Columns for api_key, access_token, refresh_token |
| Store `credential_ref` opaque identifier | Return credential_ref to frontend in DTOs |
| Validate foreign key integrity | Export database with keychain contents |

## Tauri Capability Model

### Recommended Capabilities (v1, macOS)

| Capability | Scope | Justification |
|------------|-------|---------------|
| `core:default` | App lifecycle | Required |
| `shell:allow-open` | Open OAuth URL in system browser | OAuth flow |
| Custom: `http://127.0.0.1:*` | Loopback OAuth callback | Local only |
| Network | Provider API domains only | Restrict in Phase 4 |

### Prohibited Capabilities

| Capability | Reason |
|------------|--------|
| Arbitrary shell execution | Agent panes are out of scope for v1 |
| Full filesystem read/write | Only app data directory |
| Arbitrary network | Prevents exfiltration if frontend compromised |

### App Data Directory

```
~/Library/Application Support/com.builderboard.app/
  builderboard.db
  builderboard.db-wal
  builderboard.db-shm
  backups/
    builderboard.db.{timestamp}.bak
  logs/           (no secret content)
```

No sandbox (not Mac App Store). User's home directory access is limited to app data path in application code.

## Input Validation

### SQLite Inputs

| Field | Validation |
|-------|------------|
| UUIDs (`id`, `pane_id`, etc.) | Format: `^[0-9a-f-]{36}$` |
| `metadata_json` | Valid JSON, max 64 KB |
| `content` (messages) | Max 1 MB per message |
| `label` (accounts) | Max 128 chars, no control characters |
| `system_prompt` | Max 32 KB |

### Provider API Inputs

| Field | Validation |
|-------|------------|
| `model_id` | Alphanumeric + `.` `-` `_`, max 128 chars |
| `provider_id` | Must exist in `providers` table |
| `account_id` | Must exist, `status = active`, match pane's `provider_id`, and not be expired for OAuth |

## Provider Switching Security

When a user switches provider on a pane:

1. Validate new `provider_id` exists and is enabled.
2. Validate `account_id` belongs to same `provider_id` and is active.
3. Resolve credential before first message to new provider (fail fast if expired).
4. Do not migrate or expose messages from old provider context.
5. Record new provider context in next message's `metadata_json`.

No credential from the old account is passed to the new provider.

## OAuth Security

See [OAUTH_DESIGN.md](./OAUTH_DESIGN.md) for flow details. Security-specific requirements:

| Requirement | Implementation |
|-------------|----------------|
| CSRF protection | `state` parameter validated on callback |
| Code interception prevention | PKCE S256 |
| Redirect URI validation | Exact match against registered URI |
| Token storage | Keychain only, `WhenUnlockedThisDeviceOnly` |
| Browser isolation | System browser, not embedded WebView |
| Session timeout | 5-minute pending auth expiry |
| Refresh token rotation | Update keychain on every refresh response |

## Logging Policy

| Log Level | Allowed Content | Prohibited Content |
|-----------|-----------------|-------------------|
| ERROR | Error codes, pane_id, provider_id | Credentials, message content, tokens |
| WARN | Account status changes, migration issues | API keys, OAuth codes |
| INFO | Command names, migration versions | credential_ref resolution payloads |
| DEBUG | Disabled in production builds | Everything sensitive |

Structured log fields: `timestamp`, `level`, `command`, `pane_id`, `provider_id`, `error_code`.

## Database Security

### SQLite Hardening

| Setting | Value |
|---------|-------|
| `PRAGMA foreign_keys` | ON |
| `journal_mode` | WAL |
| File permissions | `0600` (owner read/write only) |
| Encryption at rest | Not in v1 (rely on FileVault); SQLCipher optional in future |

### Backup Security

Pre-migration backups stored in `backups/` with `0600` permissions. Backups contain messages but not keychain secrets. Restoring a backup does not restore credentials.

## Message Metadata Security

`metadata_json` may contain:

- Provider request IDs (for support)
- Token counts (for display)
- Tool call structures (for replay)

`metadata_json` must not contain:

- API keys or tokens
- Full provider request headers
- Raw provider error responses with embedded credentials

Repository layer strips known sensitive keys before persistence if adapters pass them.

## Incident Response (Local)

| Scenario | Response |
|----------|----------|
| Suspected key leak | User disconnects account (revokes keychain entry) |
| OAuth token compromise | Disconnect + reconnect account |
| Database corruption | Restore from `backups/` |
| Malware on machine | Outside app scope; credentials protected by Keychain + FileVault |

## Compliance Notes

- No telemetry or analytics in v1.
- User data stays on device.
- Provider API calls subject to each provider's terms of service.
- GDPR: user can delete all data by removing app data directory and keychain entries.

## Security Acceptance Criteria (Future Phases)

| Phase | Criteria |
|-------|----------|
| 2 (Persistence) | No secret columns in schema; file permissions 0600 |
| 3 (OAuth/Accounts) | Keychain round-trip; no token in IPC; PKCE validated |
| 4 (Streaming) | Auth headers not logged; HTTPS only for remote providers |
| 5 (Multi-workspace) | Workspace isolation in queries; no cross-workspace message leak |

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Keychain prompt fatigue | Low | Batch credential operations |
| Prompt injection via messages | Medium | User awareness; no auto-execution of model output |
| SQLite file readable by other apps | Medium | File permissions 0600; FileVault recommendation |
| Dependency vulnerability | High | `cargo audit`, `npm audit` in CI |
| OAuth loopback hijack | Low | State validation + short-lived sessions + localhost only |
| Builder A accidentally requests secrets | Medium | DTO type system excludes secret fields; code review |
