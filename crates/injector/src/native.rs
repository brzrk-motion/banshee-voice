use anyhow::Result;
use banshee_core::domain::OutputTarget;

#[cfg(windows)]
mod platform {
    use super::*;
    use banshee_core::domain::ScreenRect;
    use windows::Win32::{
        Foundation::{HWND, RECT},
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomationTextEditPattern,
                IUIAutomationValuePattern, UIA_EditControlTypeId, UIA_TextEditPatternId,
                UIA_ValuePatternId,
            },
            Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_CONTROL,
                VK_V,
            },
            WindowsAndMessaging::{
                GUITHREADINFO, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowRect,
                GetWindowTextW, GetWindowThreadProcessId,
            },
        },
    };

    fn window_string(hwnd: HWND, class_name: bool) -> String {
        let mut buffer = [0_u16; 512];
        let length = unsafe {
            if class_name {
                GetClassNameW(hwnd, &mut buffer)
            } else {
                GetWindowTextW(hwnd, &mut buffer)
            }
        };
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }

    fn window_target() -> Option<OutputTarget> {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.0.is_null() {
                return None;
            }
            let mut process_id = 0_u32;
            let thread_id = GetWindowThreadProcessId(foreground, Some(&mut process_id));
            let mut gui = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            let focus =
                if GetGUIThreadInfo(thread_id, &mut gui).is_ok() && !gui.hwndFocus.0.is_null() {
                    gui.hwndFocus
                } else {
                    foreground
                };
            let mut rect = RECT::default();
            let bounds = GetWindowRect(foreground, &mut rect)
                .ok()
                .map(|_| ScreenRect {
                    x: rect.left,
                    y: rect.top,
                    width: (rect.right - rect.left).max(0) as u32,
                    height: (rect.bottom - rect.top).max(0) as u32,
                });
            Some(OutputTarget {
                identity: format!(
                    "window:{process_id}:{}:{}",
                    foreground.0 as usize, focus.0 as usize
                ),
                application_name: window_string(foreground, true),
                window_title: window_string(foreground, false),
                bounds,
                editable_verified: false,
            })
        }
    }

    fn automation_target() -> Result<Option<OutputTarget>> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let element = automation.GetFocusedElement()?;
            let enabled = element.CurrentIsEnabled()?.as_bool();
            let focusable = element.CurrentIsKeyboardFocusable()?.as_bool();
            let password = element.CurrentIsPassword()?.as_bool();
            let control_type = element.CurrentControlType()?;
            let automation_id = element.CurrentAutomationId()?.to_string();
            let name = element.CurrentName()?.to_string();
            let element_rect = element.CurrentBoundingRectangle().unwrap_or_default();
            let has_text_edit_pattern = element
                .GetCurrentPatternAs::<IUIAutomationTextEditPattern>(UIA_TextEditPatternId)
                .is_ok();
            let writable_value = element
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                .ok()
                .and_then(|pattern| pattern.CurrentIsReadOnly().ok())
                .is_some_and(|read_only| !read_only.as_bool());
            let editable_type =
                has_text_edit_pattern || writable_value || control_type == UIA_EditControlTypeId;
            if !enabled || !focusable || password || !editable_type {
                return Ok(None);
            }

            let process_id = element.CurrentProcessId()?;
            let hwnd = GetForegroundWindow();
            let mut rect = RECT::default();
            let bounds = GetWindowRect(hwnd, &mut rect).ok().map(|_| ScreenRect {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(0) as u32,
                height: (rect.bottom - rect.top).max(0) as u32,
            });

            Ok(Some(OutputTarget {
                identity: format!(
                    "{process_id}:{automation_id}:{name}:{}:{}:{}:{}",
                    element_rect.left, element_rect.top, element_rect.right, element_rect.bottom
                ),
                application_name: format!("Process {process_id}"),
                window_title: name,
                bounds,
                editable_verified: true,
            }))
        }
    }

    fn send_paste() -> bool {
        let key = |vk, flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        let events = [
            key(VK_CONTROL, Default::default()),
            key(VK_V, Default::default()),
            key(VK_V, KEYEVENTF_KEYUP),
            key(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        (unsafe { SendInput(&events, std::mem::size_of::<INPUT>() as i32) }) == events.len() as u32
    }

    fn focused_target() -> Result<Option<OutputTarget>> {
        Ok(automation_target().ok().flatten().or_else(window_target))
    }

    pub fn capture_target() -> Result<Option<OutputTarget>> {
        focused_target()
    }

    pub fn paste_if_current(target: &OutputTarget) -> Result<bool> {
        let current = if target.editable_verified {
            automation_target()?
        } else {
            window_target()
        };
        let Some(current) = current else {
            return Ok(false);
        };
        if current.identity != target.identity {
            return Ok(false);
        }
        Ok(send_paste())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::process::Command;

    fn query() -> Result<Option<OutputTarget>> {
        let script = r#"tell application "System Events"
set p to first application process whose frontmost is true
set e to value of attribute "AXFocusedUIElement" of p
set r to value of attribute "AXRole" of e
if r is not in {"AXTextField", "AXTextArea", "AXComboBox"} then return ""
if value of attribute "AXEnabled" of e is false then return ""
if settable of attribute "AXValue" of e is false then return ""
set elementID to ""
try
    set elementID to value of attribute "AXIdentifier" of e as text
end try
set elementPosition to value of attribute "AXPosition" of e
set windowPosition to position of front window of p
set windowSize to size of front window of p
return (name of p as text) & "|" & r & "|" & elementID & "|" & (item 1 of elementPosition as text) & ":" & (item 2 of elementPosition as text) & "|" & (item 1 of windowPosition as text) & "|" & (item 2 of windowPosition as text) & "|" & (item 1 of windowSize as text) & "|" & (item 2 of windowSize as text)
end tell"#;
        let output = Command::new("osascript").args(["-e", script]).output()?;
        if !output.status.success() {
            return Ok(None);
        }
        let identity = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if identity.is_empty() {
            return Ok(None);
        }
        let fields = identity.split('|').collect::<Vec<_>>();
        let application_name = fields
            .first()
            .copied()
            .unwrap_or("Focused application")
            .to_string();
        let bounds = fields.get(4..8).and_then(|values| {
            Some(banshee_core::domain::ScreenRect {
                x: values[0].parse().ok()?,
                y: values[1].parse().ok()?,
                width: values[2].parse().ok()?,
                height: values[3].parse().ok()?,
            })
        });
        Ok(Some(OutputTarget {
            identity,
            application_name,
            window_title: String::new(),
            bounds,
            editable_verified: true,
        }))
    }

    pub fn capture_target() -> Result<Option<OutputTarget>> {
        query()
    }

    pub fn paste_if_current(target: &OutputTarget) -> Result<bool> {
        if query()?.as_ref().map(|value| &value.identity) != Some(&target.identity) {
            return Ok(false);
        }
        let status = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to keystroke \"v\" using command down",
            ])
            .status()?;
        Ok(status.success())
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use atspi::{AccessibilityConnection, ObjectRefOwned, Role, State, connection::P2P};
    use banshee_core::domain::ScreenRect;
    use futures_lite::future::block_on;
    use std::collections::VecDeque;
    use x11rb::{
        CURRENT_TIME, connect,
        connection::Connection,
        protocol::{
            xproto::{AtomEnum, ConnectionExt as _, KEY_PRESS_EVENT, KEY_RELEASE_EVENT, Window},
            xtest::ConnectionExt as _,
        },
    };

    struct WindowInfo {
        id: Window,
        application_name: String,
        title: String,
        bounds: ScreenRect,
    }

    fn atom(connection: &impl Connection, name: &[u8]) -> Result<x11rb::protocol::xproto::Atom> {
        Ok(connection.intern_atom(false, name)?.reply()?.atom)
    }

    fn window_text(
        connection: &impl Connection,
        window: Window,
        property: x11rb::protocol::xproto::Atom,
        property_type: x11rb::protocol::xproto::Atom,
    ) -> Result<String> {
        let value = connection
            .get_property(false, window, property, property_type, 0, 2_048)?
            .reply()?
            .value;
        Ok(String::from_utf8_lossy(&value)
            .trim_matches(char::from(0))
            .replace(char::from(0), " ")
            .trim()
            .to_string())
    }

    fn active_window() -> Result<WindowInfo> {
        let (connection, screen_index) = connect(None)?;
        let root = connection.setup().roots[screen_index].root;
        let active_window_atom = atom(&connection, b"_NET_ACTIVE_WINDOW")?;
        let window = connection
            .get_property(false, root, active_window_atom, AtomEnum::WINDOW, 0, 1)?
            .reply()?
            .value32()
            .and_then(|mut values| values.next())
            .ok_or_else(|| anyhow::anyhow!("X11 has no active window"))?;
        let geometry = connection.get_geometry(window)?.reply()?;
        let position = connection
            .translate_coordinates(window, root, 0, 0)?
            .reply()?;
        let title = window_text(
            &connection,
            window,
            atom(&connection, b"_NET_WM_NAME")?,
            atom(&connection, b"UTF8_STRING")?,
        )
        .unwrap_or_default();
        let application_name = window_text(
            &connection,
            window,
            atom(&connection, b"WM_CLASS")?,
            AtomEnum::STRING.into(),
        )
        .unwrap_or_else(|_| "Focused application".to_string());
        Ok(WindowInfo {
            id: window,
            application_name,
            title,
            bounds: ScreenRect {
                x: i32::from(position.dst_x),
                y: i32::from(position.dst_y),
                width: u32::from(geometry.width),
                height: u32::from(geometry.height),
            },
        })
    }

    async fn query_accessibility() -> Result<Option<OutputTarget>> {
        if std::env::var("XDG_SESSION_TYPE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        {
            return Ok(None);
        }
        let window = active_window().ok();
        let connection = AccessibilityConnection::new().await?;
        let root = connection.root_accessible_on_registry().await?;
        let applications = root.get_children().await?;
        let mut active_applications = Vec::new();
        for reference in &applications {
            if let Ok(accessible) = connection.object_as_accessible(reference).await
                && accessible
                    .get_state()
                    .await
                    .is_ok_and(|states| states.contains(State::Active))
            {
                active_applications.push(reference.clone());
            }
        }
        let mut queue = if active_applications.is_empty() {
            VecDeque::<ObjectRefOwned>::from(applications)
        } else {
            VecDeque::<ObjectRefOwned>::from(active_applications)
        };
        let mut visited = 0_usize;

        while let Some(reference) = queue.pop_front() {
            visited += 1;
            if visited > 20_000 {
                break;
            }
            let Ok(accessible) = connection.object_as_accessible(&reference).await else {
                continue;
            };
            if let Ok(states) = accessible.get_state().await
                && states.contains(State::Focused)
                && states.contains(State::Editable)
                && states.contains(State::Enabled)
                && !states.contains(State::ReadOnly)
                && accessible.get_role().await.ok() != Some(Role::PasswordText)
            {
                let element_name = accessible.name().await.unwrap_or_default();
                let accessible_identity = format!(
                    "{}:{}",
                    reference.name_as_str().unwrap_or_default(),
                    reference.path_as_str()
                );
                return Ok(Some(OutputTarget {
                    identity: format!(
                        "{}:{accessible_identity}",
                        window.as_ref().map(|value| value.id).unwrap_or_default()
                    ),
                    application_name: window
                        .as_ref()
                        .map(|value| value.application_name.clone())
                        .unwrap_or_else(|| "Focused application".to_string()),
                    window_title: if element_name.is_empty() {
                        window
                            .as_ref()
                            .map(|value| value.title.clone())
                            .unwrap_or_default()
                    } else {
                        element_name
                    },
                    bounds: window.as_ref().map(|value| value.bounds),
                    editable_verified: true,
                }));
            }
            if let Ok(children) = accessible.get_children().await {
                queue.extend(children);
            }
        }
        Ok(None)
    }

    fn keycode_for(connection: &impl Connection, keysym: u32) -> Result<u8> {
        let setup = connection.setup();
        let first = setup.min_keycode;
        let count = setup.max_keycode - first + 1;
        let mapping = connection.get_keyboard_mapping(first, count)?.reply()?;
        let width = usize::from(mapping.keysyms_per_keycode);
        mapping
            .keysyms
            .chunks(width)
            .position(|symbols| symbols.contains(&keysym))
            .map(|offset| first + offset as u8)
            .ok_or_else(|| anyhow::anyhow!("X11 key symbol is unavailable"))
    }

    fn send_paste() -> Result<bool> {
        const XK_CONTROL_L: u32 = 0xffe3;
        const XK_V: u32 = 0x0076;
        let (connection, _) = connect(None)?;
        let control = keycode_for(&connection, XK_CONTROL_L)?;
        let v = keycode_for(&connection, XK_V)?;
        for (event, keycode) in [
            (KEY_PRESS_EVENT, control),
            (KEY_PRESS_EVENT, v),
            (KEY_RELEASE_EVENT, v),
            (KEY_RELEASE_EVENT, control),
        ] {
            connection
                .xtest_fake_input(event, keycode, CURRENT_TIME, 0, 0, 0, 0)?
                .check()?;
        }
        connection.flush()?;
        Ok(true)
    }

    pub fn capture_target() -> Result<Option<OutputTarget>> {
        block_on(query_accessibility())
    }

    pub fn paste_if_current(target: &OutputTarget) -> Result<bool> {
        if block_on(query_accessibility())?
            .as_ref()
            .map(|value| &value.identity)
            != Some(&target.identity)
        {
            return Ok(false);
        }
        send_paste()
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
mod platform {
    use super::*;

    pub fn capture_target() -> Result<Option<OutputTarget>> {
        Ok(None)
    }

    pub fn paste_if_current(_target: &OutputTarget) -> Result<bool> {
        Ok(false)
    }
}

pub use platform::{capture_target, paste_if_current};
