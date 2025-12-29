use std::collections::HashMap;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WebviewWindowBuilder};
use uuid::Uuid;

/// Reference to a TipTap node being edited.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum NodeRef {
    /// Editing an existing ExcalidrawChip (by its nodeId attribute)
    Chip(String),
    /// Creating new from an ExcalidrawPlaceholder (by its placeholderId attribute)
    Placeholder(String),
}

/// Origin of the Excalidraw window - determines where results are routed.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExcalidrawOrigin {
    /// Opened from TipTap annotation editor (default)
    Annotation,
    /// Opened from a mermaid code block
    CodeBlock { start_line: u32, end_line: u32 },
}

/// Context for an Excalidraw editor window.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExcalidrawContext {
    /// JSON array of Excalidraw elements
    pub elements: String,
    /// Annotation identifier (e.g., "45-52")
    pub range_key: String,
    /// Reference to the TipTap node being edited
    pub node_ref: NodeRef,
    /// Parent window label for emitting results
    pub parent_label: String,
    /// Origin of this window - determines result routing
    pub origin: ExcalidrawOrigin,
}

/// Outcome of an Excalidraw editing session.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExcalidrawOutcome {
    /// User saved the diagram
    Saved { elements: String, png: String },
    /// User cancelled (or closed window without saving)
    Cancelled,
}

/// Result emitted back to the main window (for Annotation origin).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExcalidrawResult {
    /// Annotation identifier
    pub range_key: String,
    /// Reference to the node that was edited
    pub node_ref: NodeRef,
    /// Outcome of the session
    pub outcome: ExcalidrawOutcome,
}

/// Result emitted for CodeBlock origin (mermaid → excalidraw flow).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeBlockExcalidrawResult {
    /// Start line of the mermaid code block
    pub start_line: u32,
    /// End line of the mermaid code block
    pub end_line: u32,
    /// JSON array of Excalidraw elements
    pub elements: String,
    /// Base64 PNG data URL
    pub png: String,
}

/// State tracking open Excalidraw windows.
pub struct ExcalidrawWindowState {
    contexts: HashMap<String, ExcalidrawContext>,
}

impl ExcalidrawWindowState {
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }
}

impl Default for ExcalidrawWindowState {
    fn default() -> Self {
        Self::new()
    }
}

/// Open an Excalidraw editor in a separate window.
#[tauri::command]
pub fn open_excalidraw_window(
    app: AppHandle,
    window: WebviewWindow,
    excalidraw_state: State<Mutex<ExcalidrawWindowState>>,
    elements: String,
    range_key: String,
    node_ref: NodeRef,
    origin: Option<ExcalidrawOrigin>,
) -> Result<String, String> {
    // Each Excalidraw window gets a unique label
    let label = format!("excalidraw-{}", Uuid::new_v4().simple());

    // Get the calling window's label as parent
    let parent_label = window.label().to_string();

    let context = ExcalidrawContext {
        elements,
        range_key,
        node_ref,
        parent_label,
        origin: origin.unwrap_or(ExcalidrawOrigin::Annotation),
    };

    // Calculate window size (75% of screen, min 600x400)
    let (width, height) = if let Some(monitor) = window.current_monitor().ok().flatten() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let logical_width = size.width as f64 / scale;
        let logical_height = size.height as f64 / scale;
        let w = (logical_width * 0.75).max(600.0);
        let h = (logical_height * 0.75).max(400.0);
        (w, h)
    } else {
        (900.0, 700.0) // Fallback
    };

    // Store context before creating window
    {
        let mut state = excalidraw_state.lock();
        state.contexts.insert(label.clone(), context);
    }

    // Create new window (hidden until frontend renders)
    let mut builder = WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("excalidraw".into()),
    )
    .title("Excalidraw")
    .inner_size(width, height)
    .min_inner_size(600.0, 400.0)
    .visible(false)
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .hidden_title(true);

    #[cfg(target_os = "macos")]
    {
        builder = builder.traffic_light_position(tauri::LogicalPosition::new(12.0, 22.0));
    }

    let new_window = builder
        .build()
        .map_err(|e| format!("Failed to create excalidraw window: {}", e))?;

    // Center window on screen
    let _ = new_window.center();

    // Register cleanup on window close
    let label_for_cleanup = label.clone();
    let app_for_cleanup = app.clone();

    new_window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            // Get state from app handle
            let state: tauri::State<Mutex<ExcalidrawWindowState>> = app_for_cleanup.state();
            let mut state = state.lock();
            // If context still exists, user closed without save/cancel - treat as cancel
            if let Some(ctx) = state.contexts.remove(&label_for_cleanup) {
                // Only emit cancel for Annotation origin (TipTap needs to clean up placeholder)
                // CodeBlock origin doesn't need cancel notification - nothing to clean up
                if matches!(ctx.origin, ExcalidrawOrigin::Annotation) {
                    if let Some(parent) = app_for_cleanup.get_webview_window(&ctx.parent_label) {
                        let _ = parent.emit(
                            "excalidraw-result",
                            ExcalidrawResult {
                                range_key: ctx.range_key,
                                node_ref: ctx.node_ref,
                                outcome: ExcalidrawOutcome::Cancelled,
                            },
                        );
                    }
                }
            }
        }
    });

    Ok(label)
}

/// Get Excalidraw context for a child window.
#[tauri::command]
pub fn get_excalidraw_context(
    window: WebviewWindow,
    excalidraw_state: State<Mutex<ExcalidrawWindowState>>,
) -> Result<ExcalidrawContext, String> {
    let state = excalidraw_state.lock();
    let label = window.label();

    state
        .contexts
        .get(label)
        .cloned()
        .ok_or_else(|| format!("No context found for window: {}", label))
}

/// Save Excalidraw diagram and close window.
#[tauri::command]
pub fn excalidraw_save(
    app: AppHandle,
    window: WebviewWindow,
    excalidraw_state: State<Mutex<ExcalidrawWindowState>>,
    elements: String,
    png: String,
) -> Result<(), String> {
    let label = window.label().to_string();

    // Remove context and get parent info
    let ctx = {
        let mut state = excalidraw_state.lock();
        state
            .contexts
            .remove(&label)
            .ok_or_else(|| format!("No context found for window: {}", label))?
    };

    // Emit result to parent window based on origin
    if let Some(parent) = app.get_webview_window(&ctx.parent_label) {
        match ctx.origin {
            ExcalidrawOrigin::Annotation => {
                parent
                    .emit(
                        "excalidraw-result",
                        ExcalidrawResult {
                            range_key: ctx.range_key,
                            node_ref: ctx.node_ref,
                            outcome: ExcalidrawOutcome::Saved {
                                elements,
                                png,
                            },
                        },
                    )
                    .map_err(|e| format!("Failed to emit result: {}", e))?;
            }
            ExcalidrawOrigin::CodeBlock { start_line, end_line } => {
                parent
                    .emit(
                        "codeblock-excalidraw-result",
                        CodeBlockExcalidrawResult {
                            start_line,
                            end_line,
                            elements,
                            png,
                        },
                    )
                    .map_err(|e| format!("Failed to emit result: {}", e))?;
            }
        }
    }

    // Close the window
    window
        .close()
        .map_err(|e| format!("Failed to close window: {}", e))?;

    Ok(())
}

/// Cancel Excalidraw editing and close window.
#[tauri::command]
pub fn excalidraw_cancel(
    app: AppHandle,
    window: WebviewWindow,
    excalidraw_state: State<Mutex<ExcalidrawWindowState>>,
) -> Result<(), String> {
    let label = window.label().to_string();

    // Remove context and get parent info
    let ctx = {
        let mut state = excalidraw_state.lock();
        state
            .contexts
            .remove(&label)
            .ok_or_else(|| format!("No context found for window: {}", label))?
    };

    // Emit cancel result to parent window
    if let Some(parent) = app.get_webview_window(&ctx.parent_label) {
        parent
            .emit(
                "excalidraw-result",
                ExcalidrawResult {
                    range_key: ctx.range_key,
                    node_ref: ctx.node_ref,
                    outcome: ExcalidrawOutcome::Cancelled,
                },
            )
            .map_err(|e| format!("Failed to emit result: {}", e))?;
    }

    // Close the window
    window
        .close()
        .map_err(|e| format!("Failed to close window: {}", e))?;

    Ok(())
}
