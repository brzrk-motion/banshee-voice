ALTER TABLE plugin_states
ADD COLUMN settings_json TEXT NOT NULL DEFAULT '{}';
