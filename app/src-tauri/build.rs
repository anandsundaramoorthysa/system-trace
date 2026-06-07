fn main() {
    // Generates the Tauri context (parses tauri.conf.json, embeds icons and the
    // capability/permission set) at compile time. Must be the build script for
    // any Tauri 2 app.
    tauri_build::build();
}
