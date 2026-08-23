use crate::{
    app_state::{ManagedAppState, RecordingTrigger},
    commands::recording::{cancel_recording, start_recording_with_trigger, stop_recording},
};
use banshee_core::domain::Settings;
use std::{str::FromStr, sync::Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Default)]
pub struct HotkeyBindings {
    bindings: Mutex<Option<RegisteredHotkeys>>,
}

struct RegisteredHotkeys {
    push_to_talk: Shortcut,
    toggle_recording: Shortcut,
    cancel: Shortcut,
}

impl Copy for RegisteredHotkeys {}

impl Clone for RegisteredHotkeys {
    fn clone(&self) -> Self {
        *self
    }
}

impl RegisteredHotkeys {
    fn parse(settings: &Settings) -> Result<Self, String> {
        Ok(Self {
            push_to_talk: Shortcut::from_str(&settings.push_to_talk_shortcut)
                .map_err(|error| format!("invalid push-to-talk shortcut: {error}"))?,
            toggle_recording: Shortcut::from_str(&settings.toggle_recording_shortcut)
                .map_err(|error| format!("invalid toggle shortcut: {error}"))?,
            cancel: Shortcut::from_str(&settings.cancel_shortcut)
                .map_err(|error| format!("invalid cancel shortcut: {error}"))?,
        })
    }

    fn all(&self) -> [Shortcut; 3] {
        [self.push_to_talk, self.toggle_recording, self.cancel]
    }
}

pub fn sync(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let next = RegisteredHotkeys::parse(settings)?;
    let bindings = app.state::<HotkeyBindings>();
    let mut state = bindings.bindings.lock().expect("hotkey mutex poisoned");
    let previous = state.as_ref().copied();

    if let Some(current) = previous {
        for shortcut in current.all() {
            let _ = app.global_shortcut().unregister(shortcut);
        }
    }

    let mut registered = Vec::new();
    for shortcut in next.all() {
        if let Err(error) = app.global_shortcut().register(shortcut) {
            for registered_shortcut in registered {
                let _ = app.global_shortcut().unregister(registered_shortcut);
            }
            if let Some(current) = previous {
                for shortcut in current.all() {
                    let _ = app.global_shortcut().register(shortcut);
                }
                *state = Some(current);
            }
            return Err(format!("failed to register {shortcut}: {error}"));
        }
        registered.push(shortcut);
    }

    *state = Some(next);
    Ok(())
}

pub fn handle_event(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) {
    let bindings = app.state::<HotkeyBindings>();
    let Some(registered) = bindings
        .bindings
        .lock()
        .expect("hotkey mutex poisoned")
        .as_ref()
        .copied()
    else {
        return;
    };

    let app_state = app.state::<ManagedAppState>();

    if shortcut == &registered.push_to_talk {
        match state {
            ShortcutState::Pressed => {
                let _ = start_recording_with_trigger(app, &app_state, RecordingTrigger::HoldToTalk);
            }
            ShortcutState::Released => {
                let should_stop = app_state
                    .recording()
                    .lock()
                    .expect("recording mutex poisoned")
                    .active_trigger
                    == Some(RecordingTrigger::HoldToTalk);

                if should_stop {
                    let _ = stop_recording(app, &app_state);
                }
            }
        }
        return;
    }

    if shortcut == &registered.toggle_recording && matches!(state, ShortcutState::Pressed) {
        let active = app_state
            .recording()
            .lock()
            .expect("recording mutex poisoned")
            .active_session
            .is_some();

        if active {
            let _ = stop_recording(app, &app_state);
        } else {
            let _ = start_recording_with_trigger(app, &app_state, RecordingTrigger::Toggle);
        }
        return;
    }

    if shortcut == &registered.cancel && matches!(state, ShortcutState::Pressed) {
        let _ = cancel_recording(app, &app_state);
    }
}
