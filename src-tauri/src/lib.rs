use std::sync::Mutex;

use tauri::Manager;

pub mod commands;
pub mod highlight;
pub mod output;
pub mod state;

use commands::{
    cycle_exit_mode, delete_annotation, finish_session, get_content, set_exit_mode,
    set_session_comment, upsert_annotation,
};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(state: AppState) {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(state))
        .invoke_handler(tauri::generate_handler![
            get_content,
            upsert_annotation,
            delete_annotation,
            finish_session,
            set_exit_mode,
            cycle_exit_mode,
            set_session_comment
        ])
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
