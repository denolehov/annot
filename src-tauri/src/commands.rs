use std::path::PathBuf;
use std::sync::atomic::Ordering;

use parking_lot::Mutex;
use tauri::{AppHandle, State};

use crate::config::{self, Config};
use crate::output::{format_output, OutputMode};
use crate::state::{AppState, ContentNode, ContentResponse, ExitMode, Tag};
use serde::Serialize;
use crate::{ResultSender, ShouldExit};

/// Session mode derived from ContentSource.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    Cli,
    Mcp,
}

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyMode {
    Content,
    Annotations,
    All,
}

#[tauri::command]
pub fn get_content(state: State<Mutex<AppState>>) -> ContentResponse {
    state.lock().to_response()
}

#[tauri::command]
pub fn upsert_annotation(
    state: State<Mutex<AppState>>,
    start_line: u32,
    end_line: u32,
    content: Vec<ContentNode>,
) {
    state
        .lock()
        .upsert_annotation(start_line, end_line, content);
}

#[tauri::command]
pub fn delete_annotation(state: State<Mutex<AppState>>, start_line: u32, end_line: u32) {
    state
        .lock()
        .delete_annotation(start_line, end_line);
}

/// Returns the session mode, derived from ContentSource
#[tauri::command]
pub fn get_session_mode(state: State<Mutex<AppState>>) -> SessionMode {
    if state.lock().content.source.is_mcp() {
        SessionMode::Mcp
    } else {
        SessionMode::Cli
    }
}

/// CLI mode: format output, print to stdout, exit process
#[tauri::command]
pub fn finish_session_cli(
    state: State<Mutex<AppState>>,
    should_exit: State<ShouldExit>,
    app: AppHandle,
) {
    let result = {
        let state_guard = state.lock();
        format_output(&state_guard, OutputMode::Cli)
    };

    if !result.text.is_empty() {
        print!("{}", result.text);
    }

    should_exit.store(true, Ordering::SeqCst);
    app.exit(0);
}

/// MCP mode: format output, send via channel
#[tauri::command]
pub fn finish_session_mcp(
    state: State<Mutex<AppState>>,
    result_sender: State<ResultSender>,
) -> Result<(), String> {
    let tx = result_sender
        .lock()
        .take()
        .ok_or_else(|| "MCP sender missing or already used".to_string())?;

    let result = {
        let state_guard = state.lock();
        format_output(&state_guard, OutputMode::Mcp)
    };

    tx.send(result)
        .map_err(|_| "Failed to send result to MCP host".to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_exit_mode(state: State<Mutex<AppState>>, mode_id: Option<String>) {
    state.lock().session.selected_exit_mode_id = mode_id;
}

#[tauri::command]
pub fn cycle_exit_mode(state: State<Mutex<AppState>>, direction: i32) -> Option<ExitMode> {
    let mut state = state.lock();
    let exit_modes = state.config.exit_modes();
    if exit_modes.is_empty() {
        return None;
    }

    // Find current index
    let current_index = state
        .session
        .selected_exit_mode_id
        .as_ref()
        .and_then(|id| exit_modes.iter().position(|m| &m.id == id))
        .unwrap_or(0);

    // Calculate new index with wrapping
    let len = exit_modes.len() as i32;
    let new_index = ((current_index as i32 + direction) % len + len) % len;

    let new_mode = exit_modes[new_index as usize].clone();
    state.session.selected_exit_mode_id = Some(new_mode.id.clone());

    Some(new_mode)
}

#[tauri::command]
pub fn set_session_comment(state: State<Mutex<AppState>>, content: Option<Vec<ContentNode>>) {
    state.lock().session.comment = content;
}

#[tauri::command]
pub fn get_tags(state: State<Mutex<AppState>>) -> Vec<Tag> {
    state.lock().config.tags().to_vec()
}

#[tauri::command]
pub fn upsert_tag(state: State<Mutex<AppState>>, tag: Tag) -> Vec<Tag> {
    let mut state = state.lock();
    state.config.upsert_tag(tag);
    state.config.tags().to_vec()
}

#[tauri::command]
pub fn delete_tag(state: State<Mutex<AppState>>, id: String) -> Vec<Tag> {
    let mut state = state.lock();
    state.config.delete_tag(&id);
    state.config.tags().to_vec()
}

#[tauri::command]
pub fn get_exit_modes(state: State<Mutex<AppState>>) -> Vec<ExitMode> {
    state.lock().config.exit_modes().to_vec()
}

#[tauri::command]
pub fn upsert_exit_mode(state: State<Mutex<AppState>>, mode: ExitMode) -> Vec<ExitMode> {
    let mut state = state.lock();
    state.config.upsert_exit_mode(mode);
    state.config.exit_modes().to_vec()
}

#[tauri::command]
pub fn delete_exit_mode(state: State<Mutex<AppState>>, id: String) -> Vec<ExitMode> {
    let mut state = state.lock();
    state.config.delete_exit_mode(&id);
    state.config.exit_modes().to_vec()
}

#[tauri::command]
pub fn reorder_exit_modes(state: State<Mutex<AppState>>, ids: Vec<String>) -> Vec<ExitMode> {
    let mut state = state.lock();
    state.config.reorder_exit_modes(ids);
    state.config.exit_modes().to_vec()
}

#[tauri::command]
pub fn copy_to_clipboard(state: State<Mutex<AppState>>, mode: CopyMode) -> Result<(), String> {
    let state = state.lock();

    // Reconstruct raw content from lines
    let raw_content: String = state
        .content
        .lines
        .iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let text = match mode {
        CopyMode::Content => raw_content,
        CopyMode::Annotations => format_output(&state, OutputMode::Clipboard).text,
        CopyMode::All => {
            let annotations = format_output(&state, OutputMode::Clipboard).text;
            if annotations.is_empty() {
                raw_content
            } else {
                format!("{}\n\n---\n\n{}", raw_content, annotations)
            }
        }
    };

    if text.is_empty() {
        return Err("Nothing to copy".to_string());
    }

    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| e.to_string())
}

/// Response from save_content command.
#[derive(Serialize)]
pub struct SaveContentResponse {
    /// Absolute path where the file was saved.
    pub saved_path: String,
    /// New label for the header (filename portion).
    pub new_label: String,
}

#[tauri::command]
pub fn save_content(
    state: State<Mutex<AppState>>,
    path: String,
) -> Result<SaveContentResponse, String> {
    let state = state.lock();

    // Reconstruct raw content from lines (same as copy_to_clipboard)
    let raw_content: String = state
        .content
        .lines
        .iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Resolve path (relative to cwd if not absolute)
    let path = PathBuf::from(&path);
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get working directory: {}", e))?
            .join(path)
    };

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directories: {}", e))?;
    }

    // Write the file
    std::fs::write(&path, &raw_content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    // Extract filename for new label
    let new_label = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    Ok(SaveContentResponse {
        saved_path: path.display().to_string(),
        new_label,
    })
}

// --- Config commands ---

#[tauri::command]
pub fn get_config() -> Config {
    config::load_config()
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    config::save_config(&config).map_err(|e| e.to_string())
}

// --- Obsidian export ---

/// Response from export_to_obsidian command.
#[derive(Serialize)]
pub struct ObsidianExportResponse {
    /// The obsidian:// URI to open.
    pub url: String,
}

#[tauri::command]
pub fn export_to_obsidian(
    state: State<Mutex<AppState>>,
    vault_name: String,
) -> Result<ObsidianExportResponse, String> {
    let state = state.lock();

    // Reconstruct raw content from lines
    let content: String = state
        .content
        .lines
        .iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Copy content to clipboard (Rust-side to avoid permission issues)
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(&content))
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;

    // Use H1 title as note name if present, otherwise fall back to label
    let note_name = state
        .content
        .lines
        .iter()
        .find(|l| l.content.starts_with("# "))
        .map(|l| l.content.trim_start_matches("# ").trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(&state.content.label);

    // Build Obsidian URI with clipboard parameter
    let url = format!(
        "obsidian://new?vault={}&name={}&clipboard=true",
        urlencoding::encode(&vault_name),
        urlencoding::encode(note_name)
    );

    Ok(ObsidianExportResponse { url })
}
