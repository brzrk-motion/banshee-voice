//! Platform-specific adapters for Banshee.

pub mod active_window;
pub mod capabilities;

pub use active_window::EnvActiveWindowProvider;
pub use capabilities::{PlatformCapabilityProbe, detect_session_type};
