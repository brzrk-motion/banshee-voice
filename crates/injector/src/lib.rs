//! Target-aware text delivery and clipboard fallback for Banshee.

mod native;

use anyhow::{Result, anyhow};
use arboard::Clipboard;
use banshee_core::domain::{
    OutputBackend, OutputMethod, OutputRequest, OutputResponse, OutputResultKind, OutputTarget,
    SessionType,
};
use std::{thread, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InjectorError {
    #[error("clipboard failed")]
    ClipboardFailed,
}

trait ClipboardAccess {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, value: &str) -> Result<()>;
}

impl<T: ClipboardAccess + ?Sized> ClipboardAccess for &mut T {
    fn get_text(&mut self) -> Option<String> {
        (**self).get_text()
    }

    fn set_text(&mut self, value: &str) -> Result<()> {
        (**self).set_text(value)
    }
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
    fn get_text(&mut self) -> Option<String> {
        self.clipboard.get_text().ok()
    }

    fn set_text(&mut self, value: &str) -> Result<()> {
        self.clipboard
            .set_text(value)
            .map_err(|_| anyhow!(InjectorError::ClipboardFailed))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClipboardInjector;

impl OutputBackend for ClipboardInjector {
    fn capture_target(&self) -> Result<Option<OutputTarget>> {
        native::capture_target()
    }

    fn insert_text(&self, request: OutputRequest) -> Result<OutputResponse> {
        self.insert_with_clipboard(request, SystemClipboard::new()?, native::paste_if_current)
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
        paste: impl FnOnce(&OutputTarget) -> Result<bool>,
    ) -> Result<OutputResponse> {
        let previous = request
            .preserve_clipboard
            .then(|| clipboard.get_text())
            .flatten();
        clipboard.set_text(&request.text)?;

        let may_paste = request.auto_paste_enabled
            && !matches!(request.session_type, SessionType::Wayland)
            && request.target.is_some();

        if may_paste && paste(request.target.as_ref().expect("checked target")).unwrap_or(false) {
            if request.preserve_clipboard
                && request
                    .target
                    .as_ref()
                    .is_some_and(|target| target.editable_verified)
            {
                thread::sleep(Duration::from_millis(request.paste_delay_ms.max(0) as u64));
                if clipboard.get_text().as_deref() == Some(request.text.as_str()) {
                    if let Some(previous) = previous {
                        clipboard.set_text(&previous)?;
                    }
                }
            }
            return Ok(OutputResponse {
                method: OutputMethod::ClipboardPaste,
                result: OutputResultKind::Success,
                message: if request
                    .target
                    .as_ref()
                    .is_some_and(|target| target.editable_verified)
                {
                    "Inserted into the selected text field.".to_string()
                } else {
                    "Paste sent; transcript kept on the clipboard.".to_string()
                },
            });
        }

        Ok(OutputResponse {
            method: OutputMethod::ClipboardCopyOnly,
            result: OutputResultKind::Fallback,
            message: if matches!(request.session_type, SessionType::Wayland) {
                "Copied to clipboard; automatic paste is unavailable on Wayland.".to_string()
            } else {
                "Copied to clipboard; no editable text field was selected.".to_string()
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_core::domain::ScreenRect;

    #[derive(Default)]
    struct MemoryClipboard {
        value: Option<String>,
    }

    impl ClipboardAccess for MemoryClipboard {
        fn get_text(&mut self) -> Option<String> {
            self.value.clone()
        }
        fn set_text(&mut self, value: &str) -> Result<()> {
            self.value = Some(value.to_string());
            Ok(())
        }
    }

    fn target() -> OutputTarget {
        OutputTarget {
            identity: "target-1".to_string(),
            application_name: "Editor".to_string(),
            window_title: "Document".to_string(),
            bounds: Some(ScreenRect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            }),
            editable_verified: true,
        }
    }

    fn request(session_type: SessionType, with_target: bool) -> OutputRequest {
        OutputRequest {
            text: "patched text".to_string(),
            target: with_target.then(target),
            preserve_clipboard: false,
            paste_delay_ms: 0,
            auto_paste_enabled: true,
            session_type,
        }
    }

    #[test]
    fn keeps_transcript_on_clipboard_without_an_editable_target() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard {
            value: Some("previous".into()),
        };
        let response = injector
            .insert_with_clipboard(request(SessionType::X11, false), &mut clipboard, |_| {
                Ok(false)
            })
            .expect("fallback should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardCopyOnly);
        assert_eq!(clipboard.value.as_deref(), Some("patched text"));
    }

    #[test]
    fn wayland_is_always_copy_only() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard::default();
        let response = injector
            .insert_with_clipboard(request(SessionType::Wayland, true), &mut clipboard, |_| {
                Ok(true)
            })
            .expect("fallback should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardCopyOnly);
        assert!(response.message.contains("Wayland"));
    }

    #[test]
    fn restores_clipboard_after_a_confirmed_paste() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard {
            value: Some("previous".into()),
        };
        let mut request = request(SessionType::X11, true);
        request.preserve_clipboard = true;

        let response = injector
            .insert_with_clipboard(request, &mut clipboard, |_| Ok(true))
            .expect("paste should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardPaste);
        assert_eq!(clipboard.value.as_deref(), Some("previous"));
    }

    #[test]
    fn leaves_transcript_available_when_the_locked_target_changed() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard {
            value: Some("previous".into()),
        };

        let response = injector
            .insert_with_clipboard(request(SessionType::X11, true), &mut clipboard, |_| {
                Ok(false)
            })
            .expect("fallback should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardCopyOnly);
        assert_eq!(clipboard.value.as_deref(), Some("patched text"));
    }

    #[test]
    fn paste_backend_errors_degrade_to_the_clipboard() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard::default();

        let response = injector
            .insert_with_clipboard(request(SessionType::X11, true), &mut clipboard, |_| {
                Err(anyhow!("paste backend unavailable"))
            })
            .expect("fallback should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardCopyOnly);
        assert_eq!(clipboard.value.as_deref(), Some("patched text"));
    }

    #[test]
    fn unverified_window_paste_keeps_the_transcript_on_the_clipboard() {
        let injector = ClipboardInjector;
        let mut clipboard = MemoryClipboard {
            value: Some("previous".into()),
        };
        let mut request = request(SessionType::Windows, true);
        request.preserve_clipboard = true;
        request.target.as_mut().expect("target").editable_verified = false;

        let response = injector
            .insert_with_clipboard(request, &mut clipboard, |_| Ok(true))
            .expect("paste should succeed");

        assert_eq!(response.method, OutputMethod::ClipboardPaste);
        assert_eq!(clipboard.value.as_deref(), Some("patched text"));
        assert!(response.message.contains("kept on the clipboard"));
    }
}
