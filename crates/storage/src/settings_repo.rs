use anyhow::{Context, Result};
use banshee_contracts::domain::{
    AccelerationPreference, AudioRetentionPolicy, Settings, SettingsStore, SettingsUpdate,
};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettingsValidationError {
    #[error("vad sensitivity must be between 0.0 and 1.0")]
    InvalidVadSensitivity,
    #[error("paste delay must be between 0 and 2000 ms")]
    InvalidPasteDelay,
}

#[derive(Clone)]
pub struct SqliteSettingsRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteSettingsRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    pub fn seed_default(&self) -> Result<()> {
        let settings = Settings::default();
        let connection = self.connection.lock().expect("settings mutex poisoned");
        connection.execute(
            "INSERT OR IGNORE INTO settings (
                id, launch_at_login, start_minimized, minimize_to_tray, show_hud,
                play_start_sound, play_completion_sound, microphone_device_id, vad_sensitivity,
                push_to_talk_shortcut, toggle_recording_shortcut, cancel_shortcut,
                repaste_previous_shortcut, acceleration_preference, history_enabled,
                audio_retention_policy, auto_paste_enabled, preserve_clipboard,
                paste_delay_ms, cleanup_llm_enabled, updated_at
            ) VALUES (
                1, ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11,
                ?12, ?13, ?14,
                ?15, ?16, ?17,
                ?18, ?19, CURRENT_TIMESTAMP
            )",
            params![
                settings.launch_at_login,
                settings.start_minimized,
                settings.minimize_to_tray,
                settings.show_hud,
                settings.play_start_sound,
                settings.play_completion_sound,
                settings.microphone_device_id,
                settings.vad_sensitivity,
                settings.push_to_talk_shortcut,
                settings.toggle_recording_shortcut,
                settings.cancel_shortcut,
                settings.repaste_previous_shortcut,
                encode_acceleration(settings.acceleration_preference),
                settings.history_enabled,
                encode_audio_retention(settings.audio_retention_policy),
                settings.auto_paste_enabled,
                settings.preserve_clipboard,
                settings.paste_delay_ms,
                settings.cleanup_llm_enabled,
            ],
        )?;
        Ok(())
    }

    fn row_to_settings(connection: &Connection) -> Result<Settings> {
        connection
            .query_row(
                "SELECT
                    launch_at_login, start_minimized, minimize_to_tray, show_hud,
                    play_start_sound, play_completion_sound, microphone_device_id, vad_sensitivity,
                    push_to_talk_shortcut, toggle_recording_shortcut, cancel_shortcut,
                    repaste_previous_shortcut, acceleration_preference, history_enabled,
                    audio_retention_policy, auto_paste_enabled, preserve_clipboard,
                    paste_delay_ms, cleanup_llm_enabled
                 FROM settings
                 WHERE id = 1",
                [],
                |row| {
                    Ok(Settings {
                        launch_at_login: row.get(0)?,
                        start_minimized: row.get(1)?,
                        minimize_to_tray: row.get(2)?,
                        show_hud: row.get(3)?,
                        play_start_sound: row.get(4)?,
                        play_completion_sound: row.get(5)?,
                        microphone_device_id: row.get(6)?,
                        vad_sensitivity: row.get(7)?,
                        push_to_talk_shortcut: row.get(8)?,
                        toggle_recording_shortcut: row.get(9)?,
                        cancel_shortcut: row.get(10)?,
                        repaste_previous_shortcut: row.get(11)?,
                        acceleration_preference: decode_acceleration(row.get::<_, String>(12)?),
                        history_enabled: row.get(13)?,
                        audio_retention_policy: decode_audio_retention(row.get::<_, String>(14)?),
                        auto_paste_enabled: row.get(15)?,
                        preserve_clipboard: row.get(16)?,
                        paste_delay_ms: row.get(17)?,
                        cleanup_llm_enabled: row.get(18)?,
                    })
                },
            )
            .context("failed to load settings row")
    }
}

impl SettingsStore for SqliteSettingsRepository {
    fn load(&self) -> Result<Settings> {
        let connection = self.connection.lock().expect("settings mutex poisoned");
        Self::row_to_settings(&connection)
    }

    fn update(&self, update: SettingsUpdate) -> Result<Settings> {
        if let Some(value) = update.vad_sensitivity
            && !(0.0..=1.0).contains(&value)
        {
            return Err(SettingsValidationError::InvalidVadSensitivity.into());
        }

        if let Some(value) = update.paste_delay_ms
            && !(0..=2_000).contains(&value)
        {
            return Err(SettingsValidationError::InvalidPasteDelay.into());
        }

        let mut current = self.load()?;

        if let Some(value) = update.launch_at_login {
            current.launch_at_login = value;
        }
        if let Some(value) = update.start_minimized {
            current.start_minimized = value;
        }
        if let Some(value) = update.minimize_to_tray {
            current.minimize_to_tray = value;
        }
        if let Some(value) = update.show_hud {
            current.show_hud = value;
        }
        if let Some(value) = update.play_start_sound {
            current.play_start_sound = value;
        }
        if let Some(value) = update.play_completion_sound {
            current.play_completion_sound = value;
        }
        if let Some(value) = update.microphone_device_id {
            current.microphone_device_id = value;
        }
        if let Some(value) = update.vad_sensitivity {
            current.vad_sensitivity = value;
        }
        if let Some(value) = update.push_to_talk_shortcut {
            current.push_to_talk_shortcut = value;
        }
        if let Some(value) = update.toggle_recording_shortcut {
            current.toggle_recording_shortcut = value;
        }
        if let Some(value) = update.cancel_shortcut {
            current.cancel_shortcut = value;
        }
        if let Some(value) = update.repaste_previous_shortcut {
            current.repaste_previous_shortcut = value;
        }
        if let Some(value) = update.acceleration_preference {
            current.acceleration_preference = value;
        }
        if let Some(value) = update.history_enabled {
            current.history_enabled = value;
        }
        if let Some(value) = update.audio_retention_policy {
            current.audio_retention_policy = value;
        }
        if let Some(value) = update.auto_paste_enabled {
            current.auto_paste_enabled = value;
        }
        if let Some(value) = update.preserve_clipboard {
            current.preserve_clipboard = value;
        }
        if let Some(value) = update.paste_delay_ms {
            current.paste_delay_ms = value;
        }
        if let Some(value) = update.cleanup_llm_enabled {
            current.cleanup_llm_enabled = value;
        }

        let connection = self.connection.lock().expect("settings mutex poisoned");
        connection.execute(
            "UPDATE settings SET
                launch_at_login = ?1,
                start_minimized = ?2,
                minimize_to_tray = ?3,
                show_hud = ?4,
                play_start_sound = ?5,
                play_completion_sound = ?6,
                microphone_device_id = ?7,
                vad_sensitivity = ?8,
                push_to_talk_shortcut = ?9,
                toggle_recording_shortcut = ?10,
                cancel_shortcut = ?11,
                repaste_previous_shortcut = ?12,
                acceleration_preference = ?13,
                history_enabled = ?14,
                audio_retention_policy = ?15,
                auto_paste_enabled = ?16,
                preserve_clipboard = ?17,
                paste_delay_ms = ?18,
                cleanup_llm_enabled = ?19,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = 1",
            params![
                current.launch_at_login,
                current.start_minimized,
                current.minimize_to_tray,
                current.show_hud,
                current.play_start_sound,
                current.play_completion_sound,
                current.microphone_device_id,
                current.vad_sensitivity,
                current.push_to_talk_shortcut,
                current.toggle_recording_shortcut,
                current.cancel_shortcut,
                current.repaste_previous_shortcut,
                encode_acceleration(current.acceleration_preference),
                current.history_enabled,
                encode_audio_retention(current.audio_retention_policy),
                current.auto_paste_enabled,
                current.preserve_clipboard,
                current.paste_delay_ms,
                current.cleanup_llm_enabled,
            ],
        )?;

        Ok(current)
    }
}

fn encode_acceleration(value: AccelerationPreference) -> &'static str {
    match value {
        AccelerationPreference::Auto => "auto",
        AccelerationPreference::Cpu => "cpu",
        AccelerationPreference::Gpu => "gpu",
    }
}

fn decode_acceleration(value: String) -> AccelerationPreference {
    match value.as_str() {
        "cpu" => AccelerationPreference::Cpu,
        "gpu" => AccelerationPreference::Gpu,
        _ => AccelerationPreference::Auto,
    }
}

fn encode_audio_retention(value: AudioRetentionPolicy) -> &'static str {
    match value {
        AudioRetentionPolicy::Never => "never",
        AudioRetentionPolicy::Hours24 => "24_hours",
        AudioRetentionPolicy::Forever => "forever",
    }
}

fn decode_audio_retention(value: String) -> AudioRetentionPolicy {
    match value.as_str() {
        "24_hours" => AudioRetentionPolicy::Hours24,
        "forever" => AudioRetentionPolicy::Forever,
        _ => AudioRetentionPolicy::Never,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_sound_preferences() {
        let mut connection = Connection::open_in_memory().expect("database should open");
        crate::migrate(&mut connection).expect("migration should apply");
        let repository = SqliteSettingsRepository::new(Arc::new(Mutex::new(connection)));
        repository.seed_default().expect("defaults should seed");

        let updated = repository
            .update(SettingsUpdate {
                play_start_sound: Some(true),
                play_completion_sound: Some(true),
                ..SettingsUpdate::default()
            })
            .expect("settings should update");

        assert!(updated.play_start_sound);
        assert!(updated.play_completion_sound);
        let loaded = repository.load().expect("settings should load");
        assert!(loaded.play_start_sound);
        assert!(loaded.play_completion_sound);
    }
}
