//! macOS watcher. SYSTEM_DESIGN.md section 5.
//!
//! Active app via `NSWorkspace.frontmostApplication` (bundle id as app_key,
//! localized name for display). Idle via Quartz `CGEventSourceSecondsSinceLastEventType`.
//! Frontmost window title via the Accessibility API (requires the user's
//! Accessibility permission; yields `None` without it). Media-playing via
//! CoreAudio and screen-lock via the window-server session dictionary.
//!
//! NOTE: this file is `#[cfg(target_os = "macos")]` and is therefore compiled and
//! verified by the macOS CI job, not on a Windows dev box.

#![cfg(target_os = "macos")]

use super::{ActiveWindow, Watcher};
use std::ffi::CStr;
use std::os::raw::c_char;

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;

// Quartz event-source idle query. CGEventSourceStateID::CombinedSessionState = 0,
// kCGAnyInputEventType = 0xFFFFFFFF.
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: u32, event_type: u32) -> f64;
}

// CoreGraphics session dictionary (used to detect a locked screen) and CoreAudio
// (used to detect audio actually playing). Both are best-effort and fully
// guarded; any failure degrades to "unknown" (false).
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGSessionCopyCurrentDictionary() -> id;
}

// Accessibility (AX) API for the frontmost window title. Requires the user to
// grant Accessibility permission (System Settings -> Privacy & Security ->
// Accessibility); without it the calls return an error and we yield `None`.
type AXUIElementRef = *const std::os::raw::c_void;
type CFTypeRef = *const std::os::raw::c_void;
type CFStringRef = *const std::os::raw::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn CFRelease(cf: CFTypeRef);
    fn CFStringCreateWithCString(
        alloc: CFTypeRef,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    fn CFStringGetCStringPtr(s: CFStringRef, encoding: u32) -> *const c_char;
    fn CFStringGetCString(s: CFStringRef, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
}

const KCF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

/// Make a CFString from a Rust &str (caller must CFRelease).
unsafe fn cfstr(s: &str) -> CFStringRef {
    let c = std::ffi::CString::new(s).unwrap();
    CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), KCF_STRING_ENCODING_UTF8)
}

/// Convert a CFStringRef to a Rust String.
unsafe fn cfstring_to_rust(s: CFStringRef) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let ptr = CFStringGetCStringPtr(s, KCF_STRING_ENCODING_UTF8);
    if !ptr.is_null() {
        return Some(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    let mut buf = vec![0i8; 1024];
    if CFStringGetCString(
        s,
        buf.as_mut_ptr(),
        buf.len() as isize,
        KCF_STRING_ENCODING_UTF8,
    ) {
        Some(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
    } else {
        None
    }
}

/// Frontmost window title of the app with the given pid, via the AX API.
unsafe fn ax_window_title(pid: i32) -> Option<String> {
    let app = AXUIElementCreateApplication(pid);
    if app.is_null() {
        return None;
    }
    let mut title: Option<String> = None;
    let focused_attr = cfstr("AXFocusedWindow");
    let mut window: CFTypeRef = std::ptr::null();
    if AXUIElementCopyAttributeValue(app, focused_attr, &mut window) == 0 && !window.is_null() {
        let title_attr = cfstr("AXTitle");
        let mut value: CFTypeRef = std::ptr::null();
        if AXUIElementCopyAttributeValue(window as AXUIElementRef, title_attr, &mut value) == 0
            && !value.is_null()
        {
            title = cfstring_to_rust(value as CFStringRef).filter(|s| !s.is_empty());
            CFRelease(value);
        }
        CFRelease(title_attr);
        CFRelease(window);
    }
    CFRelease(focused_attr);
    CFRelease(app);
    title
}

#[repr(C)]
struct AudioObjectPropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        qualifier_size: u32,
        qualifier: *const std::os::raw::c_void,
        data_size: *mut u32,
        data: *mut std::os::raw::c_void,
    ) -> i32;
}

/// Build a CoreAudio four-char-code selector from a 4-byte tag.
const fn fourcc(b: &[u8; 4]) -> u32 {
    ((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | (b[3] as u32)
}

const K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;

/// Whether any audio is currently being rendered by the default output device.
unsafe fn macos_audio_running() -> bool {
    // 1. Resolve the default output device id.
    let default_out_addr = AudioObjectPropertyAddress {
        selector: fourcc(b"dOut"), // kAudioHardwarePropertyDefaultOutputDevice
        scope: fourcc(b"glob"),    // kAudioObjectPropertyScopeGlobal
        element: 0,                // kAudioObjectPropertyElementMain
    };
    let mut device_id: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = AudioObjectGetPropertyData(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        &default_out_addr,
        0,
        std::ptr::null(),
        &mut size,
        &mut device_id as *mut u32 as *mut _,
    );
    if status != 0 || device_id == 0 {
        return false;
    }

    // 2. Ask whether that device is running somewhere (i.e. audio is flowing).
    let running_addr = AudioObjectPropertyAddress {
        selector: fourcc(b"gone"), // kAudioDevicePropertyDeviceIsRunningSomewhere
        scope: fourcc(b"glob"),
        element: 0,
    };
    let mut running: u32 = 0;
    let mut rsize = std::mem::size_of::<u32>() as u32;
    let status = AudioObjectGetPropertyData(
        device_id,
        &running_addr,
        0,
        std::ptr::null(),
        &mut rsize,
        &mut running as *mut u32 as *mut _,
    );
    status == 0 && running != 0
}

/// Convert an NSString to a Rust String (empty when null).
unsafe fn nsstring_to_string(ns: id) -> Option<String> {
    if ns == nil {
        return None;
    }
    let ptr: *const c_char = msg_send![ns, UTF8String];
    if ptr.is_null() {
        return None;
    }
    let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub trait MacPlatformHelper: Send {
    fn is_trusted(&self) -> bool;
    fn request_permission(&self) -> bool;
    fn frontmost_app_info(&self) -> Option<(String, String, Option<String>, i32)>;
    fn ax_window_title(&self, pid: i32) -> Option<String>;
}

pub struct RealMacPlatformHelper;

impl MacPlatformHelper for RealMacPlatformHelper {
    fn is_trusted(&self) -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    fn request_permission(&self) -> bool {
        unsafe {
            let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
            let value = CFBoolean::true_value();
            let dict = CFDictionary::from_CFType_pairs(&[(key, value.as_CFType())]);
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as CFTypeRef)
        }
    }

    fn frontmost_app_info(&self) -> Option<(String, String, Option<String>, i32)> {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            if workspace == nil {
                return None;
            }
            let app: id = msg_send![workspace, frontmostApplication];
            if app == nil {
                return None;
            }
            let name: id = msg_send![app, localizedName];
            let bundle: id = msg_send![app, bundleIdentifier];
            let app_name = nsstring_to_string(name);
            let app_key = nsstring_to_string(bundle)
                .or_else(|| app_name.clone())
                .unwrap_or_else(|| "unknown".to_string());
            let app_path = {
                let url: id = msg_send![app, bundleURL];
                if url == nil {
                    None
                } else {
                    let p: id = msg_send![url, path];
                    nsstring_to_string(p)
                }
            };
            let pid: i32 = msg_send![app, processIdentifier];
            let display_name = app_name.unwrap_or_else(|| app_key.clone());
            Some((app_key, display_name, app_path, pid))
        }
    }

    fn ax_window_title(&self, pid: i32) -> Option<String> {
        unsafe { ax_window_title(pid) }
    }
}

pub struct MacWatcher {
    capture_titles: bool,
    helper: Box<dyn MacPlatformHelper>,
}

impl MacWatcher {
    pub fn new() -> Self {
        MacWatcher {
            capture_titles: false,
            helper: Box::new(RealMacPlatformHelper),
        }
    }

    #[cfg(test)]
    pub fn with_helper(helper: Box<dyn MacPlatformHelper>) -> Self {
        MacWatcher {
            capture_titles: false,
            helper,
        }
    }
}

impl Default for MacWatcher {
    fn default() -> Self {
        MacWatcher::new()
    }
}

impl Watcher for MacWatcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
        let (app_key, app_name, app_path, pid) = self.helper.frontmost_app_info()?;

        let title = if self.capture_titles && self.helper.is_trusted() {
            self.helper.ax_window_title(pid)
        } else {
            None
        };

        Some(ActiveWindow {
            app_name,
            app_key,
            title,
            app_path,
            pid: Some(pid as u32),
        })
    }

    fn idle_ms(&mut self) -> u64 {
        let secs = unsafe { CGEventSourceSecondsSinceLastEventType(0, 0xFFFF_FFFF) };
        if secs.is_finite() && secs > 0.0 {
            (secs * 1000.0) as u64
        } else {
            0
        }
    }

    fn is_media_playing(&mut self) -> bool {
        unsafe { macos_audio_running() }
    }

    fn session_locked(&mut self) -> bool {
        // The window-server session dictionary carries CGSSessionScreenIsLocked
        // when the screen is locked. Best-effort; any failure returns false.
        unsafe {
            let dict: id = CGSessionCopyCurrentDictionary();
            if dict == nil {
                return false;
            }
            let key: id = msg_send![class!(NSString),
                stringWithUTF8String: c"CGSSessionScreenIsLocked".as_ptr()];
            let val: id = msg_send![dict, objectForKey: key];
            let locked = if val == nil {
                false
            } else {
                let b: bool = msg_send![val, boolValue];
                b
            };
            // CGSessionCopyCurrentDictionary returns a +1 reference (toll-free
            // bridged to NSDictionary); balance it.
            let _: () = msg_send![dict, release];
            locked
        }
    }

    fn set_capture_titles(&mut self, on: bool) {
        if on && !self.capture_titles {
            // Check if trusted, and if not, request permission (prompt the user)
            if !self.helper.is_trusted() {
                self.helper.request_permission();
            }
        }
        self.capture_titles = on;
    }
}

pub struct MacTerminator;

impl MacTerminator {
    pub fn new() -> Self {
        MacTerminator
    }
}

impl Default for MacTerminator {
    fn default() -> Self {
        Self::new()
    }
}

impl super::ProcessTerminator for MacTerminator {
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
pub struct FakeMacPlatformHelper {
    pub trusted: bool,
    pub permission_requested: std::sync::atomic::AtomicBool,
    pub frontmost_app: Option<(String, String, Option<String>, i32)>,
    pub window_title: Option<String>,
}

#[cfg(test)]
impl MacPlatformHelper for FakeMacPlatformHelper {
    fn is_trusted(&self) -> bool {
        self.trusted
    }

    fn request_permission(&self) -> bool {
        self.permission_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
        true
    }

    fn frontmost_app_info(&self) -> Option<(String, String, Option<String>, i32)> {
        self.frontmost_app.clone()
    }

    fn ax_window_title(&self, _pid: i32) -> Option<String> {
        self.window_title.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_mac_watcher_gating_and_permission() {
        let helper = Box::new(FakeMacPlatformHelper {
            trusted: false,
            permission_requested: std::sync::atomic::AtomicBool::new(false),
            frontmost_app: Some((
                "com.apple.Safari".to_string(),
                "Safari".to_string(),
                Some("/Applications/Safari.app".to_string()),
                1234,
            )),
            window_title: Some("Apple".to_string()),
        });

        // Pointer to the permission_requested AtomicBool so we can assert on it.
        let permission_requested_ptr =
            unsafe { &*(&helper.permission_requested as *const std::sync::atomic::AtomicBool) };

        let mut watcher = MacWatcher::with_helper(helper);

        // 1. Gating test: when capture_titles is false, active_window returns no title,
        // and set_capture_titles(false) doesn't request permission.
        watcher.set_capture_titles(false);
        let active = watcher.active_window().unwrap();
        assert_eq!(active.app_key, "com.apple.Safari");
        assert_eq!(active.title, None);
        assert!(!permission_requested_ptr.load(Ordering::SeqCst));

        // 2. Permission request on transition test: when setting capture_titles to true
        // (and it is not trusted), it should request permission.
        watcher.set_capture_titles(true);
        assert!(permission_requested_ptr.load(Ordering::SeqCst));

        // 3. Graceful degradation: since it is not trusted, active_window still returns None for title
        let active_untrusted = watcher.active_window().unwrap();
        assert_eq!(active_untrusted.title, None);
    }

    #[test]
    fn test_mac_watcher_trusted_title() {
        let helper = Box::new(FakeMacPlatformHelper {
            trusted: true,
            permission_requested: std::sync::atomic::AtomicBool::new(false),
            frontmost_app: Some((
                "com.apple.Safari".to_string(),
                "Safari".to_string(),
                Some("/Applications/Safari.app".to_string()),
                1234,
            )),
            window_title: Some("Apple".to_string()),
        });

        let mut watcher = MacWatcher::with_helper(helper);
        watcher.set_capture_titles(true);

        // When trusted, active_window returns the captured title.
        let active = watcher.active_window().unwrap();
        assert_eq!(active.app_key, "com.apple.Safari");
        assert_eq!(active.title.as_deref(), Some("Apple"));
    }

    use crate::platform::ProcessTerminator;
    use std::process::{Command, Stdio};

    #[test]
    fn test_mac_terminator_success() {
        // Spawn a dummy process (e.g., sleep 10)
        let mut child = Command::new("sleep")
            .arg("10")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn sleep process");

        let pid = child.id();
        let terminator = MacTerminator::new();

        // Terminate the process
        let res = terminator.terminate_process(pid);
        assert!(res.is_ok());

        // Wait for the child to exit and verify it was terminated
        let status = child.wait().expect("Failed to wait on child");
        assert!(!status.success()); // Should be killed by signal (not success exit)
    }

    #[test]
    fn test_mac_terminator_no_such_process() {
        let terminator = MacTerminator::new();
        // Use a highly likely unused PID, e.g. 999999
        let res = terminator.terminate_process(999999);
        assert_eq!(res, Err(crate::platform::TerminateError::NoSuchProcess));
    }
}
