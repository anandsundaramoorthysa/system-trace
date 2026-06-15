//! Linux watcher (X11). SYSTEM_DESIGN.md section 5.
//!
//! Active window via the EWMH `_NET_ACTIVE_WINDOW` hint, with `WM_CLASS` as the
//! app_key and `_NET_WM_NAME` as the title. Idle via the X SCREENSAVER extension
//! (`ms_since_user_input`).

#![cfg(target_os = "linux")]

use super::{ActiveWindow, Watcher};
use x11rb::connection::Connection;
use x11rb::protocol::screensaver::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

pub struct X11Watcher {
    conn: Option<(RustConnection, Window)>,
    /// Whether the X SCREENSAVER extension answered our probe at startup. When
    /// false we have no idle signal at all.
    idle_supported: bool,
}

impl Default for X11Watcher {
    fn default() -> Self {
        Self::new()
    }
}

impl X11Watcher {
    pub fn new() -> Self {
        let conn = x11rb::connect(None).ok().map(|(c, screen)| {
            let root = c.setup().roots[screen].root;
            (c, root)
        });
        // Probe the SCREENSAVER extension once so idle_ms can tell "no idle
        // signal available" apart from "user just had input".
        let idle_supported = conn
            .as_ref()
            .map(|(c, root)| {
                c.screensaver_query_info(*root)
                    .ok()
                    .and_then(|cookie| cookie.reply().ok())
                    .is_some()
            })
            .unwrap_or(false);
        X11Watcher {
            conn,
            idle_supported,
        }
    }

    fn atom(&self, name: &[u8]) -> Option<u32> {
        let (c, _) = self.conn.as_ref()?;
        c.intern_atom(false, name)
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    }

    fn query_active(&self) -> Option<ActiveWindow> {
        let (c, root) = self.conn.as_ref()?;
        let net_active = self.atom(b"_NET_ACTIVE_WINDOW")?;
        let prop = c
            .get_property(false, *root, net_active, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;
        let win = prop.value32()?.next()? as Window;
        if win == 0 {
            return None;
        }

        // WM_CLASS is "instance\0class\0"; prefer the class as the stable key.
        let class = c
            .get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
            .ok()?
            .reply()
            .ok()?;
        let parts: Vec<&[u8]> = class
            .value
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .collect();
        let app_key = parts
            .get(1)
            .or_else(|| parts.first())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        // Title via _NET_WM_NAME (UTF8_STRING); best-effort.
        let title = match (self.atom(b"_NET_WM_NAME"), self.atom(b"UTF8_STRING")) {
            (Some(name_atom), Some(utf8)) => c
                .get_property(false, win, name_atom, utf8, 0, 1024)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|r| String::from_utf8_lossy(&r.value).into_owned())
                .filter(|s| !s.is_empty()),
            _ => None,
        };

        Some(ActiveWindow {
            app_name: app_key.clone(),
            app_key,
            title,
            // No reliable per-window executable path on X11/Wayland.
            app_path: None,
        })
    }
}

impl Watcher for X11Watcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
        self.query_active()
    }

    fn idle_ms(&mut self) -> u64 {
        // When we have no idle signal - no X connection (e.g. Wayland) or an X
        // server without the SCREENSAVER extension - report "fully idle"
        // (u64::MAX) rather than 0. Returning 0 means "the user just had input",
        // which would count the foreground app as active around the clock even
        // while the user is away. Reporting idle instead makes the collector
        // stop accumulating, which is the safe direction.
        let Some((c, root)) = self.conn.as_ref() else {
            return u64::MAX;
        };
        if !self.idle_supported {
            return u64::MAX;
        }
        c.screensaver_query_info(*root)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|r| r.ms_since_user_input as u64)
            .unwrap_or(u64::MAX)
    }

    fn is_media_playing(&mut self) -> bool {
        // Best-effort and dependency-free: ALSA exposes a per-substream status
        // file under /proc/asound. A playback substream reads "RUNNING" while
        // audio is actually flowing, so we scan the playback ("pcm*p") streams.
        // This catches PipeWire/Pulse output too, since they sit on top of ALSA.
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
                // Playback devices are named like "pcm0p".
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
}
