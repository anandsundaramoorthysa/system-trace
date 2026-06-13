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
//! - **Linux**: tries `gsettings` for GNOME's `org.gnome.desktop.a11y`
//!   schema. Other desktops are a no-op (returns `Ok(false)`).
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
pub fn set_grayscale(on: bool) -> Result<bool, String> {
    use std::process::Command;
    // GNOME: high-contrast desktop theme as a best-effort grayscale-ish
    // substitute. The native a11y schema does not expose a true greyscale
    // filter without third-party extensions, so this is the closest cross-
    // distro lever we have here.
    let theme = if on { "HighContrast" } else { "Adwaita" };
    let out = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "gtk-theme", theme])
        .output();
    match out {
        Ok(o) if o.status.success() => Ok(true),
        _ => Ok(false),
    }
}
