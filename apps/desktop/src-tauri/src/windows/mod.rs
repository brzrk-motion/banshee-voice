use banshee_core::domain::ScreenRect;
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const HUD_WINDOW_LABEL: &str = "hud";

pub fn register(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(HUD_WINDOW_LABEL).is_none() {
        let window = WebviewWindowBuilder::new(
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
        .focusable(false)
        .shadow(false)
        .inner_size(360.0, 72.0)
        .build()?;

        // GTK must realize the native window before Tao can set its input shape.
        window.show()?;
        window.set_ignore_cursor_events(true)?;
        window.hide()?;
    }

    Ok(())
}

pub fn position_hud(app: &AppHandle, target: Option<ScreenRect>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(HUD_WINDOW_LABEL) else {
        return Ok(());
    };
    let monitors = app.available_monitors()?;
    let selected = target.and_then(|rect| {
        let center_x = rect.x as f64 + rect.width as f64 / 2.0;
        let center_y = rect.y as f64 + rect.height as f64 / 2.0;
        monitors
            .iter()
            .find(|monitor| {
                let area = monitor.work_area();
                let physical_match = center_x >= area.position.x as f64
                    && center_x < (area.position.x + area.size.width as i32) as f64
                    && center_y >= area.position.y as f64
                    && center_y < (area.position.y + area.size.height as i32) as f64;
                let scale = monitor.scale_factor();
                let logical_match = center_x >= area.position.x as f64 / scale
                    && center_x < (area.position.x as f64 + area.size.width as f64) / scale
                    && center_y >= area.position.y as f64 / scale
                    && center_y < (area.position.y as f64 + area.size.height as f64) / scale;
                physical_match || logical_match
            })
            .cloned()
    });
    let monitor = match selected {
        Some(monitor) => Some(monitor),
        None => app.primary_monitor()?,
    };
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let area = monitor.work_area();
    let size = window.outer_size()?;
    let bottom_gap = (24.0 * monitor.scale_factor()).round() as i32;
    let x = area.position.x + (area.size.width.saturating_sub(size.width) / 2) as i32;
    let y = area.position.y + area.size.height.saturating_sub(size.height) as i32 - bottom_gap;
    window.set_position(PhysicalPosition::new(x, y))
}
