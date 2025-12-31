//! Window state persistence for multi-monitor setups.
//!
//! Key insight: macOS uses logical coordinates for window positioning.
//! Previous attempts using PhysicalPosition broke cross-monitor positioning
//! on mixed-DPI setups (e.g., Retina 2x + external 1x).
//!
//! Solution: Use LogicalPosition on macOS, PhysicalPosition elsewhere.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};

#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, LogicalSize};

#[cfg(not(target_os = "macos"))]
use tauri::{PhysicalPosition, PhysicalSize};

use crate::config::config_dir;

/// Window types that persist their state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowType {
    Main,
    Excalidraw,
    Mermaid,
}

impl WindowType {
    fn as_str(&self) -> &'static str {
        match self {
            WindowType::Main => "main",
            WindowType::Excalidraw => "excalidraw",
            WindowType::Mermaid => "mermaid",
        }
    }
}

/// Persisted window geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub maximized: bool,
}

/// All window states, keyed by window type name.
type WindowStates = HashMap<String, WindowState>;

const STATE_FILE: &str = "window-state.json";

/// Save current window state to disk.
///
/// On macOS, converts to logical coordinates before saving.
/// On other platforms, uses physical coordinates.
pub fn save_window_state(window: &WebviewWindow, window_type: WindowType) -> Result<(), String> {
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().unwrap_or(1.0);

    // macOS uses logical coordinates for window positioning
    #[cfg(target_os = "macos")]
    let (x, y, w, h) = {
        let logical_pos = pos.to_logical::<i32>(scale);
        let logical_size = size.to_logical::<u32>(scale);
        (
            logical_pos.x,
            logical_pos.y,
            logical_size.width,
            logical_size.height,
        )
    };

    #[cfg(not(target_os = "macos"))]
    let (x, y, w, h) = (pos.x, pos.y, size.width, size.height);

    let maximized = window.is_maximized().unwrap_or(false);

    let state = WindowState {
        x,
        y,
        width: w,
        height: h,
        maximized,
    };

    save_state(window_type, &state)
}

/// Restore window state from disk.
///
/// Returns `true` if state was restored successfully.
/// Returns `false` if no saved state exists or position is off-screen.
///
/// On macOS, uses logical coordinates for positioning.
/// On other platforms, uses physical coordinates.
pub fn restore_window_state(window: &WebviewWindow, window_type: WindowType) -> bool {
    let Some(state) = load_state(window_type) else {
        return false;
    };

    // Validate position is on a connected monitor
    if !is_position_on_monitor(window.app_handle(), state.x, state.y) {
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window.set_position(LogicalPosition::new(state.x, state.y));
        let _ = window.set_size(LogicalSize::new(state.width, state.height));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
        let _ = window.set_size(PhysicalSize::new(state.width, state.height));
    }

    if state.maximized {
        let _ = window.maximize();
    }

    true
}

/// Check if a position falls within any connected monitor's bounds.
fn is_position_on_monitor(app: &AppHandle, x: i32, y: i32) -> bool {
    // Get all available monitors
    let Ok(monitors) = app.available_monitors() else {
        return false;
    };

    for monitor in monitors {
        let pos = monitor.position();
        let size = monitor.size();

        // Check if (x, y) is within this monitor's bounds
        // We check if the top-left corner of the window is on the monitor
        let monitor_left = pos.x;
        let monitor_top = pos.y;
        let monitor_right = pos.x + size.width as i32;
        let monitor_bottom = pos.y + size.height as i32;

        if x >= monitor_left && x < monitor_right && y >= monitor_top && y < monitor_bottom {
            return true;
        }
    }

    false
}

/// Load state for a specific window type from disk.
fn load_state(window_type: WindowType) -> Option<WindowState> {
    let dir = config_dir()?;
    let path = dir.join(STATE_FILE);

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    let states: WindowStates = serde_json::from_str(&content).ok()?;

    states.get(window_type.as_str()).cloned()
}

/// Save state for a specific window type to disk.
fn save_state(window_type: WindowType, state: &WindowState) -> Result<(), String> {
    let dir = ensure_config_dir().map_err(|e| e.to_string())?;
    let path = dir.join(STATE_FILE);

    // Load existing states or start fresh
    let mut states: WindowStates = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Update state for this window type
    states.insert(window_type.as_str().to_string(), state.clone());

    // Write atomically
    let content = serde_json::to_string_pretty(&states).map_err(|e| e.to_string())?;
    atomic_write(&path, &content).map_err(|e| e.to_string())
}

/// Ensures the config directory exists.
fn ensure_config_dir() -> io::Result<std::path::PathBuf> {
    let dir = config_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine config directory",
        )
    })?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Write to a temp file then rename for atomicity.
fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, content)?;
    fs::rename(&temp, path)?;
    Ok(())
}
