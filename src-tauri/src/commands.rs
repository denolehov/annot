use std::sync::Mutex;
use tauri::{AppHandle, State};

use crate::output::format_output;
use crate::state::{AppState, ContentNode, ContentResponse, ExitMode};

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
