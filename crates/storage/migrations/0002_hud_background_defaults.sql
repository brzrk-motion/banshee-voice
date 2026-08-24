UPDATE settings
SET show_hud = 1,
    minimize_to_tray = 1,
    auto_paste_enabled = 1,
    updated_at = CURRENT_TIMESTAMP
WHERE id = 1;
