use crate::{app_state::ManagedAppState, app_state::ipc::AppErrorDto};
use banshee_models::{ModelCapability, ModelsStatus};

#[tauri::command]
pub fn models_status_get(state: tauri::State<'_, ManagedAppState>) -> ModelsStatus {
    state.models_status()
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
