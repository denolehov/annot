use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use output::FormatResult;

use tauri::WebviewWindowBuilder;

pub mod commands;
pub mod config;
pub mod diff;
pub mod highlight;
pub mod input;
pub mod markdown;
pub mod mcp;
pub mod mermaid_window;
pub mod output;
mod perf;
pub mod state;

use commands::{
    copy_to_clipboard, cycle_exit_mode, delete_annotation, delete_exit_mode, delete_tag,
    export_to_obsidian, finish_session, get_config, get_content, get_exit_modes, get_tags,
    reorder_exit_modes, save_config, save_content, set_exit_mode, set_session_comment,
    upsert_annotation, upsert_exit_mode, upsert_tag,
};
use mermaid_window::{get_mermaid_source, open_mermaid_window, MermaidWindowState};
use state::AppState;

/// Shared flag to prevent app exit in MCP mode.
pub type ShouldExit = Arc<AtomicBool>;

/// Sender for MCP session results.
pub type ResultSender = Mutex<Option<Sender<FormatResult>>>;

/// Run in CLI mode (file/stdin input, prints result, exits).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(state: AppState, context: tauri::Context) {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(state))
        .manage::<ResultSender>(Mutex::new(None))
        .manage::<ShouldExit>(Arc::new(AtomicBool::new(true))) // CLI mode: allow exit
        .manage(Mutex::new(MermaidWindowState::new()))
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
            reorder_exit_modes,
            copy_to_clipboard,
            save_content,
            open_mermaid_window,
            get_mermaid_source,
            get_config,
            save_config,
            export_to_obsidian
        ])
        .setup(|app| {
            // Create window programmatically (not from config, for MCP compatibility)
            let mut builder = WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("annot")
            .inner_size(1000.0, 700.0)
            .visible(false) // Will be shown after content loads
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true);

            #[cfg(target_os = "macos")]
            {
                builder = builder.traffic_light_position(tauri::LogicalPosition::new(12.0, 22.0));
            }

            let _window = builder.build()?;

            #[cfg(debug_assertions)]
            _window.open_devtools();

            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}

/// Run in MCP server mode (no initial window, handles tool calls).
pub fn run_mcp(context: tauri::Context) {
    // Create empty initial state (will be replaced per-session)
    let initial_state = AppState::empty();
    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_clone = should_exit.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(initial_state))
        .manage::<ResultSender>(Mutex::new(None))
        .manage::<ShouldExit>(should_exit)
        .manage(Mutex::new(MermaidWindowState::new()))
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
            reorder_exit_modes,
            copy_to_clipboard,
            save_content,
            open_mermaid_window,
            get_mermaid_source,
            get_config,
            save_config,
            export_to_obsidian
        ])
        .setup(|app| {
            // Set accessory mode on macOS (hide dock icon)
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                app.set_activation_policy(ActivationPolicy::Accessory);
            }

            // Spawn MCP server thread
            let app_handle = app.handle().clone();
            mcp::spawn_mcp_thread(app_handle);

            Ok(())
        })
        .build(context)
        .expect("error while building tauri application")
        .run(move |_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !should_exit_clone.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}
