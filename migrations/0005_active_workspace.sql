-- Phase 5A active workspace tracking.

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO app_settings (key, value, updated_at)
VALUES (
    'active_workspace_id',
    '00000000-0000-4000-8000-000000000001',
    '2026-06-23T00:00:00Z'
);
