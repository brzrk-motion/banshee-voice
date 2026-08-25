use anyhow::Result;
use banshee_contracts::domain::PluginStateStore;
use rusqlite::{Connection, params};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SqlitePluginStateRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqlitePluginStateRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

impl PluginStateStore for SqlitePluginStateRepository {
    fn enabled(&self, plugin_id: &str) -> Result<bool> {
        let connection = self.connection.lock().expect("plugin state mutex poisoned");
        Ok(connection
            .query_row(
                "SELECT enabled FROM plugin_states WHERE plugin_id = ?1",
                [plugin_id],
                |row| row.get(0),
            )
            .unwrap_or(false))
    }

    fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let connection = self.connection.lock().expect("plugin state mutex poisoned");
        connection.execute(
            "INSERT INTO plugin_states (plugin_id, enabled, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(plugin_id) DO UPDATE SET enabled = excluded.enabled, updated_at = CURRENT_TIMESTAMP",
            params![plugin_id, enabled],
        )?;
        Ok(())
    }

    fn settings(&self, plugin_id: &str) -> Result<BTreeMap<String, String>> {
        let connection = self.connection.lock().expect("plugin state mutex poisoned");
        let stored = connection
            .query_row(
                "SELECT settings_json FROM plugin_states WHERE plugin_id = ?1",
                [plugin_id],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(stored
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    fn set_settings(&self, plugin_id: &str, settings: &BTreeMap<String, String>) -> Result<()> {
        let connection = self.connection.lock().expect("plugin state mutex poisoned");
        let json = serde_json::to_string(settings)?;
        connection.execute(
            "INSERT INTO plugin_states (plugin_id, enabled, settings_json, updated_at)
             VALUES (?1, 0, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(plugin_id) DO UPDATE SET
                settings_json = excluded.settings_json,
                updated_at = CURRENT_TIMESTAMP",
            params![plugin_id, json],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_without_changing_enabled_state() {
        let connection = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TABLE plugin_states (
                    plugin_id TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 0,
                    settings_json TEXT NOT NULL DEFAULT '{}',
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );",
            )
            .unwrap();
        let repository = SqlitePluginStateRepository::new(connection);
        repository.set_enabled("plugin", true).unwrap();
        let settings = BTreeMap::from([("targetModel".into(), "gpt-5.3-codex".into())]);

        repository.set_settings("plugin", &settings).unwrap();

        assert_eq!(repository.settings("plugin").unwrap(), settings);
        assert!(repository.enabled("plugin").unwrap());
    }
}
