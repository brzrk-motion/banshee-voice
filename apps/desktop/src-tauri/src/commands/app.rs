use crate::app_state::{
    ManagedAppState,
    ipc::{AppErrorDto, DashboardSnapshotDto},
};

#[tauri::command]
pub fn app_get_dashboard(
    state: tauri::State<'_, ManagedAppState>,
) -> Result<DashboardSnapshotDto, AppErrorDto> {
    state
        .services()
        .dashboard_snapshot()
        .map(Into::into)
        .map_err(|error| AppErrorDto::unknown(error.to_string()))
}
