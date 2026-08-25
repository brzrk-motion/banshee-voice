use anyhow::Result;
use banshee_contracts::domain::PluginStateStore;
use rusqlite::{Connection, params};
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
}
