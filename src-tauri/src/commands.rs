use std::sync::Mutex;
use tauri::{AppHandle, State};

use crate::config;
use crate::output::format_output;
use crate::state::{AppState, ContentNode, ContentResponse, ExitMode, Tag};

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
pub fn finish_session(state: State<Mutex<AppState>>, app: AppHandle) -> String {
    let output = {
        let state = state.lock().unwrap();
        format_output(&state)
    };

    // Print to stdout
    if !output.is_empty() {
        print!("{}", output);
    }

    // Close the app
    app.exit(0);

    output
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
