use crate::{app_state::ManagedAppState, app_state::ipc::AppErrorDto};
use banshee_models::{ModelCapability, ModelStatus, ModelsStatus};

#[tauri::command]
pub fn models_status_get(state: tauri::State<'_, ManagedAppState>) -> ModelsStatus {
    state.models_status()
}

// Keep the original singular command available for installed frontend assets
// that may still be cached while the desktop binary is being upgraded.
#[tauri::command]
pub fn model_status_get(state: tauri::State<'_, ManagedAppState>) -> ModelStatus {
    state.models_status().speech
}

#[tauri::command]
pub fn model_download_retry(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
    capability: ModelCapability,
) -> Result<(), AppErrorDto> {
    state.retry_model(capability, app);
    Ok(())
}
