use std::sync::Mutex;

use tauri::Manager;

pub mod commands;
pub mod state;

use commands::get_content;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(state: AppState) {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(state))
        .invoke_handler(tauri::generate_handler![get_content])
        .setup(|app| {
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
