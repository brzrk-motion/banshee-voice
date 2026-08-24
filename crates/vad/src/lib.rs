//! Voice activity detection abstractions for Banshee.

use anyhow::Result;
use banshee_core::domain::{CapturedAudio, VadProcessor, VadResult};

#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleVadProcessor;

impl VadProcessor for SimpleVadProcessor {
    fn trim(&self, audio: CapturedAudio, sensitivity: f64) -> Result<VadResult> {
        let threshold = (0.02_f32 * (1.1 - sensitivity as f32)).max(0.004);
        let mut first = None;
        let mut last = None;
        let mut peak = 0.0_f32;

        for (index, sample) in audio.samples.iter().enumerate() {
            let level = sample.abs();
            peak = peak.max(level);
            if level >= threshold {
                first.get_or_insert(index);
                last = Some(index);
            }
        }

        let padding_samples = (audio.sample_rate_hz as usize * 250 / 1_000)
            .saturating_mul(usize::from(audio.channels.max(1)));
        let trimmed_samples = match (first, last) {
            (Some(start), Some(end)) if end >= start => {
                let padded_start = start.saturating_sub(padding_samples);
                let padded_end = end
                    .saturating_add(padding_samples)
                    .min(audio.samples.len().saturating_sub(1));
                audio.samples[padded_start..=padded_end].to_vec()
            }
            _ => Vec::new(),
        };

        let duration_ms = if trimmed_samples.is_empty() {
            0
        } else {
            ((trimmed_samples.len() as f64 / audio.channels as f64 / audio.sample_rate_hz as f64)
                * 1000.0)
                .round() as u64
        };

        Ok(VadResult {
            trimmed_audio: CapturedAudio {
                samples: trimmed_samples,
                sample_rate_hz: audio.sample_rate_hz,
                channels: audio.channels,
                duration_ms,
            },
            speech_detected: first.is_some(),
            peak_level: peak,
            speech_start_ms: first.map(|index| {
                ((index as f64 / audio.channels as f64 / audio.sample_rate_hz as f64) * 1000.0)
                    .round() as u64
            }),
            speech_end_ms: last.map(|index| {
                ((index as f64 / audio.channels as f64 / audio.sample_rate_hz as f64) * 1000.0)
                    .round() as u64
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_to_detected_speech_window() {
        let processor = SimpleVadProcessor;
        let audio = CapturedAudio {
            samples: vec![0.0, 0.0, 0.03, 0.05, 0.04, 0.0],
            sample_rate_hz: 1_000,
            channels: 1,
            duration_ms: 6,
        };

        let result = processor.trim(audio, 0.5).expect("trim should succeed");

        assert!(result.speech_detected);
        assert_eq!(
            result.trimmed_audio.samples,
            vec![0.0, 0.0, 0.03, 0.05, 0.04, 0.0]
        );
        assert_eq!(result.speech_start_ms, Some(2));
        assert_eq!(result.speech_end_ms, Some(4));
    }
}
