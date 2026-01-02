use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use tauri::WebviewWindowBuilder;

pub mod asciiscript;
pub mod commands;
pub mod config;
pub mod diff;
pub mod error;
pub mod excalidraw_window;
pub mod files;
pub mod highlight;
pub mod id;
pub mod input;
pub mod lang;
pub mod markdown;
pub mod mcp;
pub mod mermaid_window;
pub mod output;
pub mod portal;
pub mod review;
pub mod state;
pub mod terraform;
pub mod window_state;

use commands::{
    compute_replace_diff, copy_section, copy_to_clipboard, create_bookmark,
    create_selection_bookmark, cycle_exit_mode, delete_annotation, delete_bookmark,
    delete_exit_mode, delete_tag, delete_terraform, export_to_obsidian, finish_review,
    get_bookmarks, get_config, get_content, get_exit_modes, get_tags, get_terraform_phrase,
    get_terraform_regions, get_theme, reload_config, reorder_exit_modes, save_config,
    save_content, set_exit_mode, set_session_comment, set_theme, update_bookmark,
    upsert_annotation, upsert_exit_mode, upsert_tag, upsert_terraform,
};
use excalidraw_window::{
    close_excalidraw_by_placeholder, excalidraw_cancel, excalidraw_save, get_excalidraw_context,
    open_excalidraw_window, ExcalidrawWindowState,
};
use files::{invalidate_file_cache, list_project_files, FileCache, FileCacheState};
use mermaid_window::{get_mermaid_source, open_mermaid_window, MermaidWindowState};

/// All IPC commands exposed to the frontend.
macro_rules! all_commands {
    () => {
        tauri::generate_handler![
            get_content,
            upsert_annotation,
            delete_annotation,
            upsert_terraform,
            delete_terraform,
            get_terraform_regions,
            get_terraform_phrase,
            finish_review,
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
            get_bookmarks,
            reload_config,
            create_bookmark,
            create_selection_bookmark,
            update_bookmark,
            delete_bookmark,
            copy_to_clipboard,
            copy_section,
            save_content,
            open_mermaid_window,
            get_mermaid_source,
            open_excalidraw_window,
            get_excalidraw_context,
            excalidraw_save,
            excalidraw_cancel,
            close_excalidraw_by_placeholder,
            get_config,
            save_config,
            export_to_obsidian,
            get_theme,
            set_theme,
            list_project_files,
            invalidate_file_cache,
            compute_replace_diff
        ]
    };
}
use review::{ActiveReview, Review};
use state::AppState;

/// Shared flag to prevent app exit in MCP mode.
pub type ShouldExit = Arc<AtomicBool>;

/// When true, CLI outputs JSON with base64 images instead of plain text.
/// Newtype wrapper to avoid Tauri state collision with ShouldExit.
pub struct JsonOutputFlag(pub AtomicBool);

impl JsonOutputFlag {
    pub fn new(value: bool) -> Self {
        Self(AtomicBool::new(value))
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Serializes MCP sessions so only one review runs at a time.
/// Held for the entire session lifecycle (window open → user closes → result returned).
pub type SessionLock = Mutex<()>;

/// Run in CLI mode (file/stdin input, prints result, exits).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(state: AppState, context: tauri::Context, json_output: bool) {
    // Convert AppState to Review (auto-detects file vs diff mode)
    let review = Review::cli(state.content, state.config, "main".to_string());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage::<ActiveReview>(Mutex::new(Some(review)))
        .manage::<ShouldExit>(Arc::new(AtomicBool::new(true))) // CLI mode: allow exit
        .manage::<JsonOutputFlag>(JsonOutputFlag::new(json_output))
        .manage(Mutex::new(MermaidWindowState::new()))
        .manage(Mutex::new(ExcalidrawWindowState::new()))
        .manage::<FileCacheState>(Mutex::new(FileCache::new()))
        .invoke_handler(all_commands!())
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

            let window = builder.build()?;

            // Restore saved window position/size (or keep defaults)
            window_state::restore_window_state(&window, window_state::WindowType::Main);

            // Save window state on close
            let window_for_save = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let _ =
                        window_state::save_window_state(&window_for_save, window_state::WindowType::Main);
                }
            });

            #[cfg(debug_assertions)]
            window.open_devtools();

            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}

/// Run in MCP server mode (no initial window, handles tool calls).
pub fn run_mcp(context: tauri::Context) {
    // No initial review - created per MCP tool call
    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_clone = should_exit.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage::<ActiveReview>(Mutex::new(None))
        .manage::<ShouldExit>(should_exit)
        .manage::<JsonOutputFlag>(JsonOutputFlag::new(false))
        .manage::<SessionLock>(Mutex::new(()))
        .manage(Mutex::new(MermaidWindowState::new()))
        .manage(Mutex::new(ExcalidrawWindowState::new()))
        .manage::<FileCacheState>(Mutex::new(FileCache::new()))
        .invoke_handler(all_commands!())
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
