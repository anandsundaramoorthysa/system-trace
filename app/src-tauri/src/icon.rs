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
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, ReleaseDC, BITMAP,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
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
                // GetDIBits returns BGRA; swap to RGBA for the canvas.
                for px in buf.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }
                out = Some((w, h, buf));
            }
        }

        let _ = DeleteObject(icon_info.hbmColor);
        let _ = DeleteObject(icon_info.hbmMask);
        let _ = DestroyIcon(hicon);
        let _ = ReleaseDC(None, HDC::default());
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
            return None;
        }
        let len = (SIZE * SIZE * 4) as usize;
        let buf = std::slice::from_raw_parts(data, len).to_vec();
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

#[cfg(target_os = "linux")]
pub fn extract_icon_rgba(_path: &str) -> Option<Rgba> {
    // Desktop icon-theme resolution is distro/DE-specific; the UI falls back to
    // a letter avatar here. (A future enhancement could parse .desktop files.)
    None
}
