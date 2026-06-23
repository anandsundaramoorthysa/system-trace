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

pub struct LinuxTerminator;

impl LinuxTerminator {
    pub fn new() -> Self {
        LinuxTerminator
    }
}

impl Default for LinuxTerminator {
    fn default() -> Self {
        Self::new()
    }
}

impl super::ProcessTerminator for LinuxTerminator {
    fn terminate_process(&self, pid: u32) -> Result<(), super::TerminateError> {
        use nix::errno::Errno;
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let pid_t = Pid::from_raw(pid as i32);

        // 1. Send SIGTERM first.
        match kill(pid_t, Signal::SIGTERM) {
            Ok(_) => {
                // 2. Spawn a background thread to wait and escalate to SIGKILL if still alive.
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    // Check if process still exists
                    if kill(pid_t, None).is_ok() {
                        let _ = kill(pid_t, Signal::SIGKILL);
                    }
                });
                Ok(())
            }
            Err(Errno::ESRCH) => Err(super::TerminateError::NoSuchProcess),
            Err(Errno::EPERM) => Err(super::TerminateError::PermissionDenied),
            Err(e) => Err(super::TerminateError::Other(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ProcessTerminator;
    use std::process::{Command, Stdio};

    #[test]
    fn test_linux_terminator_success() {
        // Spawn a dummy process (e.g., sleep 10)
        let mut child = Command::new("sleep")
            .arg("10")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn sleep process");

        let pid = child.id();
        let terminator = LinuxTerminator::new();

        // Terminate the process
        let res = terminator.terminate_process(pid);
        assert!(res.is_ok());

        // Wait for the child to exit and verify it was terminated
        let status = child.wait().expect("Failed to wait on child");
        assert!(!status.success()); // Should be killed by signal (not success exit)
    }

    #[test]
    fn test_linux_terminator_no_such_process() {
        let terminator = LinuxTerminator::new();
        // Use a highly likely unused PID, e.g. 999999
        let res = terminator.terminate_process(999999);
        assert_eq!(res, Err(crate::platform::TerminateError::NoSuchProcess));
    }
}
