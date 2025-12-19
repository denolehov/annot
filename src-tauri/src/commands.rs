use std::sync::Mutex;
use tauri::State;

use crate::state::{AppState, ContentResponse};

#[tauri::command]
pub fn get_content(state: State<Mutex<AppState>>) -> ContentResponse {
    state.lock().unwrap().to_response()
}
