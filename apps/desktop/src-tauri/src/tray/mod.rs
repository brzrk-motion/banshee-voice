use tauri::{AppHandle, Emitter, Manager, menu::MenuBuilder, tray::TrayIconBuilder};

fn show_main(app: &AppHandle, page: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        if let Some(page) = page {
            let _ = app.emit_to("main", "navigate_to_page", page);
        }
    }
}

pub fn initialize(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open", "Open Banshee")
        .text("settings", "Settings")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let mut builder = TrayIconBuilder::with_id("banshee-tray")
        .menu(&menu)
        .tooltip("Banshee - push-to-talk ready")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app, None),
            "settings" => show_main(app, Some("settings")),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
