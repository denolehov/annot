use std::collections::HashMap;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State, WebviewWindow, WebviewWindowBuilder};

/// Context for a mermaid diagram window.
#[derive(Clone, Serialize, Deserialize)]
pub struct MermaidContext {
    pub source: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// State tracking open mermaid windows.
/// Maps window hash -> (window_label, context)
pub struct MermaidWindowState {
    windows: HashMap<String, (String, MermaidContext)>,
}

impl MermaidWindowState {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }
}

/// Generate a hash for deduplication based on file path and line range.
fn context_hash(file_path: &str, start_line: u32, end_line: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}-{}", file_path, start_line, end_line));
    let result = hasher.finalize();
    // Take first 8 chars of hex
    hex::encode(&result[..4])
}

/// Open a mermaid diagram in a separate window, or focus existing.
#[tauri::command]
pub fn open_mermaid_window(
    app: AppHandle,
    mermaid_state: State<Mutex<MermaidWindowState>>,
    source: String,
    file_path: String,
    start_line: u32,
    end_line: u32,
) -> Result<String, String> {
    let hash = context_hash(&file_path, start_line, end_line);
    let label = format!("mermaid-{}", hash);

    let mut state = mermaid_state.lock();

    // Check if window already exists
    if let Some((existing_label, _)) = state.windows.get(&hash) {
        // Try to focus existing window
        if let Some(existing_window) = app.get_webview_window(existing_label) {
            let _ = existing_window.set_focus();
            return Ok(existing_label.clone());
        }
        // Window was closed, remove from state
        state.windows.remove(&hash);
    }

    // Extract just the filename for the title
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&file_path);

    let context = MermaidContext {
        source,
        file_path: file_path.clone(),
        start_line,
        end_line,
    };

    // Store context before creating window
    state.windows.insert(hash.clone(), (label.clone(), context));

    // Create new window (hidden until frontend sizes it)
    // Note: We don't use .parent() because macOS child windows can't be
    // dragged to other displays. Instead, mermaid windows are independent.
    let mut builder = WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("mermaid".into()),
    )
    .title(format!("{}:{}-{}", filename, start_line, end_line))
    .inner_size(600.0, 500.0)
    .min_inner_size(300.0, 200.0)
    .visible(false)
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .hidden_title(true);

    #[cfg(target_os = "macos")]
    {
        builder = builder.traffic_light_position(tauri::LogicalPosition::new(12.0, 22.0));
    }

    let _new_window = builder
        .build()
        .map_err(|e| format!("Failed to create mermaid window: {}", e))?;

    Ok(label)
}

/// Get mermaid source for a child window.
#[tauri::command]
pub fn get_mermaid_source(
    window: WebviewWindow,
    mermaid_state: State<Mutex<MermaidWindowState>>,
) -> Result<MermaidContext, String> {
    let state = mermaid_state.lock();

    // Find context by window label
    let window_label = window.label();
    for (_, (label, context)) in state.windows.iter() {
        if label == window_label {
            return Ok(context.clone());
        }
    }

    Err(format!("No context found for window: {}", window_label))
}
