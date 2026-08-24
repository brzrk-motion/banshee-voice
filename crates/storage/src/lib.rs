//! SQLite storage services for Banshee.

pub mod migrations;
pub mod profile_repo;
pub mod settings_repo;
pub mod transcription_repo;

use anyhow::{Context, Result};
use banshee_core::domain::AppPaths;
use rusqlite::Connection;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use profile_repo::SqliteProfileRepository;
pub use settings_repo::SqliteSettingsRepository;
pub use transcription_repo::SqliteTranscriptionRepository;

pub fn resolve_app_paths() -> Result<AppPaths> {
    let data_dir = resolve_data_dir()?;
    let database_path = data_dir.join("banshee.db");
    Ok(AppPaths {
        data_dir,
        database_path,
    })
}

pub fn resolve_data_dir() -> Result<PathBuf> {
    if let Ok(explicit) = env::var("BANSHEE_APP_DATA_DIR") {
        let path = PathBuf::from(explicit);
        fs::create_dir_all(&path)?;
        return Ok(path);
    }

    #[cfg(target_os = "linux")]
    let path = if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data_home).join("banshee")
    } else {
        let home = env::var("HOME")
            .context("HOME is not set and BANSHEE_APP_DATA_DIR was not provided")?;
        PathBuf::from(home).join(".local/share/banshee")
    };

    #[cfg(target_os = "macos")]
    let path = {
        let home = env::var("HOME")
            .context("HOME is not set and BANSHEE_APP_DATA_DIR was not provided")?;
        PathBuf::from(home).join("Library/Application Support/Banshee")
    };

    #[cfg(target_os = "windows")]
    let path = if let Ok(app_data) = env::var("APPDATA") {
        PathBuf::from(app_data).join("Banshee")
    } else {
        let user_profile = env::var("USERPROFILE").context(
            "APPDATA and USERPROFILE are not set and BANSHEE_APP_DATA_DIR was not provided",
        )?;
        PathBuf::from(user_profile).join("AppData/Roaming/Banshee")
    };

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let path = env::current_dir()?.join(".banshee-data");

    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let connection = Connection::open(path)
        .with_context(|| format!("failed to open SQLite database at {}", path.display()))?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(connection)
}

pub fn migrate(connection: &mut Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    let transaction = connection.transaction()?;
    for migration in migrations::all() {
        let applied = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [migration.version],
            |row| row.get::<_, i64>(0),
        )?;

        if applied == 0 {
            transaction.execute_batch(migration.sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![migration.version, migration.name],
            )?;
        }
    }
    transaction.commit()?;
    Ok(())
}

pub fn initialize_storage() -> Result<StorageRuntime> {
    let paths = resolve_app_paths()?;
    let mut connection = open_connection(&paths.database_path)?;
    migrate(&mut connection)?;
    let connection = Arc::new(Mutex::new(connection));

    let settings = SqliteSettingsRepository::new(connection.clone());
    settings.seed_default()?;

    let profiles = SqliteProfileRepository::new(connection.clone());
    profiles.seed_builtin_profiles()?;

    let transcriptions = SqliteTranscriptionRepository::new(connection.clone());

    Ok(StorageRuntime {
        paths,
        connection,
        settings,
        profiles,
        transcriptions,
    })
}

#[derive(Clone)]
pub struct StorageRuntime {
    pub paths: AppPaths,
    pub connection: Arc<Mutex<Connection>>,
    pub settings: SqliteSettingsRepository,
    pub profiles: SqliteProfileRepository,
    pub transcriptions: SqliteTranscriptionRepository,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_background_migration_normalizes_existing_preferences() {
        let mut connection = Connection::open_in_memory().expect("database should open");
        migrate(&mut connection).expect("migrations should apply");
        connection
            .execute(
                "INSERT INTO settings (
                    id, push_to_talk_shortcut, toggle_recording_shortcut, cancel_shortcut,
                    repaste_previous_shortcut, acceleration_preference, audio_retention_policy,
                    show_hud, minimize_to_tray, auto_paste_enabled
                 ) VALUES (
                    1, 'Ctrl+Shift+Space', 'Ctrl+Shift+R', 'Escape', 'Ctrl+Shift+V',
                    'auto', 'never', 0, 0, 0
                 )",
                [],
            )
            .expect("legacy settings should insert");
        connection
            .execute("DELETE FROM schema_migrations WHERE version = 2", [])
            .expect("migration marker should reset");

        migrate(&mut connection).expect("HUD migration should reapply");

        let values = connection
            .query_row(
                "SELECT show_hud, minimize_to_tray, auto_paste_enabled FROM settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, bool>(2)?,
                    ))
                },
            )
            .expect("settings should load");
        assert_eq!(values, (true, true, true));
    }
}
