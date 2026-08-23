//! Audio capture abstractions for Banshee.

use anyhow::{Result, anyhow, bail};
use banshee_core::domain::{
    AudioCapture, AudioCaptureRequest, AudioInputDevice, CaptureSession, CapturedAudio,
};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("microphone unavailable")]
    MicrophoneUnavailable,
}

#[derive(Clone, Default)]
pub struct CpalAudioCapture {
    sessions: Arc<Mutex<HashMap<String, ActiveSession>>>,
}

struct ActiveSession {
    started_at: Instant,
    sample_rate_hz: u32,
    channels: u16,
}

fn preview_devices() -> Vec<AudioInputDevice> {
    vec![
        AudioInputDevice {
            id: "system-default".to_string(),
            name: "System Default Microphone".to_string(),
            is_default: true,
            channels: Some(1),
            sample_rate_hz: Some(16_000),
        },
        AudioInputDevice {
            id: "usb-headset".to_string(),
            name: "USB Headset Microphone".to_string(),
            is_default: false,
            channels: Some(1),
            sample_rate_hz: Some(16_000),
        },
    ]
}

impl AudioCapture for CpalAudioCapture {
    fn list_input_devices(&self) -> Result<Vec<AudioInputDevice>> {
        Ok(preview_devices())
    }

    fn start(&self, request: AudioCaptureRequest) -> Result<CaptureSession> {
        if request.channels == 0 || request.sample_rate_hz == 0 {
            bail!(AudioError::MicrophoneUnavailable);
        }

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
                    started_at: Instant::now(),
                    sample_rate_hz: request.sample_rate_hz,
                    channels: request.channels,
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

        let duration_ms = active.started_at.elapsed().as_millis().max(250) as u64;
        let sample_count = ((active.sample_rate_hz as u64 * duration_ms) / 1000) as usize;
        let samples = (0..sample_count)
            .map(|index| {
                let t = index as f32 / active.sample_rate_hz as f32;
                (2.0 * PI * 220.0 * t).sin() * 0.12
            })
            .collect();

        Ok(CapturedAudio {
            samples,
            sample_rate_hz: active.sample_rate_hz,
            channels: active.channels,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_preview_devices() {
        let capture = CpalAudioCapture::default();
        let devices = capture.list_input_devices().expect("devices should list");

        assert!(devices.iter().any(|device| device.is_default));
        assert!(
            devices
                .iter()
                .all(|device| device.sample_rate_hz == Some(16_000))
        );
    }

    #[test]
    fn rejects_zero_channel_requests() {
        let capture = CpalAudioCapture::default();
        let error = capture
            .start(AudioCaptureRequest {
                device_id: None,
                channels: 0,
                sample_rate_hz: 16_000,
            })
            .expect_err("zero channel requests should fail");

        assert!(error.to_string().contains("microphone unavailable"));
    }
}
