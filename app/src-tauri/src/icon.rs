//! Best-effort real-icon extraction for an app, given its executable / bundle
//! path. Returns raw RGBA pixels (plus width/height); the frontend paints them
//! onto a canvas, so we don't pull in an image-encoder dependency. Any failure
//! returns `None` and the UI falls back to its deterministic letter avatar.

/// (width, height, rgba bytes) for the app's icon, or `None`.
pub type Rgba = (u32, u32, Vec<u8>);

#[cfg(target_os = "windows")]
pub fn extract_icon_rgba(path: &str) -> Option<Rgba> {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
    };
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut info = SHFILEINFOW::default();
        let res = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            Default::default(),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if res == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let hicon = info.hIcon;

        // Pull the color bitmap out of the icon.
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            let _ = DestroyIcon(hicon);
            return None;
        }

        let mut bm = BITMAP::default();
        let got = GetObjectW(
            icon_info.hbmColor,
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut std::ffi::c_void),
        );
        let (w, h) = (bm.bmWidth.max(0) as u32, bm.bmHeight.max(0) as u32);

        let mut out: Option<Rgba> = None;
        if got != 0 && w > 0 && h > 0 {
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    // Negative height = top-down rows (so y=0 is the top).
                    biHeight: -(h as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut buf = vec![0u8; (w * h * 4) as usize];
            let hdc: HDC = CreateCompatibleDC(None);
            let scan = GetDIBits(
                hdc,
                icon_info.hbmColor,
                0,
                h,
                Some(buf.as_mut_ptr() as *mut std::ffi::c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(hdc);
            if scan != 0 {
                // GetDIBits returns BGRA; swap to RGBA for the canvas, noting
                // whether the source actually carried an alpha channel.
                let mut any_alpha = false;
                for px in buf.chunks_exact_mut(4) {
                    px.swap(0, 2);
                    if px[3] != 0 {
                        any_alpha = true;
                    }
                }
                // Some icons (older / 24bpp sources) come back with an all-zero
                // alpha channel, which would render fully transparent. Treat
                // that as fully opaque so the icon is visible.
                if !any_alpha {
                    for px in buf.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                }
                out = Some((w, h, buf));
            }
        }

        let _ = DeleteObject(icon_info.hbmColor);
        let _ = DeleteObject(icon_info.hbmMask);
        let _ = DestroyIcon(hicon);
        out
    }
}

#[cfg(target_os = "macos")]
pub fn extract_icon_rgba(path: &str) -> Option<Rgba> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;

    const SIZE: u32 = 64;
    unsafe {
        let cpath = CString::new(path).ok()?;
        let ns_path: id = msg_send![class!(NSString), stringWithUTF8String: cpath.as_ptr()];
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil {
            return None;
        }
        let image: id = msg_send![workspace, iconForFile: ns_path];
        if image == nil {
            return None;
        }

        // Draw the NSImage into a fixed-size 32-bit RGBA bitmap rep.
        let rep: id = msg_send![class!(NSBitmapImageRep), alloc];
        // initWithBitmapDataPlanes:pixelsWide:pixelsHigh:bitsPerSample:
        //   samplesPerPixel:hasAlpha:isPlanar:colorSpaceName:bytesPerRow:bitsPerPixel:
        let ns_calibrated: id = msg_send![class!(NSString),
            stringWithUTF8String: c"NSCalibratedRGBColorSpace".as_ptr()];
        let rep: id = msg_send![rep,
            initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>()
            pixelsWide: SIZE as i64
            pixelsHigh: SIZE as i64
            bitsPerSample: 8i64
            samplesPerPixel: 4i64
            hasAlpha: true
            isPlanar: false
            colorSpaceName: ns_calibrated
            bytesPerRow: (SIZE * 4) as i64
            bitsPerPixel: 32i64];
        if rep == nil {
            return None;
        }

        let ctx_class = class!(NSGraphicsContext);
        let gctx: id = msg_send![ctx_class, graphicsContextWithBitmapImageRep: rep];
        let _: () = msg_send![ctx_class, saveGraphicsState];
        let _: () = msg_send![ctx_class, setCurrentContext: gctx];
        // NSRect { origin {0,0}, size {SIZE,SIZE} }
        let rect = NSRectF {
            x: 0.0,
            y: 0.0,
            w: SIZE as f64,
            h: SIZE as f64,
        };
        let _: () = msg_send![image, drawInRect: rect];
        let _: () = msg_send![ctx_class, restoreGraphicsState];

        let data: *const u8 = msg_send![rep, bitmapData];
        if data.is_null() {
            let _: () = msg_send![rep, release];
            return None;
        }
        let len = (SIZE * SIZE * 4) as usize;
        let buf = std::slice::from_raw_parts(data, len).to_vec();
        // Balance the +1 from alloc/init so we don't leak the bitmap rep.
        let _: () = msg_send![rep, release];
        Some((SIZE, SIZE, buf))
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct NSRectF {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[cfg(any(target_os = "linux", test))]
fn get_desktop_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    // ~/.local/share/applications
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(&home).join(".local/share/applications");
        if p.exists() {
            dirs.push(p);
        }
    }

    // XDG_DATA_HOME
    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        let p = std::path::PathBuf::from(xdg_data_home).join("applications");
        if p.exists() && !dirs.contains(&p) {
            dirs.push(p);
        }
    }

    // XDG_DATA_DIRS
    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in xdg_data_dirs.split(separator) {
            if !dir.is_empty() {
                let p = std::path::PathBuf::from(dir).join("applications");
                if p.exists() && !dirs.contains(&p) {
                    dirs.push(p);
                }
            }
        }
    } else {
        // Defaults
        for path in &["/usr/share/applications", "/usr/local/share/applications"] {
            let p = std::path::PathBuf::from(path);
            if p.exists() && !dirs.contains(&p) {
                dirs.push(p);
            }
        }
    }
    dirs
}

#[cfg(any(target_os = "linux", test))]
fn find_desktop_file(app_key: &str) -> Option<std::path::PathBuf> {
    let dirs = get_desktop_dirs();
    let app_key_lower = app_key.to_lowercase();
    let exact_name = format!("{}.desktop", app_key);
    let lower_name = format!("{}.desktop", app_key_lower);

    // Phase 1: Exact matches in all dirs
    for dir in &dirs {
        let p1 = dir.join(&exact_name);
        if p1.is_file() {
            return Some(p1);
        }
        let p2 = dir.join(&lower_name);
        if p2.is_file() {
            return Some(p2);
        }
    }

    // Phase 2: Check for prefix/suffix match (e.g. org.gnome.Nautilus.desktop or firefox-esr.desktop)
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        if let Some(stem) = filename.strip_suffix(".desktop") {
                            let stem_lower = stem.to_lowercase();
                            if stem_lower == app_key_lower
                                || stem_lower.split('.').any(|part| part == app_key_lower)
                                || stem_lower.contains(&app_key_lower)
                            {
                                return Some(path);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(any(target_os = "linux", test))]
fn get_icon_name_from_desktop(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut fallback_icon = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            in_desktop_entry = section == "Desktop Entry";
        }
        if let Some(stripped) = trimmed.strip_prefix("Icon=") {
            let value = stripped.trim().to_string();
            if !value.is_empty() {
                if in_desktop_entry {
                    return Some(value);
                } else if fallback_icon.is_none() {
                    fallback_icon = Some(value);
                }
            }
        }
    }
    fallback_icon
}

#[cfg(any(target_os = "linux", test))]
fn get_gtk_theme_name() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    for version in &["gtk-3.0", "gtk-4.0"] {
        let path = std::path::PathBuf::from(&home)
            .join(".config")
            .join(version)
            .join("settings.ini");
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("gtk-icon-theme-name") {
                    if let Some(idx) = trimmed.find('=') {
                        let val = trimmed[idx + 1..].trim();
                        if !val.is_empty() {
                            return Some(val.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(any(target_os = "linux", test))]
fn get_icon_base_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let p1 = std::path::PathBuf::from(&home).join(".local/share/icons");
        if p1.exists() {
            dirs.push(p1);
        }
        let p2 = std::path::PathBuf::from(&home).join(".icons");
        if p2.exists() {
            dirs.push(p2);
        }
    }

    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in xdg_data_dirs.split(separator) {
            if !dir.is_empty() {
                let p = std::path::PathBuf::from(dir).join("icons");
                if p.exists() && !dirs.contains(&p) {
                    dirs.push(p);
                }
            }
        }
    } else {
        let p = std::path::PathBuf::from("/usr/share/icons");
        if p.exists() && !dirs.contains(&p) {
            dirs.push(p);
        }
    }
    dirs
}

#[cfg(any(target_os = "linux", test))]
fn extract_size_from_path(path: &str) -> Option<u32> {
    for part in path.split(['/', '\\']) {
        if let Some(x_idx) = part.find('x') {
            let width_str = &part[..x_idx];
            let height_str = &part[x_idx + 1..];
            if let (Ok(w), Ok(h)) = (width_str.parse::<u32>(), height_str.parse::<u32>()) {
                if w == h {
                    return Some(w);
                }
            }
        } else if let Ok(size) = part.parse::<u32>() {
            return Some(size);
        }
    }
    None
}

#[cfg(any(target_os = "linux", test))]
fn find_icon_file_in_theme(
    theme_dir: &std::path::Path,
    icon_name: &str,
) -> Option<std::path::PathBuf> {
    let mut candidates = Vec::new();
    let icon_file_png = format!("{}.png", icon_name);
    let icon_file_png_lower = icon_file_png.to_lowercase();

    fn walk(
        dir: &std::path::Path,
        filename_png: &str,
        filename_png_lower: &str,
        candidates: &mut Vec<std::path::PathBuf>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, filename_png, filename_png_lower, candidates);
                } else if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let name_lower = name.to_lowercase();
                        if name == filename_png || name_lower == filename_png_lower {
                            candidates.push(path);
                        }
                    }
                }
            }
        }
    }

    walk(
        theme_dir,
        &icon_file_png,
        &icon_file_png_lower,
        &mut candidates,
    );

    if !candidates.is_empty() {
        candidates.sort_by(|a, b| {
            let a_str = a.to_string_lossy().to_lowercase();
            let b_str = b.to_string_lossy().to_lowercase();

            let a_has_apps = a_str.contains("apps");
            let b_has_apps = b_str.contains("apps");
            if a_has_apps != b_has_apps {
                return b_has_apps.cmp(&a_has_apps);
            }

            let a_size = extract_size_from_path(&a_str).unwrap_or(0);
            let b_size = extract_size_from_path(&b_str).unwrap_or(0);
            b_size.cmp(&a_size)
        });
        return Some(candidates[0].clone());
    }

    None
}

#[cfg(any(target_os = "linux", test))]
fn find_icon_file(icon_name: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(icon_name);
    if path.is_absolute() && path.is_file() {
        return Some(path.to_path_buf());
    }

    let base_dirs = get_icon_base_dirs();
    let active_theme = get_gtk_theme_name();

    let mut themes = Vec::new();
    if let Some(ref t) = active_theme {
        themes.push(t.as_str());
    }
    if active_theme.as_deref() != Some("hicolor") {
        themes.push("hicolor");
    }

    for theme in themes {
        for base in &base_dirs {
            let theme_dir = base.join(theme);
            if theme_dir.is_dir() {
                if let Some(resolved) = find_icon_file_in_theme(&theme_dir, icon_name) {
                    return Some(resolved);
                }
            }
        }
    }

    let pixmaps = std::path::Path::new("/usr/share/pixmaps");
    if pixmaps.is_dir() {
        let exact = pixmaps.join(format!("{}.png", icon_name));
        if exact.is_file() {
            return Some(exact);
        }
        let exact_lower = pixmaps.join(format!("{}.png", icon_name.to_lowercase()));
        if exact_lower.is_file() {
            return Some(exact_lower);
        }

        if let Ok(entries) = std::fs::read_dir(pixmaps) {
            let target = format!("{}.png", icon_name.to_lowercase());
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.to_lowercase() == target {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(any(target_os = "linux", test))]
pub fn resolve_linux_icon(app_key: &str) -> Option<String> {
    let desktop_path = find_desktop_file(app_key)?;
    let icon_name = get_icon_name_from_desktop(&desktop_path)?;
    let icon_path = find_icon_file(&icon_name)?;
    Some(icon_path.to_string_lossy().into_owned())
}

#[cfg(target_os = "linux")]
fn load_png_rgba(path: &std::path::Path) -> Option<Rgba> {
    let img = image::open(path).ok()?;
    let img = img.to_rgba8();
    let (width, height) = img.dimensions();
    Some((width, height, img.into_raw()))
}

#[cfg(target_os = "linux")]
pub fn extract_icon_rgba(path: &str) -> Option<Rgba> {
    if path == "none" {
        return None;
    }
    let p = std::path::Path::new(path);
    if p.is_file() {
        load_png_rgba(p)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_linux_icon_resolution_logic() {
        let temp_dir = std::env::temp_dir().join("system_trace_test_icons");
        let _ = fs::remove_dir_all(&temp_dir);

        let apps_dir = temp_dir.join("applications");
        let hicolor_apps_dir = temp_dir.join("icons/hicolor/48x48/apps");
        let pixmaps_dir = temp_dir.join("pixmaps");

        fs::create_dir_all(&apps_dir).unwrap();
        fs::create_dir_all(&hicolor_apps_dir).unwrap();
        fs::create_dir_all(&pixmaps_dir).unwrap();

        let desktop_file_path = apps_dir.join("mock-app.desktop");
        let desktop_content =
            "[Desktop Entry]\nName=Mock App\nExec=mock-app\nIcon=mock-icon-name\n";
        fs::write(&desktop_file_path, desktop_content).unwrap();

        let icon_png_path = hicolor_apps_dir.join("mock-icon-name.png");
        fs::write(&icon_png_path, b"mock-png-data").unwrap();

        let old_xdg_dirs = std::env::var("XDG_DATA_DIRS").ok();
        let old_xdg_home = std::env::var("XDG_DATA_HOME").ok();
        let old_home = std::env::var("HOME").ok();

        std::env::set_var("XDG_DATA_DIRS", temp_dir.to_str().unwrap());
        std::env::set_var("XDG_DATA_HOME", temp_dir.to_str().unwrap());
        std::env::set_var("HOME", temp_dir.to_str().unwrap());

        let resolved_desktop = find_desktop_file("mock-app");
        assert_eq!(resolved_desktop, Some(desktop_file_path.clone()));

        let icon_name = get_icon_name_from_desktop(&desktop_file_path);
        assert_eq!(icon_name, Some("mock-icon-name".to_string()));

        assert_eq!(
            extract_size_from_path("/usr/share/icons/hicolor/128x128/apps/icon.png"),
            Some(128)
        );
        assert_eq!(
            extract_size_from_path("icons/hicolor/256/apps/icon.png"),
            Some(256)
        );
        assert_eq!(extract_size_from_path("flat/icon.png"), None);

        let resolved_icon = find_icon_file("mock-icon-name");
        assert_eq!(resolved_icon, Some(icon_png_path));

        let final_path = resolve_linux_icon("mock-app");
        assert!(final_path.is_some());
        assert!(final_path.unwrap().ends_with("mock-icon-name.png"));

        if let Some(val) = old_xdg_dirs {
            std::env::set_var("XDG_DATA_DIRS", val);
        } else {
            std::env::remove_var("XDG_DATA_DIRS");
        }
        if let Some(val) = old_xdg_home {
            std::env::set_var("XDG_DATA_HOME", val);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
        if let Some(val) = old_home {
            std::env::set_var("HOME", val);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
