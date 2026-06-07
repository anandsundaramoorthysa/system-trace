// System Trace - desktop binary entry point.
// On Windows release builds, hide the console window (GUI app).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    system_trace_lib::run();
}
