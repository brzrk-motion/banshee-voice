use banshee_core::domain::{PlatformCapabilities, PlatformSupportTier, SessionType};
#[cfg(target_os = "linux")]
use std::env;

#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformCapabilityProbe;

impl PlatformCapabilityProbe {
    pub fn detect(&self) -> PlatformCapabilities {
        let session_type = detect_session_type();

        let (direct_injection, active_window_detection, global_shortcuts) = match session_type {
            SessionType::X11 => (
                PlatformSupportTier::Native,
                PlatformSupportTier::Native,
                PlatformSupportTier::Native,
            ),
            SessionType::Wayland => (
                PlatformSupportTier::Fallback,
                PlatformSupportTier::Fallback,
                PlatformSupportTier::Fallback,
            ),
            SessionType::Windows | SessionType::Macos => (
                PlatformSupportTier::Native,
                PlatformSupportTier::Native,
                PlatformSupportTier::Native,
            ),
            SessionType::Unknown => (
                PlatformSupportTier::Fallback,
                PlatformSupportTier::Fallback,
                PlatformSupportTier::Fallback,
            ),
        };

        PlatformCapabilities {
            session_type,
            direct_injection,
            active_window_detection,
            global_shortcuts,
            tray_supported: !matches!(session_type, SessionType::Unknown),
            hud_supported: true,
        }
    }
}

pub fn detect_session_type() -> SessionType {
    #[cfg(target_os = "windows")]
    {
        return SessionType::Windows;
    }

    #[cfg(target_os = "macos")]
    {
        return SessionType::Macos;
    }

    #[cfg(target_os = "linux")]
    {
        match env::var("XDG_SESSION_TYPE") {
            Ok(value) if value.eq_ignore_ascii_case("x11") => SessionType::X11,
            Ok(value) if value.eq_ignore_ascii_case("wayland") => SessionType::Wayland,
            _ if env::var_os("WAYLAND_DISPLAY").is_some() => SessionType::Wayland,
            _ if env::var_os("DISPLAY").is_some() => SessionType::X11,
            _ => SessionType::Unknown,
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        SessionType::Unknown
    }
}
