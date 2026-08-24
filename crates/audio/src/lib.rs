//! Cross-platform microphone capture for Banshee.

use anyhow::{Result, anyhow, bail};
use banshee_core::domain::{
    AudioCapture, AudioCaptureRequest, AudioInputDevice, CaptureSession, CapturedAudio,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("microphone unavailable")]
    MicrophoneUnavailable,
    #[error("microphone permission denied")]
    PermissionDenied,
    #[error("microphone stream failed: {0}")]
    StreamFailed(String),
}

#[derive(Clone, Default)]
pub struct CpalAudioCapture {
    sessions: Arc<Mutex<HashMap<String, ActiveSession>>>,
}

struct ActiveSession {
    _stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    stream_error: Arc<Mutex<Option<String>>>,
    level_bits: Arc<AtomicU32>,
    sample_rate_hz: u32,
    channels: u16,
}

fn append_and_measure(
    samples: &Arc<Mutex<Vec<f32>>>,
    level_bits: &Arc<AtomicU32>,
    values: impl IntoIterator<Item = f32>,
) {
    let mut buffer = samples.lock().expect("audio buffer mutex poisoned");
    let mut sum_squares = 0.0_f32;
    let mut count = 0_u32;
    for value in values {
        buffer.push(value);
        sum_squares += value * value;
        count += 1;
    }
    let rms = if count == 0 {
        0.0
    } else {
        (sum_squares / count as f32).sqrt().clamp(0.0, 1.0)
    };
    level_bits.store(rms.to_bits(), Ordering::Relaxed);
}

fn input_devices() -> Result<Vec<cpal::Device>> {
    cpal::default_host()
        .input_devices()
        .map(|devices| devices.collect())
        .map_err(map_device_error)
}

fn map_device_error(error: impl std::fmt::Display) -> anyhow::Error {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("permission")
        || message.to_ascii_lowercase().contains("access denied")
    {
        anyhow!(AudioError::PermissionDenied)
    } else {
        anyhow!(AudioError::StreamFailed(message))
    }
}

fn select_device(device_id: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if let Some(device_id) = device_id {
        if device_id != "system-default" {
            return input_devices()?
                .into_iter()
                .find(|device| device.name().ok().as_deref() == Some(device_id))
                .ok_or_else(|| anyhow!(AudioError::MicrophoneUnavailable));
        }
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!(AudioError::MicrophoneUnavailable))
}

fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let channels = usize::from(channels.max(1));
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == target_rate {
        return samples.to_vec();
    }
    let output_len = ((samples.len() as u64 * target_rate as u64) / source_rate as u64) as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    (0..output_len)
        .map(|index| {
            let source_position = index as f64 * ratio;
            let lower = source_position.floor() as usize;
            let upper = (lower + 1).min(samples.len() - 1);
            let fraction = (source_position - lower as f64) as f32;
            samples[lower] + (samples[upper] - samples[lower]) * fraction
        })
        .collect()
}

fn build_stream(
    device: &cpal::Device,
    supported: &cpal::SupportedStreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    stream_error: Arc<Mutex<Option<String>>>,
    level_bits: Arc<AtomicU32>,
) -> Result<Stream> {
    let config: StreamConfig = supported.clone().into();
    let on_error = move |error: cpal::StreamError| {
        *stream_error.lock().expect("stream error mutex poisoned") = Some(error.to_string());
    };
    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| append_and_measure(&samples, &level_bits, data.iter().copied()),
            on_error,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                append_and_measure(
                    &samples,
                    &level_bits,
                    data.iter().map(|sample| *sample as f32 / i16::MAX as f32),
                );
            },
            on_error,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                append_and_measure(
                    &samples,
                    &level_bits,
                    data.iter()
                        .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0),
                );
            },
            on_error,
            None,
        ),
        format => {
            return Err(anyhow!(AudioError::StreamFailed(format!(
                "unsupported sample format {format}"
            ))));
        }
    }
    .map_err(map_device_error)?;
    Ok(stream)
}

impl AudioCapture for CpalAudioCapture {
    fn list_input_devices(&self) -> Result<Vec<AudioInputDevice>> {
        let default_name = cpal::default_host()
            .default_input_device()
            .and_then(|device| device.name().ok());
        input_devices()?
            .into_iter()
            .map(|device| {
                let name = device.name().map_err(map_device_error)?;
                let config = device.default_input_config().map_err(map_device_error)?;
                Ok(AudioInputDevice {
                    id: name.clone(),
                    is_default: default_name.as_deref() == Some(name.as_str()),
                    name,
                    channels: Some(config.channels()),
                    sample_rate_hz: Some(config.sample_rate().0),
                })
            })
            .collect()
    }

    fn start(&self, request: AudioCaptureRequest) -> Result<CaptureSession> {
        if request.channels == 0 || request.sample_rate_hz == 0 {
            bail!(AudioError::MicrophoneUnavailable);
        }
        let device = select_device(request.device_id.as_deref())?;
        let supported = device.default_input_config().map_err(map_device_error)?;
        let samples = Arc::new(Mutex::new(Vec::new()));
        let stream_error = Arc::new(Mutex::new(None));
        let level_bits = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
        let stream = build_stream(
            &device,
            &supported,
            samples.clone(),
            stream_error.clone(),
            level_bits.clone(),
        )?;
        stream.play().map_err(map_device_error)?;
        let session = CaptureSession {
            id: format!(
                "capture-{}",
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
            ),
            device_id: request.device_id,
        };
        self.sessions
            .lock()
            .expect("audio sessions mutex poisoned")
            .insert(
                session.id.clone(),
                ActiveSession {
                    _stream: stream,
                    samples,
                    stream_error,
                    level_bits,
                    sample_rate_hz: supported.sample_rate().0,
                    channels: supported.channels(),
                },
            );
        Ok(session)
    }

    fn stop(&self, session: &CaptureSession) -> Result<CapturedAudio> {
        let active = self
            .sessions
            .lock()
            .expect("audio sessions mutex poisoned")
            .remove(&session.id)
            .ok_or_else(|| anyhow!("unknown capture session"))?;
        drop(active._stream);
        if let Some(error) = active
            .stream_error
            .lock()
            .expect("stream error mutex poisoned")
            .take()
        {
            bail!(AudioError::StreamFailed(error));
        }
        let interleaved = active.samples.lock().expect("audio buffer mutex poisoned");
        let mono = downmix_to_mono(&interleaved, active.channels);
        let samples = resample_linear(&mono, active.sample_rate_hz, 16_000);
        let duration_ms = (samples.len() as u64 * 1_000) / 16_000;
        Ok(CapturedAudio {
            samples,
            sample_rate_hz: 16_000,
            channels: 1,
            duration_ms,
        })
    }

    fn cancel(&self, session: &CaptureSession) -> Result<()> {
        self.sessions
            .lock()
            .expect("audio sessions mutex poisoned")
            .remove(&session.id);
        Ok(())
    }

    fn current_level(&self, session: &CaptureSession) -> Result<f32> {
        let sessions = self.sessions.lock().expect("audio sessions mutex poisoned");
        let active = sessions
            .get(&session.id)
            .ok_or_else(|| anyhow!("unknown capture session"))?;
        Ok(f32::from_bits(active.level_bits.load(Ordering::Relaxed)).clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmixes_stereo_frames() {
        assert_eq!(downmix_to_mono(&[1.0, -1.0, 0.5, 0.5], 2), vec![0.0, 0.5]);
    }

    #[test]
    fn resamples_to_requested_rate() {
        assert_eq!(
            resample_linear(&vec![0.0; 48_000], 48_000, 16_000).len(),
            16_000
        );
    }

    #[test]
    fn publishes_normalized_rms_for_the_hud() {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let level_bits = Arc::new(AtomicU32::new(0.0_f32.to_bits()));

        append_and_measure(&samples, &level_bits, [0.5, -0.5, 0.5, -0.5]);

        assert_eq!(samples.lock().expect("samples").len(), 4);
        assert!((f32::from_bits(level_bits.load(Ordering::Relaxed)) - 0.5).abs() < f32::EPSILON);
    }
}
