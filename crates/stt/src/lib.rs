//! Speech-to-text engine adapters for Banshee.

use anyhow::{Result, bail};
use banshee_core::domain::{
    AppErrorCode, TranscriptionEngine, TranscriptionOutput, TranscriptionRequest,
};
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SttError {
    #[error("model missing")]
    ModelMissing,
    #[error("no speech detected")]
    NoSpeechDetected,
}

impl SttError {
    pub const fn code(&self) -> AppErrorCode {
        match self {
            Self::ModelMissing => AppErrorCode::ModelMissing,
            Self::NoSpeechDetected => AppErrorCode::NoSpeechDetected,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WhisperCppPreviewEngine;

impl TranscriptionEngine for WhisperCppPreviewEngine {
    fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionOutput> {
        if request.audio.samples.is_empty() || request.audio.duration_ms < 150 {
            return Err(SttError::NoSpeechDetected.into());
        }

        let model_name = request
            .selected_model_name
            .unwrap_or_else(|| "Whisper Preview".to_string());
        if model_name.trim().is_empty() {
            bail!(SttError::ModelMissing);
        }

        let raw_text = if let Ok(value) = env::var("BANSHEE_PREVIEW_TRANSCRIPT") {
            value
        } else if request.audio.duration_ms > 4_000 {
            "um please review the current changes and explain any risky edge cases period"
                .to_string()
        } else {
            "um update the current file and keep the patch minimal period".to_string()
        };

        Ok(TranscriptionOutput {
            raw_text,
            backend: format!("whisper_cpp_preview:{model_name}"),
            latency_ms: 120,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_core::domain::{AccelerationPreference, CapturedAudio};

    fn request(duration_ms: u64, selected_model_name: Option<&str>) -> TranscriptionRequest {
        TranscriptionRequest {
            audio: CapturedAudio {
                samples: vec![0.2; 4_000],
                sample_rate_hz: 16_000,
                channels: 1,
                duration_ms,
            },
            language: "en".to_string(),
            acceleration_preference: AccelerationPreference::Auto,
            latency_profile: "balanced".to_string(),
            selected_model_name: selected_model_name.map(str::to_string),
        }
    }

    #[test]
    fn rejects_empty_model_names() {
        let engine = WhisperCppPreviewEngine;
        let error = engine
            .transcribe(request(500, Some("   ")))
            .expect_err("empty model names should fail");

        assert!(error.to_string().contains("model missing"));
    }

    #[test]
    fn emits_preview_backend_name() {
        let engine = WhisperCppPreviewEngine;
        let output = engine
            .transcribe(request(500, Some("Tiny English")))
            .expect("transcription should succeed");

        assert_eq!(output.backend, "whisper_cpp_preview:Tiny English");
    }
}
