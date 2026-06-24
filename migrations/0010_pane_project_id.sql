-- Phase 8A Option C: pane-scoped project binding (schema only, no behavior change).
ALTER TABLE panes ADD COLUMN project_id TEXT REFERENCES workspaces (id);

CREATE INDEX IF NOT EXISTS idx_panes_project_id ON panes (project_id)
    WHERE closed_at IS NULL;