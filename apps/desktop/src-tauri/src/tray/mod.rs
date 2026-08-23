use tauri::AppHandle;

pub fn initialize(_app: &AppHandle) -> tauri::Result<()> {
    // Phase 2 creates the bootstrap seam so tray behavior can be filled in later
    // without pushing platform setup back into main.rs.
    Ok(())
}
