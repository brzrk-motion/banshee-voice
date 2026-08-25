pub mod ipc;

use banshee_core::audio::CpalAudioCapture;
use banshee_core::injector::ClipboardInjector;
use banshee_core::models::{ModelCapability, ModelInstaller, ModelState, ModelsStatus};
use banshee_core::platform::EnvActiveWindowProvider;
use banshee_core::platform::PlatformCapabilityProbe;
use banshee_core::plugins::{PROMPT_ENHANCER_ID, PluginRegistry, PromptEnhancer};
use banshee_core::storage::SqliteDictionaryRepository;
use banshee_core::storage::SqliteTranscriptionRepository;
use banshee_core::storage::initialize_storage;
use banshee_core::stt::WhisperCppEngine;
use banshee_core::transformer::TranscriptCleanup;
use banshee_core::vad::SimpleVadProcessor;
use banshee_core::{
    AppServices,
    domain::{
        PluginRuntimeState, PluginRuntimeStatus, PluginStateStore, PluginSummary, RecordingSession,
        RecordingSnapshot,
    },
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
    prompt_enhancer_installer: ModelInstaller,
    whisper_engine: WhisperCppEngine,
    plugins: Arc<PluginRegistry>,
    prompt_enhancer: PromptEnhancer,
}

impl ManagedAppState {
    pub fn initialize() -> anyhow::Result<Self> {
        let storage = initialize_storage()?;
        let capabilities = PlatformCapabilityProbe::default().detect();
        let whisper_engine = WhisperCppEngine::default();
        let speech_model_installer = ModelInstaller::new(&storage.paths.data_dir);
        let prompt_enhancer_installer = ModelInstaller::cleanup(&storage.paths.data_dir);
        let cleanup_engine = TranscriptCleanup::default();
        let prompt_enhancer = PromptEnhancer::default();
        let plugin_state: Arc<dyn PluginStateStore> = Arc::new(storage.plugins.clone());
        let plugins = Arc::new(PluginRegistry::new(
            plugin_state,
            vec![Arc::new(prompt_enhancer.clone())],
        ));
        let recording_pipeline = Arc::new(RecordingPipeline::new(PipelineServices {
            settings: Arc::new(storage.settings.clone()),
            profiles: Arc::new(storage.profiles.clone()),
            dictionary: Arc::new(storage.dictionary.clone()),
            capabilities: capabilities.clone(),
            audio: Arc::new(CpalAudioCapture::default()),
            vad: Arc::new(SimpleVadProcessor),
            stt: Arc::new(whisper_engine.clone()),
            cleanup: Arc::new(cleanup_engine.clone()),
            plugins: plugins.clone(),
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
            prompt_enhancer_installer,
            whisper_engine,
            plugins,
            prompt_enhancer,
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
            cleanup: self.prompt_enhancer_installer.status(),
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

    pub fn plugins(&self) -> anyhow::Result<Vec<PluginSummary>> {
        self.plugins.list()
    }

    pub fn set_plugin_enabled(
        &self,
        plugin_id: &str,
        enabled: bool,
        app: tauri::AppHandle,
    ) -> anyhow::Result<Vec<PluginSummary>> {
        self.plugins.set_enabled(plugin_id, enabled)?;
        if plugin_id == PROMPT_ENHANCER_ID {
            if enabled {
                self.ensure_prompt_enhancer(app.clone());
            } else {
                self.prompt_enhancer.unload();
            }
        }
        let _ = app.emit("plugins_changed", ());
        self.plugins()
    }

    pub fn ensure_prompt_enhancer(&self, app: tauri::AppHandle) {
        let engine = self.prompt_enhancer.clone();
        let status_engine = self.prompt_enhancer.clone();
        engine.enable();
        self.prompt_enhancer_installer.ensure_installed(
            move |status| {
                if !status_engine.is_enabled() {
                    return;
                }
                status_engine.set_runtime_status(model_status_to_plugin(&status));
                let _ = app.emit("plugins_changed", ());
            },
            move |path| engine.start_worker(&prompt_worker_path()?, path),
        );
    }

    pub fn retry_plugin(&self, plugin_id: &str, app: tauri::AppHandle) -> anyhow::Result<()> {
        if plugin_id != PROMPT_ENHANCER_ID {
            anyhow::bail!("unknown plugin: {plugin_id}");
        }
        self.ensure_prompt_enhancer(app);
        Ok(())
    }

    pub fn retry_model(&self, capability: ModelCapability, app: tauri::AppHandle) {
        match capability {
            ModelCapability::Speech => self.ensure_speech_model(app),
            ModelCapability::Cleanup => self.ensure_prompt_enhancer(app),
        }
    }
}

fn prompt_worker_path() -> anyhow::Result<std::path::PathBuf> {
    let executable = std::env::current_exe()?;
    let directory = executable
        .parent()
        .ok_or_else(|| anyhow::anyhow!("desktop executable has no parent directory"))?;
    let name = if cfg!(windows) {
        "banshee-prompt-worker.exe"
    } else {
        "banshee-prompt-worker"
    };
    Ok(directory.join(name))
}

fn model_status_to_plugin(status: &banshee_core::models::ModelStatus) -> PluginRuntimeStatus {
    PluginRuntimeStatus {
        state: match status.state {
            ModelState::Missing => PluginRuntimeState::Missing,
            ModelState::Downloading => PluginRuntimeState::Downloading,
            ModelState::Loading => PluginRuntimeState::Loading,
            ModelState::Ready => PluginRuntimeState::Ready,
            ModelState::Error => PluginRuntimeState::Error,
        },
        downloaded_bytes: status.downloaded_bytes,
        total_bytes: status.total_bytes,
        message: status.message.clone(),
    }
}
