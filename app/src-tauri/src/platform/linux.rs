//! Linux watcher (X11 & Wayland). SYSTEM_DESIGN.md section 5.
//!
//! Route active window and idle monitoring to the X11 or Wayland tracking systems.

#![cfg(target_os = "linux")]

use super::linux_wayland::WaylandWatcher;
use super::linux_x11::X11Watcher;
use super::{ActiveWindow, Watcher};

pub struct LinuxWatcher {
    inner: WatcherImpl,
}

enum WatcherImpl {
    X11(Box<X11Watcher>),
    Wayland(Box<WaylandWatcher>),
}

impl Default for LinuxWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxWatcher {
    pub fn new() -> Self {
        let is_wayland = detect_session_type() == "wayland";
        let inner = if is_wayland {
            log::info!("Wayland session detected, starting WaylandWatcher");
            WatcherImpl::Wayland(Box::default())
        } else {
            log::info!("X11 session detected, starting X11Watcher");
            WatcherImpl::X11(Box::default())
        };
        LinuxWatcher { inner }
    }
}

impl Watcher for LinuxWatcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
        match &mut self.inner {
            WatcherImpl::X11(w) => w.active_window(),
            WatcherImpl::Wayland(w) => w.active_window(),
        }
    }

    fn idle_ms(&mut self) -> u64 {
        match &mut self.inner {
            WatcherImpl::X11(w) => w.idle_ms(),
            WatcherImpl::Wayland(w) => w.idle_ms(),
        }
    }

    fn is_media_playing(&mut self) -> bool {
        match &mut self.inner {
            WatcherImpl::X11(w) => w.is_media_playing(),
            WatcherImpl::Wayland(w) => w.is_media_playing(),
        }
    }

    fn session_locked(&mut self) -> bool {
        match &mut self.inner {
            WatcherImpl::X11(w) => w.session_locked(),
            WatcherImpl::Wayland(w) => w.session_locked(),
        }
    }

    fn set_capture_titles(&mut self, on: bool) {
        match &mut self.inner {
            WatcherImpl::X11(w) => w.set_capture_titles(on),
            WatcherImpl::Wayland(w) => w.set_capture_titles(on),
        }
    }
}

/// Helper function to detect session type at runtime
fn detect_session_type() -> &'static str {
    if let Ok(val) = std::env::var("XDG_SESSION_TYPE") {
        let val_lower = val.to_lowercase();
        if val_lower == "wayland" {
            return "wayland";
        }
        if val_lower == "x11" {
            return "x11";
        }
    }
    // Fallbacks
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        "wayland"
    } else if std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        "unknown"
    }
}

/// Shared ALSA implementation for media playback monitoring
pub(crate) fn is_alsa_media_playing() -> bool {
    use std::fs;
    let Ok(cards) = fs::read_dir("/proc/asound") else {
        return false;
    };
    for card in cards.flatten() {
        let Ok(pcms) = fs::read_dir(card.path()) else {
            continue;
        };
        for pcm in pcms.flatten() {
            let name = pcm.file_name();
            let name = name.to_string_lossy();
            if !(name.starts_with("pcm") && name.ends_with('p')) {
                continue;
            }
            let Ok(subs) = fs::read_dir(pcm.path()) else {
                continue;
            };
            for sub in subs.flatten() {
                if let Ok(s) = fs::read_to_string(sub.path().join("status")) {
                    if s.contains("RUNNING") {
                        return true;
                    }
                }
            }
        }
    }
    false
}
