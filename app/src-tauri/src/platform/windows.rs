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

pub struct WinWatcher {
    last_tick: Option<(std::time::Instant, u64)>,
}
impl WinWatcher {
    pub fn new() -> Self {
        // Initialize COM on this (collector) thread so the WASAPI peak-meter
        // query in `is_media_playing` can create the device enumerator. Safe to
        // call repeatedly; an already-initialized thread just returns a
        // non-fatal code we ignore.
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        WinWatcher { last_tick: None }
    }
}

impl Default for WinWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Current output peak level (0.0..=1.0) of the default render endpoint, or
/// `None` if it can't be read. A non-trivial peak means audio is actually
/// coming out of the speakers right now (music, a video with sound, a call).
unsafe fn render_peak() -> Option<f32> {
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
    let meter: IAudioMeterInformation = device.Activate(CLSCTX_ALL, None).ok()?;
    meter.GetPeakValue().ok()
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

            // If we can't resolve the owning process (a protected/elevated
            // system surrogate like the UAC prompt or lock screen), skip this
            // sample entirely (`?` returns None) rather than attributing the
            // time to a synthetic "Unknown" app - otherwise every unresolvable
            // window across different real processes collapses into one bogus
            // usage row.
            let (app_key, app_name, app_path) = process_name(pid)?;

            let title = window_title(hwnd);

            Some(ActiveWindow {
                app_key,
                app_name,
                title,
                app_path: Some(app_path),
                pid: Some(pid),
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

    fn is_media_playing(&mut self) -> bool {
        // Audio coming out of the default output device implies the user is
        // actively consuming media (video/music/a call) even with no keyboard
        // or mouse input - so the collector should not treat them as idle.
        unsafe { render_peak().map(|p| p > 0.001).unwrap_or(false) }
    }

    fn session_locked(&mut self) -> bool {
        // When the workstation is locked, the secure (Winlogon) desktop is the
        // input desktop. We can't open it, or its name is not "Default" - either
        // way the user is away, so the session is locked.
        use windows::Win32::System::StationsAndDesktops::{
            CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_CONTROL_FLAGS,
            DESKTOP_READOBJECTS, UOI_NAME,
        };
        unsafe {
            match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) {
                Ok(desktop) => {
                    let mut buf = [0u16; 256];
                    let mut needed = 0u32;
                    let ok = GetUserObjectInformationW(
                        windows::Win32::Foundation::HANDLE(desktop.0),
                        UOI_NAME,
                        Some(buf.as_mut_ptr() as *mut std::ffi::c_void),
                        std::mem::size_of_val(&buf) as u32,
                        Some(&mut needed),
                    )
                    .is_ok();
                    let _ = CloseDesktop(desktop);
                    if !ok {
                        return false;
                    }
                    // The name is null-terminated; decode only up to the first
                    // NUL rather than the whole 256-wide buffer.
                    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    let name = String::from_utf16_lossy(&buf[..len]);
                    !name.eq_ignore_ascii_case("Default")
                }
                // Couldn't open the input desktop: the secure desktop is active.
                Err(_) => true,
            }
        }
    }
    fn session_suspended(&mut self) -> bool {
        use std::time::Instant;
        let now_instant = Instant::now();
        let now_tick = unsafe { GetTickCount() } as u64;
        let suspended = if let Some((prev_instant, prev_tick)) = self.last_tick {
            let wall_ms = now_instant.duration_since(prev_instant).as_millis() as u64;
            let tick_ms = now_tick.wrapping_sub(prev_tick);
            wall_ms > tick_ms + 3_000
        } else {
            false
        };
        self.last_tick = Some((now_instant, now_tick));
        suspended
    }
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

/// Resolve a process id to (app_key, app_name, full_path) using its executable
/// path. app_key is the lowercased exe filename (e.g. "chrome.exe"); app_name is
/// the file stem (e.g. "chrome"); full_path is the absolute exe path (used to
/// extract the app's real icon).
unsafe fn process_name(pid: u32) -> Option<(String, String, String)> {
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

    Some((file_name.to_lowercase(), stem, full))
}

pub struct WinTerminator;

impl WinTerminator {
    pub fn new() -> Self {
        WinTerminator
    }
}

impl Default for WinTerminator {
    fn default() -> Self {
        Self::new()
    }
}

impl super::ProcessTerminator for WinTerminator {
    fn terminate_process(&self, pid: u32) -> Result<(), super::TerminateError> {
        use windows::Win32::Foundation::{
            CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER,
        };
        use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

        unsafe {
            let handle = match OpenProcess(PROCESS_TERMINATE, false, pid) {
                Ok(h) => h,
                Err(e) => {
                    let code = e.code();
                    if code == ERROR_ACCESS_DENIED.to_hresult() {
                        return Err(super::TerminateError::PermissionDenied);
                    } else if code == ERROR_INVALID_PARAMETER.to_hresult() {
                        return Err(super::TerminateError::NoSuchProcess);
                    } else {
                        return Err(super::TerminateError::Other(e.to_string()));
                    }
                }
            };

            let res = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);

            res.map_err(|e| {
                let code = e.code();
                if code == ERROR_ACCESS_DENIED.to_hresult() {
                    super::TerminateError::PermissionDenied
                } else {
                    super::TerminateError::Other(e.to_string())
                }
            })
        }
    }
}
