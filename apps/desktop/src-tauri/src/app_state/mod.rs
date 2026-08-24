pub mod ipc;

use banshee_core::audio::CpalAudioCapture;
use banshee_core::injector::ClipboardInjector;
use banshee_core::models::{ModelCapability, ModelInstaller, ModelState, ModelsStatus};
use banshee_core::platform::EnvActiveWindowProvider;
use banshee_core::platform::PlatformCapabilityProbe;
use banshee_core::storage::SqliteDictionaryRepository;
use banshee_core::storage::SqliteTranscriptionRepository;
use banshee_core::storage::initialize_storage;
use banshee_core::stt::WhisperCppEngine;
use banshee_core::transformer::TranscriptCleanup;
use banshee_core::vad::SimpleVadProcessor;
use banshee_core::{
    AppServices,
    domain::{RecordingSession, RecordingSnapshot},
    pipeline::{PipelineServices, RecordingPipeline},
};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

pub struct RecordingRuntimeState {
    pub active_session: Option<RecordingSession>,
    pub snapshot: RecordingSnapshot,
}

impl RecordingRuntimeState {
    pub fn begin_session(&mut self, session: RecordingSession) -> anyhow::Result<()> {
        if self.active_session.is_some() {
            anyhow::bail!("a recording session is already active");
        }

        self.active_session = Some(session);
        Ok(())
    }

    pub fn take_session(&mut self) -> Option<RecordingSession> {
        self.active_session.take()
    }
}

#[derive(Clone)]
pub struct ManagedAppState {
    services: Arc<AppServices>,
    history: SqliteTranscriptionRepository,
    dictionary: SqliteDictionaryRepository,
    recording: Arc<Mutex<RecordingRuntimeState>>,
    speech_model_installer: ModelInstaller,
    cleanup_model_installer: ModelInstaller,
    whisper_engine: WhisperCppEngine,
    cleanup_engine: TranscriptCleanup,
}

impl ManagedAppState {
    pub fn initialize() -> anyhow::Result<Self> {
        let storage = initialize_storage()?;
        let capabilities = PlatformCapabilityProbe::default().detect();
        let whisper_engine = WhisperCppEngine::default();
        let speech_model_installer = ModelInstaller::new(&storage.paths.data_dir);
        let cleanup_model_installer = ModelInstaller::cleanup(&storage.paths.data_dir);
        let cleanup_engine = TranscriptCleanup::default();
        let recording_pipeline = Arc::new(RecordingPipeline::new(PipelineServices {
            settings: Arc::new(storage.settings.clone()),
            profiles: Arc::new(storage.profiles.clone()),
            dictionary: Arc::new(storage.dictionary.clone()),
            capabilities: capabilities.clone(),
            audio: Arc::new(CpalAudioCapture::default()),
            vad: Arc::new(SimpleVadProcessor),
            stt: Arc::new(whisper_engine.clone()),
            cleanup: Arc::new(cleanup_engine.clone()),
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
            dictionary: storage.dictionary.clone(),
            recording: Arc::new(Mutex::new(RecordingRuntimeState {
                active_session: None,
                snapshot: RecordingPipeline::idle_snapshot(),
            })),
            speech_model_installer,
            cleanup_model_installer,
            whisper_engine,
            cleanup_engine,
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

    pub fn dictionary(&self) -> &SqliteDictionaryRepository {
        &self.dictionary
    }

    pub fn models_status(&self) -> ModelsStatus {
        ModelsStatus {
            speech: self.speech_model_installer.status(),
            cleanup: self.cleanup_model_installer.status(),
        }
    }

    pub fn model_ready(&self) -> bool {
        self.speech_model_installer.status().state == ModelState::Ready
            && self.whisper_engine.is_ready()
    }

    pub fn ensure_speech_model(&self, app: tauri::AppHandle) {
        let engine = self.whisper_engine.clone();
        self.speech_model_installer.ensure_installed(
            move |status| {
                let _ = app.emit("model_status_changed", status);
            },
            move |path| engine.load_model(path),
        );
    }

    pub fn ensure_cleanup_model(&self, app: tauri::AppHandle) {
        self.cleanup_engine.enable();
        let engine = self.cleanup_engine.clone();
        self.cleanup_model_installer.ensure_installed(
            move |status| {
                let _ = app.emit("model_status_changed", status);
            },
            move |path| engine.load_model(path),
        );
    }

    pub fn retry_model(&self, capability: ModelCapability, app: tauri::AppHandle) {
        match capability {
            ModelCapability::Speech => self.ensure_speech_model(app),
            ModelCapability::Cleanup => self.ensure_cleanup_model(app),
        }
    }

    pub fn disable_cleanup_model(&self) {
        self.cleanup_engine.unload();
    }
}
