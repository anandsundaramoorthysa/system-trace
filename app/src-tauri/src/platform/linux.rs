//! Linux watcher (X11). SYSTEM_DESIGN.md section 5.
//!
//! Active window via the EWMH `_NET_ACTIVE_WINDOW` hint, with `WM_CLASS` as the
//! app_key and `_NET_WM_NAME` as the title. Idle via the X SCREENSAVER extension
//! (`ms_since_user_input`). Wayland is not covered yet; under Wayland the EWMH
//! query returns nothing and the app degrades to no tracking until the portal
//! path lands (see SYSTEM_DESIGN.md section 16).
//!
//! NOTE: this file is `#[cfg(target_os = "linux")]` and is therefore compiled and
//! verified by the Linux CI job, not on a Windows dev box.

#![cfg(target_os = "linux")]

use super::{ActiveWindow, Watcher};
use x11rb::connection::Connection;
use x11rb::protocol::screensaver::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

pub struct LinuxWatcher {
    conn: Option<(RustConnection, Window)>,
}

impl Default for LinuxWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxWatcher {
    pub fn new() -> Self {
        let conn = x11rb::connect(None).ok().map(|(c, screen)| {
            let root = c.setup().roots[screen].root;
            (c, root)
        });
        LinuxWatcher { conn }
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
        })
    }
}

impl Watcher for LinuxWatcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
        self.query_active()
    }

    fn idle_ms(&mut self) -> u64 {
        let Some((c, root)) = self.conn.as_ref() else {
            return 0;
        };
        c.screensaver_query_info(*root)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|r| r.ms_since_user_input as u64)
            .unwrap_or(0)
    }
}
