use crate::{
    app_state::ManagedAppState,
    commands::recording::{start_recording_with_origin, stop_recording},
};
use banshee_core::domain::{RecordingOrigin, Settings};
use std::{str::FromStr, sync::Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Default)]
pub struct HotkeyBindings {
    bindings: Mutex<Option<RegisteredHotkeys>>,
}

struct RegisteredHotkeys {
    push_to_talk: Shortcut,
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
        })
    }

    fn all(&self) -> [Shortcut; 1] {
        [self.push_to_talk]
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
                let _ = start_recording_with_origin(app, &app_state, RecordingOrigin::PushToTalk);
            }
            ShortcutState::Released => {
                let should_stop = app_state
                    .recording()
                    .lock()
                    .expect("recording mutex poisoned")
                    .active_session
                    .as_ref()
                    .is_some_and(|session| session.origin == RecordingOrigin::PushToTalk);

                if should_stop {
                    stop_recording_in_background(app.clone());
                }
            }
        }
        return;
    }
}

fn stop_recording_in_background(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<ManagedAppState>().inner().clone();
        let _ = stop_recording(&app, &state);
    });
}
