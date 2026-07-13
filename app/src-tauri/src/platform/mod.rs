//! Platform abstraction (SYSTEM_DESIGN.md section 5).
//!
//! The collector core is OS-agnostic: it only knows the `Watcher` trait. Each OS
//! provides one concrete implementation behind a `#[cfg(target_os = ...)]` gate.
//! Title and media/lock detection are best-effort; an OS that cannot provide them
//! returns `None`/`false` and the app still works (app-level tracking).

/// The currently focused window, reduced to what the core needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWindow {
    /// Stable id: exe file stem on Windows, bundle id on macOS, WM_CLASS on Linux.
    pub app_key: String,
    /// Friendly name for display (falls back to `app_key`).
    pub app_name: String,
    /// Window title; only populated when the user enabled title capture.
    pub title: Option<String>,
    /// On-disk path of the executable / app bundle, when known. Stored so the
    /// UI can extract a real OS icon for the app. `None` when not resolvable
    /// (e.g. Linux, where there's no reliable per-window path).
    pub app_path: Option<String>,
    /// The process identifier (PID) of the active window, when known.
    pub pid: Option<u32>,
}

/// The only platform-specific surface. Implementations must be cheap to call once
/// per second and must never panic on transient OS failures (return None instead).
pub trait Watcher: Send {
    /// The foreground window, or `None` when it cannot be determined.
    fn active_window(&mut self) -> Option<ActiveWindow>;
    /// Milliseconds since the last keyboard/mouse input.
    fn idle_ms(&mut self) -> u64;
    /// Best-effort: is audio/video playing? `false` when unknown (MVP default).
    fn is_media_playing(&mut self) -> bool {
        false
    }
    /// Best-effort: is the session locked/asleep? `false` when unknown (MVP default).
    fn session_locked(&mut self) -> bool {
        false
    }
    /// Tell the watcher whether the user enabled window-title capture, so a
    /// watcher whose title lookup is expensive (e.g. the macOS Accessibility
    /// API) can skip it when off. Default: ignore (cheap title lookups).
    fn set_capture_titles(&mut self, _on: bool) {}
    /// Best-effort: did the system just resume from sleep/suspend?
    /// Detected by a large monotonic-clock jump between ticks.
    /// `false` when unknown (MVP default).
    fn session_suspended(&mut self) -> bool {
        false
    }
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WinWatcher as PlatformWatcher;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacWatcher as PlatformWatcher;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod linux_wayland;
#[cfg(target_os = "linux")]
mod linux_x11;
#[cfg(target_os = "linux")]
pub use linux::LinuxWatcher as PlatformWatcher;

/// Construct the watcher for the current OS.
pub fn make_watcher() -> Box<dyn Watcher> {
    Box::new(PlatformWatcher::new())
}

/// Reposition the window on the active monitor that currently has focus or holds the cursor.
#[cfg(target_os = "windows")]
pub fn position_window_on_active_monitor(window: &tauri::WebviewWindow) {
    use ::windows::Win32::Foundation::POINT;
    use ::windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use ::windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

    unsafe {
        let mut hmonitor = None;

        // 1. Try to get monitor from the foreground window
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            let hm = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if !hm.0.is_null() {
                hmonitor = Some(hm);
            }
        }

        // 2. If that failed, try to get monitor from the cursor position
        if hmonitor.is_none() {
            let mut pt = POINT::default();
            if GetCursorPos(&mut pt).is_ok() {
                let hm = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
                if !hm.0.is_null() {
                    hmonitor = Some(hm);
                }
            }
        }

        // 3. If we have a monitor, get its working area and center our window on it
        if let Some(hm) = hmonitor {
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hm, &mut info).as_bool() {
                let rect = info.rcWork;
                let monitor_width = rect.right - rect.left;
                let monitor_height = rect.bottom - rect.top;

                if let Ok(size) = window.outer_size() {
                    let win_width = size.width as i32;
                    let win_height = size.height as i32;

                    let x = rect.left + (monitor_width - win_width) / 2;
                    let y = rect.top + (monitor_height - win_height) / 2;

                    let _ = window
                        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                }
            }
        }
    }
}

/// Fallback for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn position_window_on_active_monitor(_window: &tauri::WebviewWindow) {
    // Non-windows fallback is a no-op (the window will just open on its default monitor)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminateError {
    NoSuchProcess,
    PermissionDenied,
    Other(String),
}

impl std::fmt::Display for TerminateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminateError::NoSuchProcess => write!(f, "Process not found (already dead)"),
            TerminateError::PermissionDenied => write!(f, "Permission denied"),
            TerminateError::Other(s) => write!(f, "{}", s),
        }
    }
}

pub trait ProcessTerminator: Send + Sync {
    fn terminate_process(&self, pid: u32) -> Result<(), TerminateError>;
}

#[cfg(target_os = "windows")]
pub use windows::WinTerminator as PlatformTerminator;

#[cfg(target_os = "linux")]
pub use linux::LinuxTerminator as PlatformTerminator;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub struct PlatformTerminator;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
impl PlatformTerminator {
    pub fn new() -> Self {
        PlatformTerminator
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
impl Default for PlatformTerminator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
impl ProcessTerminator for PlatformTerminator {
    fn terminate_process(&self, _pid: u32) -> Result<(), TerminateError> {
        Err(TerminateError::Other(
            "Not implemented on this platform".to_string(),
        ))
    }
}

pub fn make_terminator() -> Box<dyn ProcessTerminator> {
    Box::new(PlatformTerminator::new())
}
