CREATE TABLE IF NOT EXISTS plugin_states (
    plugin_id TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO plugin_states (plugin_id, enabled)
VALUES ('banshee.prompt-enhancer', 0);

ALTER TABLE transcriptions ADD COLUMN plugin_runs_json TEXT NOT NULL DEFAULT '[]';

