//! Local whisper.cpp speech-to-text engine.

use anyhow::Result;
use banshee_contracts::domain::{
    AccelerationBackend, AccelerationPreference, AccelerationStatus, AppErrorCode,
    TranscriptionEngine, TranscriptionOutput, TranscriptionRequest,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use thiserror::Error;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Debug, Error)]
pub enum SttError {
    #[error("speech model is not ready")]
    ModelMissing,
    #[error("no speech detected")]
    NoSpeechDetected,
    #[error("speech model failed to load: {0}")]
    ModelLoadFailed(String),
    #[error("transcription failed: {0}")]
    InferenceFailed(String),
}

impl SttError {
    pub const fn code(&self) -> AppErrorCode {
        match self {
            Self::ModelMissing | Self::ModelLoadFailed(_) => AppErrorCode::ModelMissing,
            Self::NoSpeechDetected => AppErrorCode::NoSpeechDetected,
            Self::InferenceFailed(_) => AppErrorCode::InferenceFailed,
        }
    }
}

struct LoadedContext {
    context: Arc<WhisperContext>,
    model_name: String,
    backend: AccelerationBackend,
}

#[derive(Clone)]
pub struct WhisperCppEngine {
    loaded: Arc<RwLock<Option<LoadedContext>>>,
    model_path: Arc<RwLock<Option<PathBuf>>>,
    preference: Arc<RwLock<AccelerationPreference>>,
}

impl Default for WhisperCppEngine {
    fn default() -> Self {
        Self::new(AccelerationPreference::Auto)
    }
}

impl WhisperCppEngine {
    pub fn new(preference: AccelerationPreference) -> Self {
        Self {
            loaded: Arc::new(RwLock::new(None)),
            model_path: Arc::new(RwLock::new(None)),
            preference: Arc::new(RwLock::new(preference)),
        }
    }

    pub fn load_model(&self, path: &Path) -> Result<()> {
        *self
            .model_path
            .write()
            .expect("whisper model path lock poisoned") = Some(path.to_path_buf());
        let preference = *self
            .preference
            .read()
            .expect("whisper preference lock poisoned");
        let loaded = load_context(path, preference)?;
        *self.loaded.write().expect("whisper context lock poisoned") = Some(loaded);
        Ok(())
    }

    pub fn set_acceleration_preference(&self, preference: AccelerationPreference) -> Result<()> {
        if *self
            .preference
            .read()
            .expect("whisper preference lock poisoned")
            == preference
        {
            return Ok(());
        }

        let model_path = self
            .model_path
            .read()
            .expect("whisper model path lock poisoned")
            .clone();
        if let Some(path) = model_path {
            let loaded = load_context(&path, preference)?;
            *self.loaded.write().expect("whisper context lock poisoned") = Some(loaded);
        }
        *self
            .preference
            .write()
            .expect("whisper preference lock poisoned") = preference;
        Ok(())
    }

    pub fn acceleration_status(&self) -> AccelerationStatus {
        vulkan_status()
    }

    pub fn actual_backend(&self) -> Option<AccelerationBackend> {
        self.loaded
            .read()
            .expect("whisper context lock poisoned")
            .as_ref()
            .map(|loaded| loaded.backend)
    }

    pub fn is_ready(&self) -> bool {
        self.loaded
            .read()
            .expect("whisper context lock poisoned")
            .is_some()
    }
}

fn load_context(path: &Path, preference: AccelerationPreference) -> Result<LoadedContext> {
    let model_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .trim_start_matches("ggml-")
        .to_string();

    let load = |use_gpu: bool| {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(use_gpu);
        WhisperContext::new_with_params(path, params)
            .map(Arc::new)
            .map_err(|error| SttError::ModelLoadFailed(error.to_string()))
    };

    match preference {
        AccelerationPreference::Cpu => Ok(LoadedContext {
            context: load(false)?,
            model_name,
            backend: AccelerationBackend::Cpu,
        }),
        AccelerationPreference::Gpu => {
            let status = vulkan_status();
            if !status.gpu_available {
                return Err(SttError::ModelLoadFailed(
                    status
                        .unavailable_reason
                        .unwrap_or_else(|| "Vulkan GPU acceleration is unavailable".into()),
                )
                .into());
            }
            Ok(LoadedContext {
                context: load(true)?,
                model_name,
                backend: AccelerationBackend::Gpu,
            })
        }
        AccelerationPreference::Auto => {
            if vulkan_status().gpu_available {
                if let Ok(context) = load(true) {
                    return Ok(LoadedContext {
                        context,
                        model_name,
                        backend: AccelerationBackend::Gpu,
                    });
                }
            }
            Ok(LoadedContext {
                context: load(false)?,
                model_name,
                backend: AccelerationBackend::Cpu,
            })
        }
    }
}

#[cfg(feature = "gpu-vulkan")]
fn vulkan_status() -> AccelerationStatus {
    match whisper_rs::vulkan::list_devices().into_iter().next() {
        Some(device) => AccelerationStatus {
            gpu_available: true,
            backend: Some("vulkan".into()),
            device_name: Some(device.name),
            unavailable_reason: None,
        },
        None => AccelerationStatus {
            gpu_available: false,
            backend: Some("vulkan".into()),
            device_name: None,
            unavailable_reason: Some("No Vulkan-compatible GPU was detected".into()),
        },
    }
}

#[cfg(not(feature = "gpu-vulkan"))]
fn vulkan_status() -> AccelerationStatus {
    AccelerationStatus {
        gpu_available: false,
        backend: None,
        device_name: None,
        unavailable_reason: Some("This build does not include Vulkan GPU acceleration".into()),
    }
}

impl TranscriptionEngine for WhisperCppEngine {
    fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionOutput> {
        if request.audio.samples.is_empty() || request.audio.duration_ms < 150 {
            return Err(SttError::NoSpeechDetected.into());
        }
        if request.audio.sample_rate_hz != 16_000 || request.audio.channels != 1 {
            return Err(SttError::InferenceFailed("expected 16 kHz mono PCM".to_string()).into());
        }

        self.set_acceleration_preference(request.acceleration_preference)?;
        let loaded = self
            .loaded
            .read()
            .expect("whisper context lock poisoned")
            .as_ref()
            .map(|loaded| {
                (
                    Arc::clone(&loaded.context),
                    loaded.model_name.clone(),
                    loaded.backend,
                )
            })
            .ok_or(SttError::ModelMissing)?;
        let started = Instant::now();
        let mut state = loaded
            .0
            .create_state()
            .map_err(|error| SttError::InferenceFailed(error.to_string()))?;
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        let threads = std::thread::available_parallelism()
            .map(|count| count.get().min(8) as i32)
            .unwrap_or(1);
        params.set_n_threads(threads);
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_no_context(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        if let Some(prompt) = request
            .initial_prompt
            .as_deref()
            .filter(|prompt| !prompt.is_empty())
        {
            params.set_initial_prompt(prompt);
        }
        state
            .full(params, &request.audio.samples)
            .map_err(|error| SttError::InferenceFailed(error.to_string()))?;

        let raw_text = state
            .as_iter()
            .map(|segment| {
                segment
                    .to_str_lossy()
                    .map(|text| text.into_owned())
                    .map_err(|error| SttError::InferenceFailed(error.to_string()))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("")
            .trim()
            .to_string();
        if raw_text.is_empty() {
            return Err(SttError::NoSpeechDetected.into());
        }

        Ok(TranscriptionOutput {
            raw_text,
            backend: format!("whisper.cpp:{}:{}", loaded.1, loaded.2.as_str()),
            acceleration_backend: loaded.2,
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_contracts::domain::{AccelerationPreference, CapturedAudio};

    #[test]
    fn reports_missing_model_before_inference() {
        let engine = WhisperCppEngine::default();
        let error = engine
            .transcribe(TranscriptionRequest {
                audio: CapturedAudio {
                    samples: vec![0.2; 4_000],
                    sample_rate_hz: 16_000,
                    channels: 1,
                    duration_ms: 250,
                },
                language: "en".to_string(),
                acceleration_preference: AccelerationPreference::Auto,
                latency_profile: "fast".to_string(),
                selected_model_name: Some("base.en".to_string()),
                initial_prompt: None,
            })
            .expect_err("an unloaded engine should fail");
        assert!(error.to_string().contains("not ready"));
    }

    #[cfg(not(feature = "gpu-vulkan"))]
    #[test]
    fn reports_when_vulkan_was_not_compiled_in() {
        let status = WhisperCppEngine::default().acceleration_status();
        assert!(!status.gpu_available);
        assert_eq!(status.backend, None);
        assert!(
            status
                .unavailable_reason
                .unwrap()
                .contains("does not include")
        );
    }
}
