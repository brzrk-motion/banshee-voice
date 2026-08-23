use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const HUD_WINDOW_LABEL: &str = "hud";

pub fn register(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(HUD_WINDOW_LABEL).is_none() {
        WebviewWindowBuilder::new(
            app,
            HUD_WINDOW_LABEL,
            WebviewUrl::App("index.html?view=hud".into()),
        )
        .title("Banshee HUD")
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .visible(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .inner_size(420.0, 160.0)
        .build()?;
    }

    Ok(())
}
