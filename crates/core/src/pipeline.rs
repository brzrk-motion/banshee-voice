use anyhow::{Result, anyhow};
use std::sync::Arc;

use crate::domain::{
    ActiveWindowProvider, AudioCapture, AudioCaptureRequest, AudioInputDevice, CleanupEngine,
    CleanupRequest, DictionaryStore, HudState, HudStateChanged, OutputBackend, OutputRequest,
    PipelineRunResult, PipelineRunStatus, PlatformCapabilities, ProfileStore, RecordingOrigin,
    RecordingSession, RecordingSnapshot, RecordingState, SettingsStore, TranscriptionEngine,
    TranscriptionRequest, VadProcessor,
};

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("no speech detected")]
    NoSpeechDetected,
}

pub struct PipelineServices {
    pub settings: Arc<dyn SettingsStore>,
    pub profiles: Arc<dyn ProfileStore>,
    pub dictionary: Arc<dyn DictionaryStore>,
    pub capabilities: PlatformCapabilities,
    pub audio: Arc<dyn AudioCapture>,
    pub vad: Arc<dyn VadProcessor>,
    pub stt: Arc<dyn TranscriptionEngine>,
    pub cleanup: Arc<dyn CleanupEngine>,
    pub injector: Arc<dyn OutputBackend>,
    pub active_window: Arc<dyn ActiveWindowProvider>,
}

pub struct RecordingPipeline {
    services: PipelineServices,
}

impl RecordingPipeline {
    pub fn new(services: PipelineServices) -> Self {
        Self { services }
    }

    pub fn list_input_devices(&self) -> Result<Vec<AudioInputDevice>> {
        self.services.audio.list_input_devices()
    }

    pub fn start(&self, origin: RecordingOrigin) -> Result<RecordingSession> {
        let settings = self.services.settings.load()?;
        let capture = self.services.audio.start(AudioCaptureRequest {
            device_id: settings.microphone_device_id,
            channels: 1,
            sample_rate_hz: 16_000,
        })?;
        let output_target = if origin == RecordingOrigin::PushToTalk {
            self.services.injector.capture_target().unwrap_or(None)
        } else {
            None
        };
        Ok(RecordingSession {
            capture,
            origin,
            output_target,
        })
    }

    pub fn current_level(&self, session: &RecordingSession) -> Result<f32> {
        self.services.audio.current_level(&session.capture)
    }

    pub fn cancel(&self, session: &RecordingSession) -> Result<()> {
        self.services.audio.cancel(&session.capture)
    }

    pub fn stop(&self, session: &RecordingSession) -> Result<PipelineRunResult> {
        let settings = self.services.settings.load()?;
        let profile = self.services.profiles.default_profile()?;
        let active_window = session
            .output_target
            .as_ref()
            .map(|target| crate::domain::ActiveWindowInfo {
                application_name: target.application_name.clone(),
                window_title: target.window_title.clone(),
            })
            .map(Ok)
            .unwrap_or_else(|| self.services.active_window.active_window())?;
        let captured_audio = self.services.audio.stop(&session.capture)?;
        let vad_result = self
            .services
            .vad
            .trim(captured_audio, settings.vad_sensitivity)?;

        if !vad_result.speech_detected {
            return Err(anyhow!(PipelineError::NoSpeechDetected));
        }

        let vocabulary = self.services.dictionary.list_global()?;

        let transcription = self.services.stt.transcribe(TranscriptionRequest {
            audio: vad_result.trimmed_audio.clone(),
            language: "en".to_string(),
            acceleration_preference: settings.acceleration_preference,
            latency_profile: "fast".to_string(),
            selected_model_name: Some("base.en".to_string()),
            initial_prompt: Some(build_initial_prompt(&vocabulary)),
        })?;

        let profile_id = profile.id.clone();
        let cleanup = self.services.cleanup.cleanup(CleanupRequest {
            raw_text: transcription.raw_text.clone(),
            profile,
            vocabulary,
            llm_enabled: settings.cleanup_llm_enabled,
            active_application: active_window.application_name.clone(),
        })?;

        let output = if session.origin == RecordingOrigin::PushToTalk {
            self.services.injector.insert_text(OutputRequest {
                text: cleanup.final_text.clone(),
                target: session.output_target.clone(),
                preserve_clipboard: settings.preserve_clipboard,
                paste_delay_ms: settings.paste_delay_ms,
                auto_paste_enabled: true,
                session_type: self.services.capabilities.session_type,
            })?
        } else {
            crate::domain::OutputResponse {
                method: crate::domain::OutputMethod::None,
                result: crate::domain::OutputResultKind::Success,
                message: "Transcript is ready in the scratch space.".to_string(),
            }
        };

        Ok(PipelineRunResult {
            session_id: session.capture.id.clone(),
            origin: session.origin,
            raw_text: transcription.raw_text,
            deterministic_text: cleanup.deterministic_text,
            final_text: cleanup.final_text,
            stt_backend: transcription.backend,
            cleanup_backend: cleanup.backend,
            stt_latency_ms: transcription.latency_ms,
            cleanup_latency_ms: cleanup.latency_ms,
            cleanup_fallback_reason: cleanup.fallback_reason,
            peak_level: vad_result.peak_level,
            status: if output.result == crate::domain::OutputResultKind::Success {
                PipelineRunStatus::Completed
            } else {
                PipelineRunStatus::FallbackUsed
            },
            output,
            active_window,
            duration_ms: vad_result.trimmed_audio.duration_ms,
            profile_id,
            acceleration_preference: settings.acceleration_preference,
            session_type: self.services.capabilities.session_type,
        })
    }

    pub fn idle_snapshot() -> RecordingSnapshot {
        RecordingSnapshot {
            state: RecordingState::Idle,
            hud: HudStateChanged {
                session_id: None,
                state: HudState::Hidden,
                message: None,
            },
            last_transcript: None,
            last_error: None,
        }
    }
}

fn build_initial_prompt(vocabulary: &[crate::domain::DictionaryEntry]) -> String {
    [
        "Banshee",
        "HUD",
        "Codex",
        "Claude Code",
        "GitHub",
        "Tauri",
        "Rust",
        "PowerShell",
        "TypeScript",
    ]
    .into_iter()
    .map(str::to_string)
    .chain(vocabulary.iter().map(|entry| entry.output_form.clone()))
    .collect::<Vec<_>>()
    .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ActiveWindowInfo, CaptureSession, CleanupOutput, OutputMethod, OutputResponse,
        OutputResultKind, ProfileSummary, SessionType, Settings,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestSettingsStore;

    impl SettingsStore for TestSettingsStore {
        fn load(&self) -> Result<Settings> {
            Ok(Settings::default())
        }

        fn update(&self, _update: crate::domain::SettingsUpdate) -> Result<Settings> {
            Ok(Settings::default())
        }
    }

    struct TestProfileStore;

    impl ProfileStore for TestProfileStore {
        fn list(&self) -> Result<Vec<ProfileSummary>> {
            Ok(vec![self.default_profile()?])
        }

        fn default_profile(&self) -> Result<ProfileSummary> {
            Ok(ProfileSummary {
                id: "profile-agent".to_string(),
                name: "Agent".to_string(),
                slug: "agent".to_string(),
                description: String::new(),
                built_in: true,
            })
        }
    }

    struct TestDictionaryStore;

    impl crate::domain::DictionaryStore for TestDictionaryStore {
        fn list_global(&self) -> Result<Vec<crate::domain::DictionaryEntry>> {
            Ok(Vec::new())
        }

        fn replace_global(
            &self,
            entries: Vec<crate::domain::DictionaryEntry>,
        ) -> Result<Vec<crate::domain::DictionaryEntry>> {
            Ok(entries)
        }
    }

    struct TestAudioCapture;

    impl AudioCapture for TestAudioCapture {
        fn list_input_devices(&self) -> Result<Vec<AudioInputDevice>> {
            Ok(vec![])
        }

        fn start(&self, request: AudioCaptureRequest) -> Result<CaptureSession> {
            Ok(CaptureSession {
                id: "session-1".to_string(),
                device_id: request.device_id,
            })
        }

        fn stop(&self, _session: &CaptureSession) -> Result<crate::domain::CapturedAudio> {
            Ok(crate::domain::CapturedAudio {
                samples: vec![0.2; 4_000],
                sample_rate_hz: 16_000,
                channels: 1,
                duration_ms: 250,
            })
        }

        fn cancel(&self, _session: &CaptureSession) -> Result<()> {
            Ok(())
        }

        fn current_level(&self, _session: &CaptureSession) -> Result<f32> {
            Ok(0.25)
        }
    }

    struct TestVad {
        speech_detected: bool,
    }

    impl VadProcessor for TestVad {
        fn trim(
            &self,
            audio: crate::domain::CapturedAudio,
            _sensitivity: f64,
        ) -> Result<crate::domain::VadResult> {
            Ok(crate::domain::VadResult {
                trimmed_audio: audio,
                speech_detected: self.speech_detected,
                peak_level: 0.7,
                speech_start_ms: Some(10),
                speech_end_ms: Some(220),
            })
        }
    }

    struct TestStt;

    impl TranscriptionEngine for TestStt {
        fn transcribe(
            &self,
            _request: TranscriptionRequest,
        ) -> Result<crate::domain::TranscriptionOutput> {
            Ok(crate::domain::TranscriptionOutput {
                raw_text: "um ship it period".to_string(),
                backend: "whisper_cpp_preview:test".to_string(),
                latency_ms: 100,
            })
        }
    }

    struct TestCleanup;

    impl CleanupEngine for TestCleanup {
        fn cleanup(&self, _request: CleanupRequest) -> Result<CleanupOutput> {
            Ok(CleanupOutput {
                deterministic_text: "ship it.".to_string(),
                final_text: "ship it.".to_string(),
                backend: "deterministic".to_string(),
                latency_ms: 1,
                fallback_reason: None,
            })
        }
    }

    struct TestOutputBackend {
        result: OutputResultKind,
        calls: Arc<AtomicUsize>,
    }

    impl OutputBackend for TestOutputBackend {
        fn capture_target(&self) -> Result<Option<crate::domain::OutputTarget>> {
            Ok(Some(crate::domain::OutputTarget {
                identity: "target-1".to_string(),
                application_name: "Editor".to_string(),
                window_title: "main.rs".to_string(),
                bounds: None,
                editable_verified: true,
            }))
        }

        fn insert_text(&self, _request: OutputRequest) -> Result<OutputResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(OutputResponse {
                method: OutputMethod::ClipboardPaste,
                result: self.result,
                message: "done".to_string(),
            })
        }
    }

    struct TestActiveWindowProvider;

    impl ActiveWindowProvider for TestActiveWindowProvider {
        fn active_window(&self) -> Result<ActiveWindowInfo> {
            Ok(ActiveWindowInfo {
                application_name: "Editor".to_string(),
                window_title: "main.rs".to_string(),
            })
        }
    }

    fn pipeline_with_counter(
        speech_detected: bool,
        result: OutputResultKind,
        calls: Arc<AtomicUsize>,
    ) -> RecordingPipeline {
        RecordingPipeline::new(PipelineServices {
            settings: Arc::new(TestSettingsStore),
            profiles: Arc::new(TestProfileStore),
            dictionary: Arc::new(TestDictionaryStore),
            capabilities: PlatformCapabilities {
                session_type: SessionType::X11,
                direct_injection: crate::domain::PlatformSupportTier::Native,
                active_window_detection: crate::domain::PlatformSupportTier::Native,
                global_shortcuts: crate::domain::PlatformSupportTier::Native,
                tray_supported: true,
                hud_supported: true,
            },
            audio: Arc::new(TestAudioCapture),
            vad: Arc::new(TestVad { speech_detected }),
            stt: Arc::new(TestStt),
            cleanup: Arc::new(TestCleanup),
            injector: Arc::new(TestOutputBackend { result, calls }),
            active_window: Arc::new(TestActiveWindowProvider),
        })
    }

    fn pipeline(speech_detected: bool, result: OutputResultKind) -> RecordingPipeline {
        pipeline_with_counter(speech_detected, result, Arc::new(AtomicUsize::new(0)))
    }

    #[test]
    fn initial_prompt_contains_built_in_and_custom_terms() {
        let prompt = build_initial_prompt(&[crate::domain::DictionaryEntry {
            spoken_form: "banci".into(),
            output_form: "Banshee Voice".into(),
        }]);
        assert!(prompt.contains("HUD"));
        assert!(prompt.contains("Banshee Voice"));
    }

    #[test]
    fn returns_fallback_status_when_output_falls_back() {
        let pipeline = pipeline(true, OutputResultKind::Fallback);
        let session = pipeline
            .start(RecordingOrigin::PushToTalk)
            .expect("session should start");

        let result = pipeline.stop(&session).expect("pipeline should succeed");

        assert_eq!(result.status, PipelineRunStatus::FallbackUsed);
        assert_eq!(result.session_id, "session-1");
        assert_eq!(result.stt_backend, "whisper_cpp_preview:test");
    }

    #[test]
    fn rejects_no_speech_runs() {
        let pipeline = pipeline(false, OutputResultKind::Success);
        let session = pipeline
            .start(RecordingOrigin::PushToTalk)
            .expect("session should start");
        let error = pipeline
            .stop(&session)
            .expect_err("no-speech runs should fail");

        assert!(error.to_string().contains("no speech detected"));
    }

    #[test]
    fn scratch_space_runs_skip_output_injection() {
        let calls = Arc::new(AtomicUsize::new(0));
        let pipeline = pipeline_with_counter(true, OutputResultKind::Success, calls.clone());
        let session = pipeline
            .start(RecordingOrigin::Scratch)
            .expect("session should start");

        let result = pipeline.stop(&session).expect("pipeline should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(result.output.method, OutputMethod::None);
        assert_eq!(result.final_text, "ship it.");
    }
}
