#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod commands;
mod events;
mod hotkeys;
mod tray;
mod windows;

use tauri::Manager;

fn setup_error(error: impl Into<anyhow::Error>) -> tauri::Error {
    tauri::Error::Setup(Box::<dyn std::error::Error>::from(error.into()).into())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(|app, shortcut, event| {
                        hotkeys::handle_event(app, shortcut, event.state());
                    })
                    .build(),
            )?;

            let state = app_state::ManagedAppState::initialize().map_err(setup_error)?;
            app.manage(hotkeys::HotkeyBindings::default());
            app.manage(state);
            windows::register(app.handle())?;
            tray::initialize(app.handle())?;
            app.state::<app_state::ManagedAppState>()
                .ensure_model(app.handle().clone());
            let settings = app
                .state::<app_state::ManagedAppState>()
                .services()
                .settings()
                .map_err(setup_error)?;
            hotkeys::sync(app.handle(), &settings)
                .map_err(|error| setup_error(anyhow::anyhow!(error)))?;
            if !settings.start_minimized {
                if let Some(window) = app.get_webview_window(windows::MAIN_WINDOW_LABEL) {
                    window.show()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != windows::MAIN_WINDOW_LABEL {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                tauri::WindowEvent::Resized(_) if window.is_minimized().unwrap_or(false) => {
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_get_dashboard,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::audio_list_input_devices,
            commands::models::model_status_get,
            commands::models::model_download_retry,
            commands::recording::recording_start_manual,
            commands::recording::recording_stop_manual,
            commands::recording::recording_cancel,
            commands::recording::recording_snapshot_get,
            commands::history::history_list,
            commands::history::clipboard_write_text,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Banshee desktop application");
}
