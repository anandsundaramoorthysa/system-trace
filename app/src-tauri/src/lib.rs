//! System Trace core library. The binary (`main.rs`) is a thin entry point that
//! calls `run()`. Splitting into a lib lets `cargo test` exercise the pure core
//! (collector state machine, aggregation, migrations) without the OS watchers.

// The `objc` 0.2 crate uses a legacy `cfg(feature = "cargo-clippy")` check in
// its `msg_send!` / `class!` macros that trips the newer `unexpected_cfgs`
// lint. The lint fires inside the expanded macro, so it has to be allowed at
// the crate root (module-level `#![allow]` does not reach into upstream
// macro expansions).
#![allow(unexpected_cfgs)]

pub mod blocker;
pub mod collector;
pub mod commands;
pub mod db;
pub mod models;
pub mod platform;
pub mod state;

use state::{AppState, Shared};
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Build and run the System Trace desktop app.
pub fn run() {
    tauri::Builder::default()
        // Single instance must be registered first: a second launch focuses the
        // existing window instead of starting a new collector.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Check for test mode to avoid polluting real user data.
            let is_test = std::env::var("SYSTEM_TRACE_TEST_MODE").is_ok();

            // Local database in the OS app-data directory (no cloud).
            let dir = app
                .path()
                .app_data_dir()
                .expect("could not resolve app data dir");
            
            let db_path = if is_test {
                std::env::temp_dir().join("system-trace-test.sqlite")
            } else {
                dir.join("system-trace.sqlite")
            };

            // If it's a new test run, we might want to wipe the test DB.
            if is_test && db_path.exists() {
                let _ = std::fs::remove_file(&db_path);
            }

            let conn = db::open(&db_path).expect("failed to open database");

            let settings = db::get_settings(&conn).expect("failed to read settings");
            // Trim raw events past the retention window on startup.
            let _ = db::trim_old_events(&conn, settings.retention_days);

            // Apply the launch-at-login preference (skip in test mode).
            if !is_test {
                let mgr = app.autolaunch();
                if settings.launch_at_login {
                    let _ = mgr.enable();
                } else {
                    let _ = mgr.disable();
                }
            }

            let tracking_paused = if is_test { true } else { settings.tracking_paused };

            let shared = Arc::new(Mutex::new(Shared::new(
                settings.idle_threshold_secs as u64 * 1000,
                settings.capture_titles,
                tracking_paused,
            )));
            let db = Arc::new(Mutex::new(conn));

            app.manage(AppState {
                db: db.clone(),
                shared: shared.clone(),
            });

            // Start the always-on collector.
            collector::spawn(app.handle().clone(), db, shared);

            // System tray with Show / Quit.
            let show = MenuItemBuilder::with_id("show", "Show System Trace").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("System Trace")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_overview,
            commands::get_range_overview,
            commands::get_day_overview,
            commands::get_apps,
            commands::set_app_category,
            commands::get_categories,
            commands::upsert_category,
            commands::delete_category,
            commands::get_settings,
            commands::set_setting,
            commands::get_exclusions,
            commands::add_exclusion,
            commands::remove_exclusion,
            commands::export_data,
            commands::import_data,
            commands::wipe_all_data,
            commands::get_collector_state,
            commands::set_tracking_paused,
            commands::get_limits,
            commands::set_limit,
            commands::remove_limit,
            commands::get_block_rules,
            commands::set_block_rule,
            commands::remove_block_rule,
            commands::start_focus_session,
            commands::stop_focus_session,
            commands::get_focus_state,
            commands::apply_website_block,
            commands::clear_website_block,
        ])
        .run(tauri::generate_context!())
        .expect("error while running System Trace");
}
