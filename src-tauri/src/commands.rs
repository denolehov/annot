use std::sync::Mutex;
use tauri::{AppHandle, State};

use crate::output::format_output;
use crate::state::{AppState, ContentNode, ContentResponse};

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
