use crate::app_state::{ManagedAppState, ipc::AppErrorDto};
use banshee_core::domain::{DictionaryEntry, DictionaryStore};

#[tauri::command]
pub fn dictionary_entries_get(
    state: tauri::State<'_, ManagedAppState>,
) -> Result<Vec<DictionaryEntry>, AppErrorDto> {
    state
        .dictionary()
        .list_global()
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}

#[tauri::command]
pub fn dictionary_entries_replace(
    state: tauri::State<'_, ManagedAppState>,
    entries: Vec<DictionaryEntry>,
) -> Result<Vec<DictionaryEntry>, AppErrorDto> {
    state
        .dictionary()
        .replace_global(entries)
        .map_err(|error| AppErrorDto::settings_invalid(error.to_string()))
}
