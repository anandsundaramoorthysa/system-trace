//! Best-effort OS color filter toggle. Used by the collector to apply
//! grayscale during quiet hours when `bedtime_grayscale_enabled` is on.
//!
//! Per-platform behavior:
//! - **Windows**: writes the `ColorFiltering` registry keys under HKCU and
//!   broadcasts `WM_SETTINGCHANGE` so the active session picks it up. No
//!   admin elevation needed; matches what the Settings UI would do.
//! - **macOS**: shells out to `defaults write com.apple.universalaccess` and
//!   reloads the prefs. Best-effort; the user may need to sign out / in for
//!   the change to fully take. Requires no admin.
//! - **Linux**: swaps GNOME's `gtk-theme` to `HighContrast` via `gsettings`,
//!   remembering and restoring the user's previous theme so it is never
//!   clobbered. Other desktops are a no-op (returns `Ok(false)`).
//!
//! Every call returns `Ok(true)` on a successful change, `Ok(false)` when
//! the platform path is a no-op, or `Err` on a hard failure that should be
//! surfaced.

#[cfg(target_os = "windows")]
pub fn set_grayscale(on: bool) -> Result<bool, String> {
    use std::process::Command;
    // Use reg.exe to flip the well-known ColorFiltering keys. This matches
    // what flipping the toggle in Settings does. Safe (HKCU only) and does
    // not need admin.
    let active = if on { "1" } else { "0" };
    let mut a = Command::new("reg");
    a.args([
        "ADD",
        r"HKCU\Software\Microsoft\ColorFiltering",
        "/v",
        "Active",
        "/t",
        "REG_DWORD",
        "/d",
        active,
        "/f",
    ]);
    a.output().map_err(|e| e.to_string())?;
    let mut b = Command::new("reg");
    // FilterType 0 = grayscale.
    b.args([
        "ADD",
        r"HKCU\Software\Microsoft\ColorFiltering",
        "/v",
        "FilterType",
        "/t",
        "REG_DWORD",
        "/d",
        "0",
        "/f",
    ]);
    b.output().map_err(|e| e.to_string())?;
    let mut c = Command::new("reg");
    c.args([
        "ADD",
        r"HKCU\Software\Microsoft\ColorFiltering",
        "/v",
        "HotkeyEnabled",
        "/t",
        "REG_DWORD",
        "/d",
        "0",
        "/f",
    ]);
    c.output().map_err(|e| e.to_string())?;

    // The registry write alone does not nudge the running session to re-read
    // the color-filter state. Broadcast WM_SETTINGCHANGE so the shell and any
    // listeners refresh - this is the same notification the Settings toggle
    // raises. Best-effort: a failed broadcast must not fail the whole call.
    unsafe {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        };
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            100,
            None,
        );
    }
    Ok(true)
}

#[cfg(target_os = "macos")]
pub fn set_grayscale(on: bool) -> Result<bool, String> {
    use std::process::Command;
    let val = if on { "true" } else { "false" };
    Command::new("defaults")
        .args([
            "write",
            "com.apple.universalaccess",
            "grayscale",
            "-bool",
            val,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    // Reload prefs cache so apps see the change without a logout/login.
    Command::new("killall")
        .args(["cfprefsd"])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(target_os = "linux")]
const GTK_SCHEMA: &str = "org.gnome.desktop.interface";
#[cfg(target_os = "linux")]
const GTK_THEME_KEY: &str = "gtk-theme";
#[cfg(target_os = "linux")]
const GRAY_THEME: &str = "HighContrast";

#[cfg(target_os = "linux")]
pub fn set_grayscale(on: bool) -> Result<bool, String> {
    // GNOME has no built-in grayscale filter without third-party extensions, so
    // the closest cross-distro lever is swapping to the HighContrast gtk-theme.
    // To avoid clobbering whatever theme the user actually runs, we remember it
    // before switching and restore it when grayscale turns off. The previous
    // theme is persisted to a small state file so a restart between "on" and
    // "off" (e.g. across an overnight bedtime window) still restores the right
    // theme rather than resetting to a hardcoded default.
    if on {
        // Save the current theme once. Skip if grayscale is already applied, so
        // a second "on" call can never overwrite the saved original with the
        // HighContrast theme.
        if let Some(current) = current_theme() {
            if !current.is_empty() && current != GRAY_THEME {
                let _ = save_prev_theme(&current);
            }
        }
        Ok(set_theme(GRAY_THEME))
    } else {
        // Restore the saved theme; fall back to Adwaita only if nothing was
        // saved (e.g. an "off" with no prior "on").
        let prev = take_prev_theme().unwrap_or_else(|| "Adwaita".to_string());
        Ok(set_theme(&prev))
    }
}

#[cfg(target_os = "linux")]
fn set_theme(theme: &str) -> bool {
    std::process::Command::new("gsettings")
        .args(["set", GTK_SCHEMA, GTK_THEME_KEY, theme])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn current_theme() -> Option<String> {
    let out = std::process::Command::new("gsettings")
        .args(["get", GTK_SCHEMA, GTK_THEME_KEY])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // gsettings prints the value GVariant-quoted, e.g. `'Adwaita-dark'\n`.
    let s = String::from_utf8_lossy(&out.stdout);
    Some(s.trim().trim_matches('\'').to_string())
}

#[cfg(target_os = "linux")]
fn prev_theme_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    // Persist under XDG_STATE_HOME (or ~/.local/state) so the saved theme
    // survives a restart between turning grayscale on and off.
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("system-trace").join("grayscale-prev-theme")
}

#[cfg(target_os = "linux")]
fn save_prev_theme(theme: &str) -> std::io::Result<()> {
    let path = prev_theme_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, theme)
}

#[cfg(target_os = "linux")]
fn take_prev_theme() -> Option<String> {
    let path = prev_theme_path();
    let theme = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let theme = theme.trim().to_string();
    if theme.is_empty() {
        None
    } else {
        Some(theme)
    }
}
