//! Shared, in-memory collector state, plus the Tauri-managed app state.
//!
//! `Shared` is the small live snapshot the collector updates each tick and the
//! commands read (current state, active app, live settings the loop honors).
//! `AppState` is what `#[tauri::command]` handlers receive via `tauri::State`.

use crate::models::CollectorState;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Live state shared between the collector thread and the command handlers.
pub struct Shared {
    pub state: CollectorState,
    pub active_app: Option<String>,
    /// When true, the collector closes the open session and stops counting.
    pub paused: bool,
    /// Idle threshold the loop honors, in milliseconds (mirrors the setting).
    pub idle_threshold_ms: u64,
    /// Whether the loop persists window titles (mirrors the setting).
    pub capture_titles: bool,
    /// Phase 2: focus mode is on (block rules are enforced as nudges).
    pub focus_active: bool,
    /// Phase 2: when the current focus session auto-ends (UTC unix-millis).
    pub focus_ends_ms: Option<i64>,
}

impl Shared {
    pub fn new(idle_threshold_ms: u64, capture_titles: bool, paused: bool) -> Self {
        Shared {
            state: if paused {
                CollectorState::Paused
            } else {
                CollectorState::Idle
            },
            active_app: None,
            paused,
            idle_threshold_ms,
            capture_titles,
            focus_active: false,
            focus_ends_ms: None,
        }
    }
}

/// Application state managed by Tauri and shared with every command.
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub shared: Arc<Mutex<Shared>>,
}
