use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State, WebviewWindow};

use crate::config::{self, Config};
use crate::output::{format_output, OutputMode};
use crate::state::{AppState, ContentNode, ContentResponse, ExitMode, Tag};
use serde::Serialize;
use crate::{ResultSender, ShouldExit};

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
    state.lock().unwrap().to_response()
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
        .unwrap()
        .upsert_annotation(start_line, end_line, content);
}

#[tauri::command]
pub fn delete_annotation(state: State<Mutex<AppState>>, start_line: u32, end_line: u32) {
    state
        .lock()
        .unwrap()
        .delete_annotation(start_line, end_line);
}

#[tauri::command]
pub fn finish_session(
    state: State<Mutex<AppState>>,
    result_sender: State<ResultSender>,
    should_exit: State<ShouldExit>,
    _window: WebviewWindow,
    app: AppHandle,
) -> String {
    // Check if we're in MCP mode (has a result sender)
    let sender = {
        let mut guard = result_sender.lock().unwrap();
        guard.take()
    };

    let mode = if sender.is_some() { OutputMode::Mcp } else { OutputMode::Cli };

    let result = {
        let state = state.lock().unwrap();
        format_output(&state, mode)
    };

    let output_text = result.text.clone();

    if let Some(tx) = sender {
        // MCP mode: send full result via channel (includes images)
        let _ = tx.send(result);
    } else {
        // CLI mode: print text to stdout and exit app
        if !output_text.is_empty() {
            print!("{}", output_text);
        }
        should_exit.store(true, std::sync::atomic::Ordering::SeqCst);
        app.exit(0);
    }

    output_text
}

#[tauri::command]
pub fn set_exit_mode(state: State<Mutex<AppState>>, mode_id: Option<String>) {
    state.lock().unwrap().selected_exit_mode_id = mode_id;
}

#[tauri::command]
pub fn cycle_exit_mode(state: State<Mutex<AppState>>, direction: i32) -> Option<ExitMode> {
    let mut state = state.lock().unwrap();
    if state.exit_modes.is_empty() {
        return None;
    }

    // Find current index
    let current_index = state
        .selected_exit_mode_id
        .as_ref()
        .and_then(|id| state.exit_modes.iter().position(|m| &m.id == id))
        .unwrap_or(0);

    // Calculate new index with wrapping
    let len = state.exit_modes.len() as i32;
    let new_index = ((current_index as i32 + direction) % len + len) % len;

    let new_mode = state.exit_modes[new_index as usize].clone();
    state.selected_exit_mode_id = Some(new_mode.id.clone());

    Some(new_mode)
}

#[tauri::command]
pub fn set_session_comment(state: State<Mutex<AppState>>, content: Option<Vec<ContentNode>>) {
    state.lock().unwrap().session_comment = content;
}

#[tauri::command]
pub fn get_tags(state: State<Mutex<AppState>>) -> Vec<Tag> {
    state.lock().unwrap().tags.clone()
}

#[tauri::command]
pub fn upsert_tag(state: State<Mutex<AppState>>, tag: Tag) -> Vec<Tag> {
    let mut state = state.lock().unwrap();

    // Find existing tag by ID and update, or add new
    if let Some(existing) = state.tags.iter_mut().find(|t| t.id == tag.id) {
        existing.name = tag.name;
        existing.instruction = tag.instruction;
    } else {
        state.tags.push(tag);
    }

    // Persist to disk (with locking and merge)
    if let Err(e) = config::save_tags(&state.tags, &state.deleted_tag_ids) {
        eprintln!("Warning: Failed to save tags: {}", e);
    }

    state.tags.clone()
}

#[tauri::command]
pub fn delete_tag(state: State<Mutex<AppState>>, id: String) -> Vec<Tag> {
    let mut state = state.lock().unwrap();
    state.tags.retain(|t| t.id != id);
    state.deleted_tag_ids.insert(id);

    // Persist to disk (with locking and merge)
    if let Err(e) = config::save_tags(&state.tags, &state.deleted_tag_ids) {
        eprintln!("Warning: Failed to save tags: {}", e);
    }

    state.tags.clone()
}

#[tauri::command]
pub fn get_exit_modes(state: State<Mutex<AppState>>) -> Vec<ExitMode> {
    state.lock().unwrap().exit_modes.clone()
}

#[tauri::command]
pub fn upsert_exit_mode(state: State<Mutex<AppState>>, mode: ExitMode) -> Vec<ExitMode> {
    let mut state = state.lock().unwrap();

    // Find existing mode by ID and update, or add new
    if let Some(existing) = state.exit_modes.iter_mut().find(|m| m.id == mode.id) {
        existing.name = mode.name;
        existing.color = mode.color;
        existing.instruction = mode.instruction;
        existing.order = mode.order;
    } else {
        state.exit_modes.push(mode);
    }

    // Sort by order
    state.exit_modes.sort_by_key(|m| m.order);

    // Persist to disk (with locking and merge)
    if let Err(e) = config::save_exit_modes(&state.exit_modes, &state.deleted_exit_mode_ids) {
        eprintln!("Warning: Failed to save exit modes: {}", e);
    }

    state.exit_modes.clone()
}

#[tauri::command]
pub fn delete_exit_mode(state: State<Mutex<AppState>>, id: String) -> Vec<ExitMode> {
    let mut state = state.lock().unwrap();
    state.exit_modes.retain(|m| m.id != id);
    state.deleted_exit_mode_ids.insert(id);

    // Persist to disk (with locking and merge)
    if let Err(e) = config::save_exit_modes(&state.exit_modes, &state.deleted_exit_mode_ids) {
        eprintln!("Warning: Failed to save exit modes: {}", e);
    }

    state.exit_modes.clone()
}

#[tauri::command]
pub fn reorder_exit_modes(state: State<Mutex<AppState>>, ids: Vec<String>) -> Vec<ExitMode> {
    let mut state = state.lock().unwrap();

    // Update order based on position in ids array
    for (new_order, id) in ids.iter().enumerate() {
        if let Some(mode) = state.exit_modes.iter_mut().find(|m| &m.id == id) {
            mode.order = new_order as u32;
        }
    }

    // Sort by new order
    state.exit_modes.sort_by_key(|m| m.order);

    // Persist to disk (with locking and merge)
    if let Err(e) = config::save_exit_modes(&state.exit_modes, &state.deleted_exit_mode_ids) {
        eprintln!("Warning: Failed to save exit modes: {}", e);
    }

    state.exit_modes.clone()
}

#[tauri::command]
pub fn copy_to_clipboard(state: State<Mutex<AppState>>, mode: CopyMode) -> Result<(), String> {
    let state = state.lock().unwrap();

    // Reconstruct raw content from lines
    let raw_content: String = state
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
    let state = state.lock().unwrap();

    // Reconstruct raw content from lines (same as copy_to_clipboard)
    let raw_content: String = state
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
    let state = state.lock().unwrap();

    // Reconstruct raw content from lines
    let content: String = state
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
        .lines
        .iter()
        .find(|l| l.content.starts_with("# "))
        .map(|l| l.content.trim_start_matches("# ").trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(&state.label);

    // Build Obsidian URI with clipboard parameter
    let url = format!(
        "obsidian://new?vault={}&name={}&clipboard=true",
        urlencoding::encode(&vault_name),
        urlencoding::encode(note_name)
    );

    Ok(ObsidianExportResponse { url })
}
