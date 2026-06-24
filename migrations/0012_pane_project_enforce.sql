-- Phase 8A Option C: remove rejected columns; enforce project_id via trigger.
ALTER TABLE panes DROP COLUMN project_name;
ALTER TABLE panes DROP COLUMN approved_root;

CREATE TRIGGER IF NOT EXISTS panes_require_project_id_insert
BEFORE INSERT ON panes
WHEN NEW.project_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'panes.project_id is required');
END;

CREATE TRIGGER IF NOT EXISTS panes_require_project_id_update
BEFORE UPDATE ON panes
WHEN NEW.project_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'panes.project_id is required');
END;