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
pub mod crypto;
pub mod db;
pub mod grayscale;
pub mod icon;
pub mod models;
pub mod platform;
pub mod state;

use state::{AppState, Shared};
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Undo any bedtime grayscale we applied, so a best-effort OS change (notably
/// the Linux GTK theme swap) does not outlive the app. Called from every exit
/// path - the tray Quit item and the event-loop `Exit` event - because
/// `RunEvent::Exit` is not guaranteed to fire before the process dies on every
/// platform (Windows `app.exit(0)` in particular). `set_grayscale(false)` is
/// idempotent, so running it from both paths is harmless.
fn revert_grayscale_if_applied(app: &tauri::AppHandle) {
    if let Some(st) = app.try_state::<AppState>() {
        let applied = st
            .shared
            .lock()
            .map(|s| s.grayscale_applied)
            .unwrap_or(false);
        if applied {
            let _ = grayscale::set_grayscale(false);
        }
    }
}

/// Persist a final encrypted snapshot of the in-memory database on the way out,
/// so the most recent activity is saved. Idempotent and best-effort.
fn snapshot_db_on_exit(app: &tauri::AppHandle) {
    if let Some(st) = app.try_state::<AppState>() {
        if let Some((path, key)) = &st.enc {
            let guard = st.db.lock();
            if let Ok(conn) = guard {
                if let Err(e) = db::snapshot_encrypted(&conn, path, key) {
                    log::warn!("final encrypted snapshot failed: {e}");
                }
            }
        }
    }
}

/// Build and run the System Trace desktop app.
pub fn run() {
    // Test mode is opt-in via env var; gates both the WDIO plugin (so
    // production builds don't load it) and the data-isolation hooks inside
    // `.setup()`.
    let is_test = std::env::var("SYSTEM_TRACE_TEST_MODE").is_ok();

    let mut builder = tauri::Builder::default()
        // Single instance must be registered first: a second launch focuses the
        // existing window instead of starting a new collector.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                crate::platform::position_window_on_active_monitor(&w);
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());

    // Only load the WDIO bridge during E2E runs. End-user installs never see it.
    if is_test {
        builder = builder.plugin(tauri_plugin_wdio::init());
    }

    builder
        .setup(move |app| {
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

            // Production keeps the live database in memory and writes only
            // encrypted snapshots to disk, so no plaintext DB exists at rest.
            // Test mode uses a plaintext file DB to keep the E2E harness simple
            // (and free of the OS keyring).
            let enc_path = dir.join("system-trace.enc");
            let (conn, enc) = if is_test {
                (db::open(&db_path).expect("failed to open database"), None)
            } else {
                let data_exists = enc_path.exists();
                // If the key store errors while encrypted data exists, fail the
                // launch safely (do NOT mint a new key - that would lose the
                // data). It recovers on a later launch once the keyring is up.
                let key = match crypto::load_or_create_key(&dir.join("db.key"), data_exists) {
                    Ok(k) => k,
                    Err(e) => return Err(e.into()),
                };
                // If the snapshot can't be decrypted/loaded (wrong key or
                // corruption), quarantine it and start fresh so the app still
                // launches instead of panicking on every boot.
                let conn = match db::open_encrypted(&enc_path, &key, &db_path) {
                    Ok(c) => {
                        let _ = db::prune_corrupt_snapshot(&enc_path);
                        c
                    }
                    Err(e) => {
                        log::error!(
                            "could not open encrypted database ({e}); quarantining it and starting fresh"
                        );
                        if enc_path.exists() {
                            let _ = std::fs::rename(
                                &enc_path,
                                enc_path.with_extension("enc.corrupt"),
                            );
                        }
                        db::open_encrypted(&enc_path, &key, &db_path)
                            .expect("failed to open a fresh database")
                    }
                };
                (conn, Some((enc_path.clone(), key)))
            };

            // In E2E test mode the database is fresh each run, which would
            // otherwise drop the app on the first-run onboarding screen and
            // hide the dashboard the smoke tests assert on. Mark onboarding
            // complete so test mode boots straight to the dashboard.
            if is_test {
                let _ = db::set_setting(&conn, "onboarding_complete", "true");
            }

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

            let tracking_paused = if is_test {
                true
            } else {
                settings.tracking_paused
            };

            let shared = Arc::new(Mutex::new(Shared::new(
                settings.idle_threshold_secs as u64 * 1000,
                settings.capture_titles,
                tracking_paused,
            )));

            // Persist the initial encrypted snapshot. Only AFTER it succeeds do
            // we remove the legacy plaintext database we migrated from - so a
            // failed snapshot never leaves us with neither a readable plaintext
            // file nor a good encrypted one.
            if let Some((ref path, ref key)) = enc {
                match db::snapshot_encrypted(&conn, path, key) {
                    Ok(()) => {
                        if db_path.exists() {
                            let _ = std::fs::remove_file(&db_path);
                            let _ = std::fs::remove_file(db_path.with_extension("sqlite-wal"));
                            let _ = std::fs::remove_file(db_path.with_extension("sqlite-shm"));
                        }
                    }
                    Err(e) => log::error!(
                        "initial encrypted snapshot failed ({e}); keeping the plaintext database"
                    ),
                }
            }

            let db = Arc::new(Mutex::new(conn));

            app.manage(AppState {
                db: db.clone(),
                shared: shared.clone(),
                db_path: db_path.clone(),
                enc,
            });

            // Start the always-on background collector: window/idle/media/lock
            // sampling, batched event flushes, limits/wellbeing, and the
            // periodic encrypted snapshot. This call was accidentally dropped in
            // v0.3.0, which silently disabled ALL activity tracking - restore it.
            collector::spawn(app.handle().clone(), db.clone(), shared.clone());

            // Register the global pause/resume hotkey (default Ctrl+Alt+P).
            // Toggling shared.paused is enough: the collector reads it every
            // loop and the UI catches up on the next usage_tick.
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };
                let hotkey = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyP);
                let shared_for_key = shared.clone();
                let db_for_key = db.clone();
                let gs = app.global_shortcut();
                let reg = gs.on_shortcut(hotkey, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let mut new_paused = None;
                        if let Ok(mut s) = shared_for_key.lock() {
                            s.paused = !s.paused;
                            s.state = if s.paused {
                                crate::models::CollectorState::Paused
                            } else {
                                crate::models::CollectorState::Idle
                            };
                            if s.paused {
                                s.active_app = None;
                            }
                            new_paused = Some(s.paused);
                        }
                        // Persist so the pause state survives a restart, exactly
                        // like toggling it from the Settings UI does.
                        if let Some(paused) = new_paused {
                            if let Ok(conn) = db_for_key.lock() {
                                let _ = db::set_setting(
                                    &conn,
                                    "tracking_paused",
                                    if paused { "true" } else { "false" },
                                );
                            }
                        }
                    }
                });
                if let Err(e) = reg {
                    log::warn!("could not register pause hotkey: {e}");
                    // Surface the failure so Settings can show the chord as
                    // unavailable instead of looking dead.
                    if let Ok(mut s) = shared.lock() {
                        s.hotkey_registered = false;
                    }
                }
            }

            // Hide the window on autostart-from-boot launches (the autostart
            // plugin passes "--minimized") or when the user picked
            // "Start minimized to tray" in Settings. The collector is already
            // spinning at this point, so tracing continues either way.
            let launched_minimized = std::env::args().any(|a| a == "--minimized");
            if launched_minimized || settings.start_minimized {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

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
                            crate::platform::position_window_on_active_monitor(&w);
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        snapshot_db_on_exit(app);
                        revert_grayscale_if_applied(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // The whole point of System Trace is to keep tracing in the
            // background even after the user dismisses the window. Without
            // this handler, clicking the X tears the window down and Tauri
            // exits the process, which kills the collector. Hide the main
            // window instead and rely on the tray's Quit item for a real
            // exit. Child windows (none today) still close normally.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_overview,
            commands::get_range_overview,
            commands::get_day_overview,
            commands::get_focus_score,
            commands::get_category_goals,
            commands::set_category_goal,
            commands::remove_category_goal,
            commands::get_goal_streaks,
            commands::get_app_goals,
            commands::set_app_goal,
            commands::remove_app_goal,
            commands::get_app_icon,
            commands::search_usage,
            commands::save_focus_session,
            commands::list_focus_sessions,
            commands::backup_database,
            commands::restore_database,
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
            commands::get_hotkey_status,
            commands::focus_main_window,
            commands::save_report_pdf,
        ])
        .build(tauri::generate_context!())
        .expect("error while building System Trace")
        .run(|app_handle, event| {
            // Backstop for exit paths other than the tray Quit item (which
            // already reverts before calling app.exit). Idempotent.
            if let tauri::RunEvent::Exit = event {
                snapshot_db_on_exit(app_handle);
                revert_grayscale_if_applied(app_handle);
            }
        });
}
