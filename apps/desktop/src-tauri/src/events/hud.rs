use crate::app_state::ipc::{HudStateChangedDto, RecordingStateChangedDto};
use banshee_core::domain::{HudState, HudStateChanged, RecordingStateChanged};
use tauri::{AppHandle, Emitter, Manager};

pub const HUD_STATE_CHANGED_EVENT: &str = "hud_state_changed";
pub const RECORDING_STATE_CHANGED_EVENT: &str = "recording_state_changed";

pub fn emit_hud_state(app: &AppHandle, payload: HudStateChanged) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("hud") {
        if payload.state == HudState::Hidden {
            window.hide()?;
        } else {
            window.show()?;
        }
    }
    app.emit(HUD_STATE_CHANGED_EVENT, HudStateChangedDto::from(payload))
}

pub fn emit_recording_state(app: &AppHandle, payload: RecordingStateChanged) -> tauri::Result<()> {
    app.emit(
        RECORDING_STATE_CHANGED_EVENT,
        RecordingStateChangedDto::from(payload),
    )
}
