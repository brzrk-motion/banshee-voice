//! Text injection backends for Banshee.

use anyhow::{Result, anyhow};
use arboard::Clipboard;
use banshee_core::domain::{
    OutputBackend, OutputMethod, OutputRequest, OutputResponse, OutputResultKind, SessionType,
};
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InjectorError {
    #[error("clipboard failed")]
    ClipboardFailed,
}

trait ClipboardAccess {
    fn set_text(&mut self, value: &str) -> Result<()>;
}

struct SystemClipboard {
    clipboard: Clipboard,
}

impl SystemClipboard {
    fn new() -> Result<Self> {
        Ok(Self {
            clipboard: Clipboard::new().map_err(|_| anyhow!(InjectorError::ClipboardFailed))?,
        })
    }
}

impl ClipboardAccess for SystemClipboard {
    fn set_text(&mut self, value: &str) -> Result<()> {
        self.clipboard
            .set_text(value)
            .map_err(|_| anyhow!(InjectorError::ClipboardFailed))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClipboardInjector;

impl OutputBackend for ClipboardInjector {
    fn insert_text(&self, request: OutputRequest) -> Result<OutputResponse> {
        self.insert_with_clipboard(request, SystemClipboard::new()?)
    }
}

impl ClipboardInjector {
    pub fn copy_text(&self, text: &str) -> Result<()> {
        let mut clipboard = SystemClipboard::new()?;
        clipboard.set_text(text)
    }

    fn insert_with_clipboard(
        &self,
        request: OutputRequest,
        mut clipboard: impl ClipboardAccess,
    ) -> Result<OutputResponse> {
        let supports_direct_insert = env::var("BANSHEE_DIRECT_INSERT")
            .map(|value| value == "1")
            .unwrap_or(false)
            && !matches!(request.session_type, SessionType::Wayland)
            && request.auto_paste_enabled;

        if supports_direct_insert {
            return Ok(OutputResponse {
                method: OutputMethod::DirectInsert,
                result: OutputResultKind::Success,
                message: "Transcript inserted directly into the focused application.".to_string(),
            });
        }

        clipboard.set_text(&request.text)?;

        let result = OutputResultKind::Fallback;
        let method = OutputMethod::ClipboardCopyOnly;
        let message = if request.preserve_clipboard {
            "Transcript copied to the clipboard. Preserving the previous clipboard requires a real paste backend, which is not available in this preview path."
        } else {
            "Transcript copied to the clipboard because direct paste is unavailable in this session."
        };

        Ok(OutputResponse {
            method,
            result,
            message: message.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryClipboard {
        value: Option<String>,
    }

    impl ClipboardAccess for MemoryClipboard {
        fn set_text(&mut self, value: &str) -> Result<()> {
            self.value = Some(value.to_string());
            Ok(())
        }
    }

    impl ClipboardAccess for &mut MemoryClipboard {
        fn set_text(&mut self, value: &str) -> Result<()> {
            self.value = Some(value.to_string());
            Ok(())
        }
    }

    fn request(
        session_type: SessionType,
        auto_paste_enabled: bool,
        preserve_clipboard: bool,
    ) -> OutputRequest {
        OutputRequest {
            text: "patched text".to_string(),
            preserve_clipboard,
            paste_delay_ms: 40,
            auto_paste_enabled,
            session_type,
        }
    }

    #[test]
    fn keeps_transcript_on_clipboard_for_copy_only_fallback() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard {
            value: Some("previous clipboard".to_string()),
        };

        let response = injector
            .insert_with_clipboard(request(SessionType::Wayland, true, true), &mut clipboard)
            .expect("insertion should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardCopyOnly);
        assert_eq!(clipboard.value.as_deref(), Some("patched text"));
    }

    #[test]
    fn explains_preserve_clipboard_limitation_in_preview_mode() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard {
            value: Some("previous clipboard".to_string()),
        };

        let response = injector
            .insert_with_clipboard(request(SessionType::X11, true, true), &mut clipboard)
            .expect("insertion should succeed");

        assert!(response.message.contains("requires a real paste backend"));
        assert_eq!(clipboard.value.as_deref(), Some("patched text"));
    }
}
