#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Runs before the Tauri builder creates the webview, so the renamed app
    // directories (settings + WebView2 profile) are already in place.
    grabmeta_lib::migrate_legacy_directories();
    grabmeta_lib::run();
}
