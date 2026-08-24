use crate::{
    app_state::ipc::{AudioLevelChangedDto, HudStateChangedDto, RecordingStateChangedDto},
    windows,
};
use banshee_core::domain::{
    AudioLevelChanged, HudState, HudStateChanged, RecordingStateChanged, ScreenRect,
};
use tauri::{AppHandle, Emitter, Manager};

pub const HUD_STATE_CHANGED_EVENT: &str = "hud_state_changed";
pub const RECORDING_STATE_CHANGED_EVENT: &str = "recording_state_changed";
pub const AUDIO_LEVEL_CHANGED_EVENT: &str = "audio_level_changed";

pub fn emit_hud_state(
    app: &AppHandle,
    payload: HudStateChanged,
    target_bounds: Option<ScreenRect>,
) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("hud") {
        if payload.state == HudState::Hidden {
            window.hide()?;
        } else {
            windows::position_hud(app, target_bounds)?;
            window.show()?;
        }
    }
    app.emit(HUD_STATE_CHANGED_EVENT, HudStateChangedDto::from(payload))
}

pub fn emit_audio_level(app: &AppHandle, payload: AudioLevelChanged) -> tauri::Result<()> {
    app.emit(
        AUDIO_LEVEL_CHANGED_EVENT,
        AudioLevelChangedDto::from(payload),
    )
}

pub fn emit_recording_state(app: &AppHandle, payload: RecordingStateChanged) -> tauri::Result<()> {
    app.emit(
        RECORDING_STATE_CHANGED_EVENT,
        RecordingStateChangedDto::from(payload),
    )
}
