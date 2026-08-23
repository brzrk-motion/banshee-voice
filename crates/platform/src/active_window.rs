use anyhow::Result;
use banshee_core::domain::{ActiveWindowInfo, ActiveWindowProvider};
use std::env;

#[derive(Debug, Default, Clone, Copy)]
pub struct EnvActiveWindowProvider;

impl ActiveWindowProvider for EnvActiveWindowProvider {
    fn active_window(&self) -> Result<ActiveWindowInfo> {
        let desktop = env::var("XDG_CURRENT_DESKTOP").ok();

        Ok(ActiveWindowInfo {
            application_name: env::var("BANSHEE_ACTIVE_APP")
                .ok()
                .or(desktop)
                .unwrap_or_else(|| "Focused Application".to_string()),
            window_title: env::var("BANSHEE_ACTIVE_WINDOW")
                .unwrap_or_else(|_| "Focused Window".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_placeholder_metadata() {
        let provider = EnvActiveWindowProvider;
        let window = provider
            .active_window()
            .expect("window lookup should succeed");

        assert!(!window.application_name.is_empty());
        assert!(!window.window_title.is_empty());
    }
}
