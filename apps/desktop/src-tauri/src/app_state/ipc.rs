use banshee_core::domain::{
    AccelerationPreference, AppError, AppErrorCode, AudioInputDevice, AudioRetentionPolicy,
    DashboardSnapshot, FallbackUsed, HudState, HudStateChanged, OutputMethod, OutputResultKind,
    PipelineRunStatus, RecordingState, RecordingStateChanged, SessionType, Settings,
    SettingsUpdate,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorCodeDto {
    MicrophoneUnavailable,
    MicrophonePermissionDenied,
    ModelMissing,
    InferenceFailed,
    NoSpeechDetected,
    PasteUnavailable,
    ClipboardFailed,
    ProjectIndexFailed,
    SettingsInvalid,
    Unknown,
}

impl From<AppErrorCode> for AppErrorCodeDto {
    fn from(value: AppErrorCode) -> Self {
        match value {
            AppErrorCode::MicrophoneUnavailable => Self::MicrophoneUnavailable,
            AppErrorCode::MicrophonePermissionDenied => Self::MicrophonePermissionDenied,
            AppErrorCode::ModelMissing => Self::ModelMissing,
            AppErrorCode::InferenceFailed => Self::InferenceFailed,
            AppErrorCode::NoSpeechDetected => Self::NoSpeechDetected,
            AppErrorCode::PasteUnavailable => Self::PasteUnavailable,
            AppErrorCode::ClipboardFailed => Self::ClipboardFailed,
            AppErrorCode::ProjectIndexFailed => Self::ProjectIndexFailed,
            AppErrorCode::SettingsInvalid => Self::SettingsInvalid,
            AppErrorCode::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackUsedDto {
    Clipboard,
    History,
    None,
}

impl From<FallbackUsed> for FallbackUsedDto {
    fn from(value: FallbackUsed) -> Self {
        match value {
            FallbackUsed::Clipboard => Self::Clipboard,
            FallbackUsed::History => Self::History,
            FallbackUsed::None => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub code: AppErrorCodeDto,
    pub message: String,
    pub recoverable: bool,
    pub fallback_used: Option<FallbackUsedDto>,
}

impl From<AppError> for AppErrorDto {
    fn from(value: AppError) -> Self {
        Self {
            code: value.code.into(),
            message: value.message,
            recoverable: value.recoverable,
            fallback_used: value.fallback_used.map(Into::into),
        }
    }
}

impl AppErrorDto {
    pub fn settings_invalid(message: impl Into<String>) -> Self {
        Self {
            code: AppErrorCodeDto::SettingsInvalid,
            message: message.into(),
            recoverable: true,
            fallback_used: Some(FallbackUsedDto::None),
        }
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self {
            code: AppErrorCodeDto::Unknown,
            message: message.into(),
            recoverable: false,
            fallback_used: Some(FallbackUsedDto::None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshotDto {
    pub privacy_mode: String,
    pub transcriptions_today: u64,
    pub words_today: u64,
    pub speech_minutes_today: u64,
    pub microphone_name: Option<String>,
    pub speech_model_name: Option<String>,
    pub cleanup_model_name: Option<String>,
    pub active_profile_name: Option<String>,
    pub push_to_talk_shortcut: String,
    pub session_type: SessionType,
}

impl From<DashboardSnapshot> for DashboardSnapshotDto {
    fn from(value: DashboardSnapshot) -> Self {
        Self {
            privacy_mode: value.privacy_mode,
            transcriptions_today: value.transcriptions_today,
            words_today: value.words_today,
            speech_minutes_today: value.speech_minutes_today,
            microphone_name: value.microphone_name,
            speech_model_name: value.speech_model_name,
            cleanup_model_name: value.cleanup_model_name,
            active_profile_name: value.active_profile_name,
            push_to_talk_shortcut: value.push_to_talk_shortcut,
            session_type: value.session_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub launch_at_login: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub show_hud: bool,
    pub play_start_sound: bool,
    pub play_completion_sound: bool,
    pub microphone_device_id: Option<String>,
    pub vad_sensitivity: f64,
    pub push_to_talk_shortcut: String,
    pub toggle_recording_shortcut: String,
    pub cancel_shortcut: String,
    pub repaste_previous_shortcut: String,
    pub acceleration_preference: AccelerationPreference,
    pub history_enabled: bool,
    pub audio_retention_policy: AudioRetentionPolicy,
    pub auto_paste_enabled: bool,
    pub preserve_clipboard: bool,
    pub paste_delay_ms: i64,
    pub cleanup_llm_enabled: bool,
}

impl From<Settings> for SettingsDto {
    fn from(value: Settings) -> Self {
        Self {
            launch_at_login: value.launch_at_login,
            start_minimized: value.start_minimized,
            minimize_to_tray: value.minimize_to_tray,
            show_hud: value.show_hud,
            play_start_sound: value.play_start_sound,
            play_completion_sound: value.play_completion_sound,
            microphone_device_id: value.microphone_device_id,
            vad_sensitivity: value.vad_sensitivity,
            push_to_talk_shortcut: value.push_to_talk_shortcut,
            toggle_recording_shortcut: value.toggle_recording_shortcut,
            cancel_shortcut: value.cancel_shortcut,
            repaste_previous_shortcut: value.repaste_previous_shortcut,
            acceleration_preference: value.acceleration_preference,
            history_enabled: value.history_enabled,
            audio_retention_policy: value.audio_retention_policy,
            auto_paste_enabled: value.auto_paste_enabled,
            preserve_clipboard: value.preserve_clipboard,
            paste_delay_ms: value.paste_delay_ms,
            cleanup_llm_enabled: value.cleanup_llm_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateDto {
    pub launch_at_login: Option<bool>,
    pub start_minimized: Option<bool>,
    pub minimize_to_tray: Option<bool>,
    pub show_hud: Option<bool>,
    pub play_start_sound: Option<bool>,
    pub play_completion_sound: Option<bool>,
    pub microphone_device_id: Option<Option<String>>,
    pub vad_sensitivity: Option<f64>,
    pub push_to_talk_shortcut: Option<String>,
    pub toggle_recording_shortcut: Option<String>,
    pub cancel_shortcut: Option<String>,
    pub repaste_previous_shortcut: Option<String>,
    pub acceleration_preference: Option<AccelerationPreference>,
    pub history_enabled: Option<bool>,
    pub audio_retention_policy: Option<AudioRetentionPolicy>,
    pub auto_paste_enabled: Option<bool>,
    pub preserve_clipboard: Option<bool>,
    pub paste_delay_ms: Option<i64>,
    pub cleanup_llm_enabled: Option<bool>,
}

impl From<SettingsUpdateDto> for SettingsUpdate {
    fn from(value: SettingsUpdateDto) -> Self {
        Self {
            launch_at_login: value.launch_at_login,
            start_minimized: value.start_minimized,
            minimize_to_tray: value.minimize_to_tray,
            show_hud: value.show_hud,
            play_start_sound: value.play_start_sound,
            play_completion_sound: value.play_completion_sound,
            microphone_device_id: value.microphone_device_id,
            vad_sensitivity: value.vad_sensitivity,
            push_to_talk_shortcut: value.push_to_talk_shortcut,
            toggle_recording_shortcut: value.toggle_recording_shortcut,
            cancel_shortcut: value.cancel_shortcut,
            repaste_previous_shortcut: value.repaste_previous_shortcut,
            acceleration_preference: value.acceleration_preference,
            history_enabled: value.history_enabled,
            audio_retention_policy: value.audio_retention_policy,
            auto_paste_enabled: value.auto_paste_enabled,
            preserve_clipboard: value.preserve_clipboard,
            paste_delay_ms: value.paste_delay_ms,
            cleanup_llm_enabled: value.cleanup_llm_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInputDeviceDto {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub channels: Option<u16>,
    pub sample_rate_hz: Option<u32>,
}

impl From<AudioInputDevice> for AudioInputDeviceDto {
    fn from(value: AudioInputDevice) -> Self {
        Self {
            id: value.id,
            name: value.name,
            is_default: value.is_default,
            channels: value.channels,
            sample_rate_hz: value.sample_rate_hz,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudStateChangedDto {
    pub state: HudState,
    pub message: Option<String>,
    pub level: Option<f32>,
    pub live_transcript: Option<String>,
}

impl From<HudStateChanged> for HudStateChangedDto {
    fn from(value: HudStateChanged) -> Self {
        Self {
            state: value.state,
            message: value.message,
            level: value.level,
            live_transcript: value.live_transcript,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStateChangedDto {
    pub state: RecordingState,
    pub transcription_id: Option<String>,
}

impl From<RecordingStateChanged> for RecordingStateChangedDto {
    fn from(value: RecordingStateChanged) -> Self {
        Self {
            state: value.state,
            transcription_id: value.transcription_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResultDto {
    pub session_id: String,
    pub raw_text: String,
    pub deterministic_text: String,
    pub final_text: String,
    pub stt_backend: String,
    pub peak_level: f32,
    pub status: PipelineRunStatus,
    pub output_method: OutputMethod,
    pub output_result: OutputResultKind,
    pub output_message: String,
    pub application_name: String,
    pub window_title: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQueryDto {
    pub limit: usize,
    pub cursor: Option<String>,
}
