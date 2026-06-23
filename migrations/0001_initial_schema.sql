-- BuilderBoard initial schema (Phase 2A)
-- Additive, idempotent: safe to re-run via CREATE IF NOT EXISTS / INSERT OR IGNORE

CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    is_default INTEGER NOT NULL DEFAULT 0,
    layout_json TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_workspaces_slug ON workspaces (slug);
CREATE INDEX IF NOT EXISTS idx_workspaces_active ON workspaces (archived_at)
    WHERE archived_at IS NULL;

CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    provider_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    auth_mode TEXT NOT NULL,
    supports_chat INTEGER NOT NULL DEFAULT 1,
    supports_streaming INTEGER NOT NULL DEFAULT 1,
    supports_tool_use INTEGER NOT NULL DEFAULT 0,
    supports_vision INTEGER NOT NULL DEFAULT 0,
    context_window INTEGER,
    locality TEXT NOT NULL DEFAULT 'remote',
    oauth_config_json TEXT,
    configuration_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (auth_mode IN ('oauth', 'api_key', 'none', 'local')),
    CHECK (locality IN ('local', 'remote'))
);

CREATE INDEX IF NOT EXISTS idx_providers_type_enabled ON providers (provider_type, enabled);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    label TEXT NOT NULL,
    auth_type TEXT NOT NULL,
    credential_ref TEXT NOT NULL,
    external_account_id TEXT,
    external_email TEXT,
    token_expires_at TEXT,
    scopes_json TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    last_used_at TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (auth_type IN ('oauth', 'api_key')),
    CHECK (status IN ('active', 'expired', 'revoked', 'error')),
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_credential_ref ON accounts (credential_ref);
CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts (provider_id, status);

CREATE TABLE IF NOT EXISTS panes (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    title TEXT,
    role_label TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    width_ratio REAL,
    height_ratio REAL,
    provider_id TEXT,
    account_id TEXT,
    model_id TEXT,
    system_prompt TEXT,
    status TEXT NOT NULL DEFAULT 'idle',
    layout_json TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    CHECK (status IN ('idle', 'streaming', 'error')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces (id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE SET NULL,
    FOREIGN KEY (account_id) REFERENCES accounts (id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_panes_workspace_order ON panes (workspace_id, sort_order)
    WHERE closed_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_panes_provider ON panes (provider_id, account_id);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    pane_id TEXT NOT NULL,
    parent_id TEXT,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'text',
    status TEXT NOT NULL DEFAULT 'complete',
    provider_id TEXT,
    account_id TEXT,
    model_id TEXT,
    token_count_input INTEGER,
    token_count_output INTEGER,
    error_code TEXT,
    error_message TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    CHECK (status IN ('pending', 'streaming', 'complete', 'error')),
    CHECK (content_type IN ('text', 'markdown', 'json')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces (id) ON DELETE CASCADE,
    FOREIGN KEY (pane_id) REFERENCES panes (id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES messages (id) ON DELETE SET NULL,
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE SET NULL,
    FOREIGN KEY (account_id) REFERENCES accounts (id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_pane_created ON messages (pane_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_workspace_created ON messages (workspace_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_status ON messages (pane_id, status)
    WHERE status IN ('pending', 'streaming');

-- Default workspace (fixed id for idempotent seed)
INSERT OR IGNORE INTO workspaces (
    id, name, slug, is_default, created_at, updated_at
) VALUES (
    '00000000-0000-4000-8000-000000000001',
    'Default',
    'default',
    1,
    '2026-06-23T00:00:00Z',
    '2026-06-23T00:00:00Z'
);

-- Provider seeds (Phase 2A: anthropic, openai, google only)
INSERT OR IGNORE INTO providers (
    id, provider_type, display_name, enabled, auth_mode,
    supports_chat, supports_streaming, supports_tool_use, supports_vision,
    context_window, locality, created_at, updated_at
) VALUES
    ('anthropic', 'anthropic', 'Anthropic', 1, 'api_key', 1, 1, 1, 0, 200000, 'remote', '2026-06-23T00:00:00Z', '2026-06-23T00:00:00Z'),
    ('openai', 'openai', 'OpenAI', 1, 'api_key', 1, 1, 1, 1, 128000, 'remote', '2026-06-23T00:00:00Z', '2026-06-23T00:00:00Z'),
    ('google', 'google', 'Google', 1, 'oauth', 1, 1, 0, 1, 1000000, 'remote', '2026-06-23T00:00:00Z', '2026-06-23T00:00:00Z');