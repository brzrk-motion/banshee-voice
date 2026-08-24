//! Local whisper.cpp speech-to-text engine.

use anyhow::Result;
use banshee_core::domain::{
    AppErrorCode, TranscriptionEngine, TranscriptionOutput, TranscriptionRequest,
};
use std::path::Path;
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

#[derive(Clone, Default)]
pub struct WhisperCppEngine {
    context: Arc<RwLock<Option<Arc<WhisperContext>>>>,
    model_name: Arc<RwLock<String>>,
}

impl WhisperCppEngine {
    pub fn load_model(&self, path: &Path) -> Result<()> {
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|error| SttError::ModelLoadFailed(error.to_string()))?;
        *self.context.write().expect("whisper context lock poisoned") = Some(Arc::new(context));
        *self
            .model_name
            .write()
            .expect("whisper model name lock poisoned") = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .trim_start_matches("ggml-")
            .to_string();
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.context
            .read()
            .expect("whisper context lock poisoned")
            .is_some()
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

        let context = self
            .context
            .read()
            .expect("whisper context lock poisoned")
            .clone()
            .ok_or(SttError::ModelMissing)?;
        let started = Instant::now();
        let mut state = context
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
            backend: format!(
                "whisper.cpp:{}:cpu",
                self.model_name
                    .read()
                    .expect("whisper model name lock poisoned")
            ),
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_core::domain::{AccelerationPreference, CapturedAudio};

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
}
