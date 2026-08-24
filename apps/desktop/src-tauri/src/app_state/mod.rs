pub mod ipc;

use banshee_audio::CpalAudioCapture;
use banshee_core::{
    AppServices,
    domain::{CaptureSession, RecordingSnapshot},
    pipeline::{PipelineServices, RecordingPipeline},
};
use banshee_injector::ClipboardInjector;
use banshee_models::{ModelInstaller, ModelState, ModelStatus};
use banshee_platform::EnvActiveWindowProvider;
use banshee_platform::PlatformCapabilityProbe;
use banshee_storage::SqliteTranscriptionRepository;
use banshee_storage::initialize_storage;
use banshee_stt::WhisperCppEngine;
use banshee_transformer::DeterministicCleanup;
use banshee_vad::SimpleVadProcessor;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

pub struct RecordingRuntimeState {
    pub active_session: Option<CaptureSession>,
    pub active_trigger: Option<RecordingTrigger>,
    pub snapshot: RecordingSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingTrigger {
    Manual,
    HoldToTalk,
    Toggle,
}

impl RecordingRuntimeState {
    pub fn begin_session(
        &mut self,
        session: CaptureSession,
        trigger: RecordingTrigger,
    ) -> anyhow::Result<()> {
        if self.active_session.is_some() {
            anyhow::bail!("a recording session is already active");
        }

        self.active_session = Some(session);
        self.active_trigger = Some(trigger);
        Ok(())
    }

    pub fn take_session(&mut self) -> Option<(CaptureSession, RecordingTrigger)> {
        let session = self.active_session.take()?;
        let trigger = self.active_trigger.take()?;
        Some((session, trigger))
    }
}

#[derive(Clone)]
pub struct ManagedAppState {
    services: Arc<AppServices>,
    history: SqliteTranscriptionRepository,
    recording: Arc<Mutex<RecordingRuntimeState>>,
    model_installer: ModelInstaller,
    whisper_engine: WhisperCppEngine,
}

impl ManagedAppState {
    pub fn initialize() -> anyhow::Result<Self> {
        let storage = initialize_storage()?;
        let capabilities = PlatformCapabilityProbe::default().detect();
        let whisper_engine = WhisperCppEngine::default();
        let model_installer = ModelInstaller::new(&storage.paths.data_dir);
        let recording_pipeline = Arc::new(RecordingPipeline::new(PipelineServices {
            settings: Arc::new(storage.settings.clone()),
            profiles: Arc::new(storage.profiles.clone()),
            capabilities: capabilities.clone(),
            audio: Arc::new(CpalAudioCapture::default()),
            vad: Arc::new(SimpleVadProcessor),
            stt: Arc::new(whisper_engine.clone()),
            cleanup: Arc::new(DeterministicCleanup),
            injector: Arc::new(ClipboardInjector),
            active_window: Arc::new(EnvActiveWindowProvider),
        }));

        Ok(Self {
            services: Arc::new(AppServices::new(
                Arc::new(storage.settings.clone()),
                Arc::new(storage.profiles.clone()),
                capabilities,
                storage.paths.clone(),
                recording_pipeline,
            )),
            history: storage.transcriptions.clone(),
            recording: Arc::new(Mutex::new(RecordingRuntimeState {
                active_session: None,
                active_trigger: None,
                snapshot: RecordingPipeline::idle_snapshot(),
            })),
            model_installer,
            whisper_engine,
        })
    }

    pub fn services(&self) -> &AppServices {
        self.services.as_ref()
    }

    pub fn recording(&self) -> &Arc<Mutex<RecordingRuntimeState>> {
        &self.recording
    }

    pub fn history(&self) -> &SqliteTranscriptionRepository {
        &self.history
    }

    pub fn model_status(&self) -> ModelStatus {
        self.model_installer.status()
    }

    pub fn model_ready(&self) -> bool {
        self.model_status().state == ModelState::Ready && self.whisper_engine.is_ready()
    }

    pub fn ensure_model(&self, app: tauri::AppHandle) {
        let engine = self.whisper_engine.clone();
        self.model_installer.ensure_installed(
            move |status| {
                let _ = app.emit("model_status_changed", status);
            },
            move |path| engine.load_model(path),
        );
    }
}
