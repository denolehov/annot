use std::sync::Mutex;

use tauri::Manager;

pub mod commands;
pub mod config;
pub mod highlight;
pub mod input;
pub mod output;
pub mod state;

use commands::{
    cycle_exit_mode, delete_annotation, delete_exit_mode, delete_tag, finish_session, get_content,
    get_exit_modes, get_tags, reorder_exit_modes, set_exit_mode, set_session_comment,
    upsert_annotation, upsert_exit_mode, upsert_tag,
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
            set_session_comment,
            get_tags,
            upsert_tag,
            delete_tag,
            get_exit_modes,
            upsert_exit_mode,
            delete_exit_mode,
            reorder_exit_modes
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
