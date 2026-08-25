use crate::app_state::{ManagedAppState, ipc::AppErrorDto};
use banshee_core::domain::PluginSummary;

#[tauri::command]
pub fn plugins_list(
    state: tauri::State<'_, ManagedAppState>,
) -> Result<Vec<PluginSummary>, AppErrorDto> {
    state
        .plugins()
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}

#[tauri::command]
pub fn plugin_set_enabled(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<Vec<PluginSummary>, AppErrorDto> {
    state
        .set_plugin_enabled(&plugin_id, enabled, app)
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}

#[tauri::command]
pub fn plugin_setup_retry(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
    plugin_id: String,
) -> Result<(), AppErrorDto> {
    state
        .retry_plugin(&plugin_id, app)
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}
