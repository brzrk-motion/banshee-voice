use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    X11,
    Wayland,
    Windows,
    Macos,
    Unknown,
}

impl SessionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccelerationPreference {
    Auto,
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioRetentionPolicy {
    Never,
    Hours24,
    Forever,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformSupportTier {
    Native,
    Fallback,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordingState {
    Idle,
    Recording,
    Stopping,
    Transcribing,
    Inserting,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HudState {
    Hidden,
    Listening,
    Processing,
    Complete,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMethod {
    DirectInsert,
    ClipboardPaste,
    ClipboardCopyOnly,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputResultKind {
    Success,
    Fallback,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineRunStatus {
    Completed,
    FallbackUsed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorCode {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackUsed {
    Clipboard,
    History,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            start_minimized: false,
            minimize_to_tray: true,
            show_hud: true,
            play_start_sound: false,
            play_completion_sound: false,
            microphone_device_id: None,
            vad_sensitivity: 0.5,
            push_to_talk_shortcut: "Ctrl+Shift+Space".to_string(),
            toggle_recording_shortcut: "Ctrl+Shift+R".to_string(),
            cancel_shortcut: "Escape".to_string(),
            repaste_previous_shortcut: "Ctrl+Shift+V".to_string(),
            acceleration_preference: AccelerationPreference::Auto,
            history_enabled: true,
            audio_retention_policy: AudioRetentionPolicy::Never,
            auto_paste_enabled: true,
            preserve_clipboard: true,
            paste_delay_ms: 40,
            cleanup_llm_enabled: false,
        }
    }
}

impl From<Settings> for SettingsUpdate {
    fn from(value: Settings) -> Self {
        Self {
            launch_at_login: Some(value.launch_at_login),
            start_minimized: Some(value.start_minimized),
            minimize_to_tray: Some(value.minimize_to_tray),
            show_hud: Some(value.show_hud),
            play_start_sound: Some(value.play_start_sound),
            play_completion_sound: Some(value.play_completion_sound),
            microphone_device_id: Some(value.microphone_device_id),
            vad_sensitivity: Some(value.vad_sensitivity),
            push_to_talk_shortcut: Some(value.push_to_talk_shortcut),
            toggle_recording_shortcut: Some(value.toggle_recording_shortcut),
            cancel_shortcut: Some(value.cancel_shortcut),
            repaste_previous_shortcut: Some(value.repaste_previous_shortcut),
            acceleration_preference: Some(value.acceleration_preference),
            history_enabled: Some(value.history_enabled),
            audio_retention_policy: Some(value.audio_retention_policy),
            auto_paste_enabled: Some(value.auto_paste_enabled),
            preserve_clipboard: Some(value.preserve_clipboard),
            paste_delay_ms: Some(value.paste_delay_ms),
            cleanup_llm_enabled: Some(value.cleanup_llm_enabled),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub built_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioInputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub channels: Option<u16>,
    pub sample_rate_hz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub session_type: SessionType,
    pub direct_injection: PlatformSupportTier,
    pub active_window_detection: PlatformSupportTier,
    pub global_shortcuts: PlatformSupportTier,
    pub tray_supported: bool,
    pub hud_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HudStateChanged {
    pub state: HudState,
    pub message: Option<String>,
    pub level: Option<f32>,
    pub live_transcript: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStateChanged {
    pub state: RecordingState,
    pub transcription_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
    pub recoverable: bool,
    pub fallback_used: Option<FallbackUsed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioCaptureRequest {
    pub device_id: Option<String>,
    pub channels: u16,
    pub sample_rate_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSession {
    pub id: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapturedAudio {
    pub samples: Vec<f32>,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VadResult {
    pub trimmed_audio: CapturedAudio,
    pub speech_detected: bool,
    pub peak_level: f32,
    pub speech_start_ms: Option<u64>,
    pub speech_end_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionRequest {
    pub audio: CapturedAudio,
    pub language: String,
    pub acceleration_preference: AccelerationPreference,
    pub latency_profile: String,
    pub selected_model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOutput {
    pub raw_text: String,
    pub backend: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRequest {
    pub raw_text: String,
    pub profile: ProfileSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOutput {
    pub deterministic_text: String,
    pub final_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindowInfo {
    pub application_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputRequest {
    pub text: String,
    pub preserve_clipboard: bool,
    pub paste_delay_ms: i64,
    pub auto_paste_enabled: bool,
    pub session_type: SessionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputResponse {
    pub method: OutputMethod,
    pub result: OutputResultKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRunResult {
    pub session_id: String,
    pub raw_text: String,
    pub deterministic_text: String,
    pub final_text: String,
    pub stt_backend: String,
    pub peak_level: f32,
    pub status: PipelineRunStatus,
    pub output: OutputResponse,
    pub active_window: ActiveWindowInfo,
    pub duration_ms: u64,
    pub profile_id: String,
    pub acceleration_preference: AccelerationPreference,
    pub session_type: SessionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub created_at: String,
    pub final_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<HistoryItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSnapshot {
    pub state: RecordingState,
    pub hud: HudStateChanged,
    pub last_transcript: Option<String>,
    pub last_error: Option<AppError>,
}

pub trait SettingsStore: Send + Sync {
    fn load(&self) -> anyhow::Result<Settings>;
    fn update(&self, update: SettingsUpdate) -> anyhow::Result<Settings>;
}

pub trait ProfileStore: Send + Sync {
    fn list(&self) -> anyhow::Result<Vec<ProfileSummary>>;
    fn default_profile(&self) -> anyhow::Result<ProfileSummary>;
}

pub trait CapabilityProbe: Send + Sync {
    fn probe(&self) -> PlatformCapabilities;
}

pub trait AudioCapture: Send + Sync {
    fn list_input_devices(&self) -> anyhow::Result<Vec<AudioInputDevice>>;
    fn start(&self, request: AudioCaptureRequest) -> anyhow::Result<CaptureSession>;
    fn stop(&self, session: &CaptureSession) -> anyhow::Result<CapturedAudio>;
    fn cancel(&self, session: &CaptureSession) -> anyhow::Result<()>;
}

pub trait VadProcessor: Send + Sync {
    fn trim(&self, audio: CapturedAudio, sensitivity: f64) -> anyhow::Result<VadResult>;
}

pub trait TranscriptionEngine: Send + Sync {
    fn transcribe(&self, request: TranscriptionRequest) -> anyhow::Result<TranscriptionOutput>;
}

pub trait CleanupEngine: Send + Sync {
    fn cleanup(&self, request: CleanupRequest) -> anyhow::Result<CleanupOutput>;
}

pub trait OutputBackend: Send + Sync {
    fn insert_text(&self, request: OutputRequest) -> anyhow::Result<OutputResponse>;
}

pub trait ActiveWindowProvider: Send + Sync {
    fn active_window(&self) -> anyhow::Result<ActiveWindowInfo>;
}
