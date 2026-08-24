use crate::app_state::{
    ManagedAppState,
    ipc::{AppErrorDto, HistoryQueryDto},
};
use banshee_core::domain::HistoryPage;
use banshee_injector::ClipboardInjector;

#[tauri::command]
pub fn history_list(
    state: tauri::State<'_, ManagedAppState>,
    query: HistoryQueryDto,
) -> Result<HistoryPage, AppErrorDto> {
    state
        .history()
        .list(query.limit, query.cursor.as_deref())
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}

#[tauri::command]
pub fn clipboard_write_text(text: String) -> Result<(), AppErrorDto> {
    ClipboardInjector
        .copy_text(&text)
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}
