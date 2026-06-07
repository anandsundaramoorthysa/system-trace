//! Windows watcher (Win32). SYSTEM_DESIGN.md section 5.
//!
//! - active window: GetForegroundWindow -> process exe (app_key) + GetWindowText (title)
//! - idle: GetLastInputInfo vs GetTickCount
//! - media / locked: best-effort, return defaults in the MVP (documented TODO)

#![cfg(target_os = "windows")]

use super::{ActiveWindow, Watcher};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

#[derive(Default)]
pub struct WinWatcher;

impl WinWatcher {
    pub fn new() -> Self {
        WinWatcher
    }
}

impl Watcher for WinWatcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            // Resolve the owning process id.
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
            if pid == 0 {
                return None;
            }

            let (app_key, app_name) =
                process_name(pid).unwrap_or_else(|| ("unknown".to_string(), "Unknown".to_string()));

            let title = window_title(hwnd);

            Some(ActiveWindow {
                app_key,
                app_name,
                title,
            })
        }
    }

    fn idle_ms(&mut self) -> u64 {
        unsafe {
            let mut info = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if GetLastInputInfo(&mut info).as_bool() {
                let now = GetTickCount();
                // Both are milliseconds since boot; handle the rare wraparound.
                now.wrapping_sub(info.dwTime) as u64
            } else {
                0
            }
        }
    }

    // is_media_playing / session_locked use the trait defaults (false) in the MVP.
}

/// Read the foreground window title. Returns `None` when empty.
unsafe fn window_title(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buf);
    if len <= 0 {
        return None;
    }
    let s = String::from_utf16_lossy(&buf[..len as usize]);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Resolve a process id to (app_key, app_name) using its executable path.
/// app_key is the lowercased exe filename (e.g. "chrome.exe"); app_name is the
/// file stem (e.g. "chrome").
unsafe fn process_name(pid: u32) -> Option<(String, String)> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

    let mut buf = [0u16; MAX_PATH as usize];
    let mut size = buf.len() as u32;
    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_WIN32,
        PWSTR(buf.as_mut_ptr()),
        &mut size,
    );
    let _ = CloseHandle(handle);
    result.ok()?;

    let full = String::from_utf16_lossy(&buf[..size as usize]);
    let path = std::path::Path::new(&full);
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.clone());

    Some((file_name.to_lowercase(), stem))
}
