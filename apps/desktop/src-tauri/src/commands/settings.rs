use crate::app_state::{
    ManagedAppState,
    ipc::{
        AccelerationStatusDto, AppErrorDto, AudioInputDeviceDto, SettingsDto, SettingsUpdateDto,
    },
};
use crate::hotkeys;
use banshee_core::storage::settings_repo::SettingsValidationError;

#[tauri::command]
pub fn settings_get(state: tauri::State<'_, ManagedAppState>) -> Result<SettingsDto, AppErrorDto> {
    state
        .services()
        .settings()
        .map(Into::into)
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}

#[tauri::command]
pub fn acceleration_status_get(state: tauri::State<'_, ManagedAppState>) -> AccelerationStatusDto {
    state.acceleration_status()
}

#[tauri::command]
pub fn settings_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
    payload: SettingsUpdateDto,
) -> Result<SettingsDto, AppErrorDto> {
    let previous = state
        .services()
        .settings()
        .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    let next = state
        .services()
        .update_settings(payload.into())
        .map_err(|error| {
            if let Some(validation_error) = error.downcast_ref::<SettingsValidationError>() {
                AppErrorDto::settings_invalid(validation_error.to_string())
            } else {
                AppErrorDto::unknown(error.to_string())
            }
        })?;

    if let Err(error) = hotkeys::sync(&app, &next) {
        let _ = state.services().update_settings(previous.clone().into());
        let _ = hotkeys::sync(&app, &previous);
        return Err(AppErrorDto::settings_invalid(error));
    }

    if next.acceleration_preference != previous.acceleration_preference {
        if let Err(error) = state.set_acceleration_preference(next.acceleration_preference) {
            let _ = state.services().update_settings(previous.clone().into());
            let _ = hotkeys::sync(&app, &previous);
            return Err(AppErrorDto::settings_invalid(error.to_string()));
        }
    }

    Ok(next.into())
}

#[tauri::command]
pub fn audio_list_input_devices(
    state: tauri::State<'_, ManagedAppState>,
) -> Result<Vec<AudioInputDeviceDto>, AppErrorDto> {
    Ok(state
        .services()
        .audio_input_devices()
        .into_iter()
        .map(Into::into)
        .collect())
}
