//! Core orchestration services for Banshee.

pub mod pipeline;

pub use banshee_audio as audio;
pub use banshee_context as context;
pub use banshee_contracts::domain;
pub use banshee_dictionary as dictionary;
pub use banshee_history as history;
pub use banshee_injector as injector;
pub use banshee_models as models;
pub use banshee_platform as platform;
pub use banshee_plugins as plugins;
pub use banshee_storage as storage;
pub use banshee_stt as stt;
pub use banshee_transformer as transformer;
pub use banshee_vad as vad;

use std::sync::Arc;

use anyhow::Result;
use domain::{
    AppPaths, AudioInputDevice, DashboardSnapshot, PlatformCapabilities, ProfileStore, Settings,
    SettingsStore, SettingsUpdate,
};
use pipeline::RecordingPipeline;

pub struct AppServices {
    settings: Arc<dyn SettingsStore>,
    profiles: Arc<dyn ProfileStore>,
    capabilities: PlatformCapabilities,
    paths: AppPaths,
    recording_pipeline: Arc<RecordingPipeline>,
}

impl AppServices {
    pub fn new(
        settings: Arc<dyn SettingsStore>,
        profiles: Arc<dyn ProfileStore>,
        capabilities: PlatformCapabilities,
        paths: AppPaths,
        recording_pipeline: Arc<RecordingPipeline>,
    ) -> Self {
        Self {
            settings,
            profiles,
            capabilities,
            paths,
            recording_pipeline,
        }
    }

    pub fn settings(&self) -> Result<Settings> {
        self.settings.load()
    }

    pub fn update_settings(&self, update: SettingsUpdate) -> Result<Settings> {
        self.settings.update(update)
    }

    pub fn dashboard_snapshot(&self) -> Result<DashboardSnapshot> {
        let settings = self.settings()?;
        let profile = self.profiles.default_profile()?;
        let microphone_name = self
            .audio_input_devices()
            .into_iter()
            .find(|device| {
                Some(device.id.clone()) == settings.microphone_device_id || device.is_default
            })
            .map(|device| device.name);

        Ok(DashboardSnapshot {
            privacy_mode: "local_only".to_string(),
            transcriptions_today: 0,
            words_today: 0,
            speech_minutes_today: 0,
            microphone_name,
            speech_model_name: Some("Whisper base.en".to_string()),
            cleanup_model_name: None,
            active_profile_name: Some(profile.name),
            push_to_talk_shortcut: settings.push_to_talk_shortcut,
            session_type: self.capabilities.session_type,
        })
    }

    pub fn audio_input_devices(&self) -> Vec<AudioInputDevice> {
        self.recording_pipeline
            .list_input_devices()
            .unwrap_or_default()
    }

    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    pub fn data_paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn recording_pipeline(&self) -> &Arc<RecordingPipeline> {
        &self.recording_pipeline
    }
}
