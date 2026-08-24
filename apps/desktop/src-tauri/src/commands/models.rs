use crate::{app_state::ManagedAppState, app_state::ipc::AppErrorDto};
use banshee_models::ModelStatus;

#[tauri::command]
pub fn model_status_get(state: tauri::State<'_, ManagedAppState>) -> ModelStatus {
    state.model_status()
}

#[tauri::command]
pub fn model_download_retry(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<(), AppErrorDto> {
    state.ensure_model(app);
    Ok(())
}
