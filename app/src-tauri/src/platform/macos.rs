//! macOS watcher. SYSTEM_DESIGN.md section 5.
//!
//! Active app via `NSWorkspace.frontmostApplication` (bundle id as app_key,
//! localized name for display). Idle via Quartz `CGEventSourceSecondsSinceLastEventType`.
//! Window titles need the Accessibility API and the user's permission; they are
//! left as None for now (app-level tracking works without that grant).
//!
//! NOTE: this file is `#[cfg(target_os = "macos")]` and is therefore compiled and
//! verified by the macOS CI job, not on a Windows dev box.

#![cfg(target_os = "macos")]

use super::{ActiveWindow, Watcher};
use std::ffi::CStr;
use std::os::raw::c_char;

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};

// Quartz event-source idle query. CGEventSourceStateID::CombinedSessionState = 0,
// kCGAnyInputEventType = 0xFFFFFFFF.
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: u32, event_type: u32) -> f64;
}

#[derive(Default)]
pub struct MacWatcher;

impl MacWatcher {
    pub fn new() -> Self {
        MacWatcher
    }
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

impl Watcher for MacWatcher {
    fn active_window(&mut self) -> Option<ActiveWindow> {
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
            Some(ActiveWindow {
                app_name: app_name.unwrap_or_else(|| app_key.clone()),
                app_key,
                // Title needs the Accessibility permission; deferred.
                title: None,
            })
        }
    }

    fn idle_ms(&mut self) -> u64 {
        let secs = unsafe { CGEventSourceSecondsSinceLastEventType(0, 0xFFFF_FFFF) };
        if secs.is_finite() && secs > 0.0 {
            (secs * 1000.0) as u64
        } else {
            0
        }
    }
}
