//! Linux watcher (Wayland). SYSTEM_DESIGN.md section 5.
//!
//! Handles Wayland sessions under GNOME (via D-Bus Focused Window extension or Eval)
//! and KDE Plasma (via transient KWin scripting calling a D-Bus receiver).

#![cfg(target_os = "linux")]

use super::{ActiveWindow, Watcher};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use zbus::blocking::Proxy;

pub trait WaylandWindowFetcher: Send {
    fn fetch_active_window(&mut self) -> Option<ActiveWindow>;
}

pub struct NullWaylandWindowFetcher;

impl WaylandWindowFetcher for NullWaylandWindowFetcher {
    fn fetch_active_window(&mut self) -> Option<ActiveWindow> {
        None
    }
}

#[derive(Deserialize, Debug)]
struct GnomeWindowInfo {
    title: Option<String>,
    #[serde(alias = "class")]
    wm_class: Option<String>,
    pid: Option<u32>,
}

pub struct DbusWaylandWindowFetcher {
    conn: Option<zbus::blocking::Connection>,
    desktop: String,
    kde_active: Arc<Mutex<Option<ActiveWindow>>>,
}

impl WaylandWindowFetcher for DbusWaylandWindowFetcher {
    fn fetch_active_window(&mut self) -> Option<ActiveWindow> {
        let conn = self.conn.as_ref()?;
        match self.desktop.as_str() {
            "gnome" => {
                // Method A: Try Focused Window D-Bus extension
                if let Ok(proxy) = Proxy::new(
                    conn,
                    "org.gnome.Shell",
                    "/org/gnome/shell/extensions/FocusedWindow",
                    "org.gnome.shell.extensions.FocusedWindow",
                ) {
                    let res: Result<String, zbus::Error> = proxy.call("Get", &());
                    if let Ok(json_str) = res {
                        if let Ok(info) = serde_json::from_str::<GnomeWindowInfo>(&json_str) {
                            if let Some(app_key) = info.wm_class {
                                return Some(ActiveWindow {
                                    app_name: app_key.clone(),
                                    app_key,
                                    title: info.title,
                                    app_path: None,
                                    pid: info.pid,
                                });
                            }
                        }
                    }
                }

                // Method B: Fall back to org.gnome.Shell.Eval (unsafe mode must be enabled in newer GNOME)
                if let Ok(proxy) = Proxy::new(
                    conn,
                    "org.gnome.Shell",
                    "/org/gnome/Shell",
                    "org.gnome.Shell",
                ) {
                    let script = "let win = global.display.get_focus_window(); win ? JSON.stringify({class: win.get_wm_class(), title: win.get_title(), pid: win.get_pid ? win.get_pid() : null}) : 'null'";
                    let res: Result<(bool, String), zbus::Error> = proxy.call("Eval", &(script,));
                    if let Ok((success, result)) = res {
                        if success && result != "null" {
                            if let Ok(info) = serde_json::from_str::<GnomeWindowInfo>(&result) {
                                if let Some(app_key) = info.wm_class {
                                    return Some(ActiveWindow {
                                        app_name: app_key.clone(),
                                        app_key,
                                        title: info.title,
                                        app_path: None,
                                        pid: info.pid,
                                    });
                                }
                            }
                        }
                    }
                }
                None
            }
            "kde" => {
                // KDE: read from the shared State updated by the KWin script callback
                self.kde_active.lock().ok().and_then(|guard| guard.clone())
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ActiveWindowReceiver {
    active: Arc<Mutex<Option<ActiveWindow>>>,
}

#[zbus::interface(name = "org.system_trace.Receiver")]
impl ActiveWindowReceiver {
    #[zbus(name = "UpdateActiveWindow")]
    fn update_active_window(&self, app_key: String, title: String, pid: u32) {
        if let Ok(mut active) = self.active.lock() {
            *active = Some(ActiveWindow {
                app_name: app_key.clone(),
                app_key,
                title: if title.is_empty() { None } else { Some(title) },
                app_path: None,
                pid: if pid == 0 { None } else { Some(pid) },
            });
        }
    }
}

pub struct WaylandWatcher {
    conn: Option<zbus::blocking::Connection>,
    fetcher: Box<dyn WaylandWindowFetcher>,
    desktop: String,
    capture_titles: bool,
}

impl Default for WaylandWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl WaylandWatcher {
    pub fn new() -> Self {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();

        let mut conn_opt = None;
        let mut fetcher: Box<dyn WaylandWindowFetcher> = Box::new(NullWaylandWindowFetcher);
        let kde_active = Arc::new(Mutex::new(None));

        if desktop.contains("gnome") || desktop.contains("kde") {
            let conn_res = if desktop.contains("kde") {
                let receiver = ActiveWindowReceiver {
                    active: kde_active.clone(),
                };
                zbus::blocking::connection::Builder::session().and_then(|b| {
                    b.name("org.system_trace.Receiver")?
                        .serve_at("/org/system_trace/Receiver", receiver)?
                        .build()
                })
            } else {
                zbus::blocking::Connection::session()
            };

            match conn_res {
                Ok(conn) => {
                    log::info!("WaylandWatcher connected to D-Bus session bus");
                    if desktop.contains("kde") {
                        log::info!("KDE Plasma detected, setting up KWin script");
                        if let Err(e) = setup_kwin_script(&conn) {
                            log::error!("Failed to setup KWin script: {:?}", e);
                        }
                    }

                    fetcher = Box::new(DbusWaylandWindowFetcher {
                        conn: Some(conn.clone()),
                        desktop: if desktop.contains("kde") {
                            "kde".to_string()
                        } else {
                            "gnome".to_string()
                        },
                        kde_active,
                    });
                    conn_opt = Some(conn);
                }
                Err(e) => {
                    log::error!("Failed to connect to D-Bus session bus: {:?}", e);
                }
            }
        }

        WaylandWatcher {
            conn: conn_opt,
            fetcher,
            desktop: if desktop.contains("kde") {
                "kde".to_string()
            } else if desktop.contains("gnome") {
                "gnome".to_string()
            } else {
                "other".to_string()
            },
            capture_titles: true,
        }
    }
}

fn setup_kwin_script(conn: &zbus::blocking::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let scripting_proxy = Proxy::new(conn, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting")?;

    // Unload any previously loaded script of the same name to prevent naming conflicts
    let _: Result<(), _> = scripting_proxy.call("unloadScript", &("system_trace_watcher",));

    let script_content = r#"
        function sendActive(window) {
            if (window) {
                callDBus(
                    "org.system_trace.Receiver",
                    "/org/system_trace/Receiver",
                    "org.system_trace.Receiver",
                    "UpdateActiveWindow",
                    window.resourceClass.toString(),
                    window.caption,
                    window.pid ? window.pid : 0
                );
            }
        }

        if (typeof workspace.windowActivated !== 'undefined') {
            workspace.windowActivated.connect(sendActive);
        } else if (typeof workspace.clientActivated !== 'undefined') {
            workspace.clientActivated.connect(sendActive);
        }

        sendActive(workspace.activeWindow);
    "#;

    let temp_file = std::env::temp_dir().join("system-trace-kwin-script.js");
    std::fs::write(&temp_file, script_content)?;

    let script_path_str = temp_file.to_string_lossy().to_string();
    let script_obj_path: zbus::zvariant::OwnedObjectPath =
        scripting_proxy.call("loadScript", &(script_path_str, "system_trace_watcher"))?;

    let script_proxy = Proxy::new(conn, "org.kde.KWin", script_obj_path, "org.kde.kwin.Script")?;
    let _: () = script_proxy.call("run", &())?;

    let _ = std::fs::remove_file(temp_file);
    Ok(())
}

fn unload_kwin_script(conn: &zbus::blocking::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let scripting_proxy = Proxy::new(conn, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting")?;
    let _: Result<(), _> = scripting_proxy.call("unloadScript", &("system_trace_watcher",));
    Ok(())
}

impl Watcher for WaylandWatcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
        let mut win = self.fetcher.fetch_active_window()?;
        if !self.capture_titles {
            win.title = None;
        }
        Some(win)
    }

    fn idle_ms(&mut self) -> u64 {
        if self.desktop == "gnome" {
            if let Some(conn) = &self.conn {
                if let Ok(proxy) = Proxy::new(
                    conn,
                    "org.gnome.Mutter.IdleMonitor",
                    "/org/gnome/Mutter/IdleMonitor/Core",
                    "org.gnome.Mutter.IdleMonitor",
                ) {
                    let res: Result<u64, zbus::Error> = proxy.call("GetIdletime", &());
                    if let Ok(idle) = res {
                        return idle;
                    }
                }
            }
        }
        // Under KDE and other desktop environments, we lack a stable public D-Bus idle monitor.
        // Returning u64::MAX counts the user as idle so that the collector stops accumulating when away.
        u64::MAX
    }

    fn is_media_playing(&mut self) -> bool {
        super::linux::is_alsa_media_playing()
    }

    fn session_locked(&mut self) -> bool {
        if let Some(conn) = &self.conn {
            if let Ok(proxy) = Proxy::new(
                conn,
                "org.freedesktop.ScreenSaver",
                "/org/freedesktop/ScreenSaver",
                "org.freedesktop.ScreenSaver",
            ) {
                let res: Result<bool, zbus::Error> = proxy.call("GetActive", &());
                if let Ok(locked) = res {
                    return locked;
                }
            }
        }
        false
    }

    fn set_capture_titles(&mut self, on: bool) {
        self.capture_titles = on;
    }
}

impl Drop for WaylandWatcher {
    fn drop(&mut self) {
        if self.desktop == "kde" {
            if let Some(conn) = &self.conn {
                log::info!("WaylandWatcher drop: unloading KWin script");
                if let Err(e) = unload_kwin_script(conn) {
                    log::error!("Failed to unload KWin script on drop: {:?}", e);
                }
            }
        }
    }
}

#[cfg(test)]
pub struct FakeWaylandWindowFetcher {
    pub active: Option<ActiveWindow>,
}

#[cfg(test)]
impl WaylandWindowFetcher for FakeWaylandWindowFetcher {
    fn fetch_active_window(&mut self) -> Option<ActiveWindow> {
        self.active.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_watcher_fake() {
        let fake_fetcher = Box::new(FakeWaylandWindowFetcher {
            active: Some(ActiveWindow {
                app_key: "firefox".to_string(),
                app_name: "Firefox".to_string(),
                title: Some("GitHub".to_string()),
                app_path: None,
                pid: Some(1234),
            }),
        });

        let mut watcher = WaylandWatcher {
            conn: None,
            fetcher: fake_fetcher,
            desktop: "gnome".to_string(),
            capture_titles: true,
        };

        let active = watcher.active_window().unwrap();
        assert_eq!(active.app_key, "firefox");
        assert_eq!(active.app_name, "Firefox");
        assert_eq!(active.title, Some("GitHub".to_string()));
        assert_eq!(active.pid, Some(1234));

        // Test title capture gating
        watcher.set_capture_titles(false);
        let active_no_title = watcher.active_window().unwrap();
        assert_eq!(active_no_title.title, None);
    }
}
