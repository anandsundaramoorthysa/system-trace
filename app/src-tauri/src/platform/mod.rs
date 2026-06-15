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
mod linux_x11;
#[cfg(target_os = "linux")]
mod linux_wayland;
#[cfg(target_os = "linux")]
pub use linux::LinuxWatcher as PlatformWatcher;

/// Construct the watcher for the current OS.
pub fn make_watcher() -> Box<dyn Watcher> {
    Box::new(PlatformWatcher::new())
}
