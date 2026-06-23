-- Phase 3A: default account support (additive)

ALTER TABLE accounts ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_accounts_provider_default
    ON accounts (provider_id, is_default)
    WHERE status = 'active';