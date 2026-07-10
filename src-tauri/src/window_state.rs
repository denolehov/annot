//! Window state persistence for multi-monitor setups.
//!
//! Geometry is keyed by *display configuration* — a fingerprint of the
//! currently-connected monitors (a sorted multiset of `WxH@scale`). Each
//! configuration keeps its own per-window-type slot, so disconnecting a
//! monitor and later reconnecting it restores windows to where they were
//! on that setup, without the in-between (fallback) layout clobbering it.
//!
//! macOS uses logical coordinates for window positioning; physical
//! coordinates broke cross-monitor positioning on mixed-DPI setups. All
//! position comparisons happen in logical space on macOS, physical elsewhere.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Monitor, WebviewWindow};

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

/// Per-display-configuration slot: geometry keyed by window type name.
type ConfigSlot = HashMap<String, WindowState>;

/// On-disk state file (v2): geometry keyed by display-configuration id.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateFile {
    /// Format version. Required — its absence is how a v1 (flat) file is detected.
    version: u32,
    #[serde(default)]
    configs: HashMap<String, ConfigSlot>,
}

impl Default for StateFile {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            configs: HashMap::new(),
        }
    }
}

const STATE_FILE: &str = "window-state.json";
const STATE_VERSION: u32 = 2;

// ════════════════════════════════════════════════════════════════════════════
// DISPLAY CONFIGURATION FINGERPRINT
// ════════════════════════════════════════════════════════════════════════════

/// Fingerprint one monitor as `WxH@scale` (physical size + scale factor).
///
/// Tauri exposes no stable, unique, *persistable* monitor identity: `name()` is
/// the model number (identical monitors collide), `CGDirectDisplayID` is not
/// exposed and is unstable anyway, and the stable UUID is not exposed at all.
/// Geometry is the best observable signal — and a multiset of it naturally
/// distinguishes "two identical monitors" from "one".
fn monitor_key(width: u32, height: u32, scale: f64) -> String {
    format!("{width}x{height}@{scale}")
}

/// Combine monitor keys into an order-independent configuration id.
fn config_id_from(mut keys: Vec<String>) -> String {
    keys.sort();
    keys.join(";")
}

/// Fingerprint of the currently-connected monitors. `None` if no monitors are
/// reported (can't meaningfully key geometry).
fn display_config_id(app: &AppHandle) -> Option<String> {
    let monitors = app.available_monitors().ok()?;
    if monitors.is_empty() {
        return None;
    }
    let keys = monitors
        .iter()
        .map(|m| monitor_key(m.size().width, m.size().height, m.scale_factor()))
        .collect();
    Some(config_id_from(keys))
}

// ════════════════════════════════════════════════════════════════════════════
// SAVE / RESTORE
// ════════════════════════════════════════════════════════════════════════════

/// Save current window state to disk, under the current display configuration.
///
/// On macOS, converts to logical coordinates before saving.
/// On other platforms, uses physical coordinates.
pub fn save_window_state(window: &WebviewWindow, window_type: WindowType) -> Result<(), String> {
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
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

    save_state(window.app_handle(), window_type, &state)
}

/// Restore window state from disk.
///
/// Returns `true` if state was restored successfully.
/// Returns `false` if no saved state exists for the current display
/// configuration, or the saved position is off-screen.
///
/// On macOS, uses logical coordinates for positioning.
/// On other platforms, uses physical coordinates.
pub fn restore_window_state(window: &WebviewWindow, window_type: WindowType) -> bool {
    let app = window.app_handle();

    let Some(state) = load_state(app, window_type) else {
        return false;
    };

    // Backstop: a matched config slot can still hold stale coordinates if the
    // user rearranged monitors (position is excluded from the fingerprint).
    if !is_position_on_monitor(app, state.x, state.y) {
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

// ════════════════════════════════════════════════════════════════════════════
// MONITOR BOUNDS CHECK
// ════════════════════════════════════════════════════════════════════════════

/// Convert a monitor's physical bounds into the coordinate space used for
/// position comparison: logical on macOS, physical elsewhere.
///
/// `monitor.position()` / `monitor.size()` are physical pixels; the saved
/// `(x, y)` is logical on macOS. Comparing the two spaces directly was the
/// original bug — an off-screen logical x like 2964 passed against a physical
/// width of 3024 even though the monitor is only 1512 logical pixels wide.
fn monitor_compare_bounds(
    pos_x: i32,
    pos_y: i32,
    width: u32,
    height: u32,
    scale: f64,
) -> (i32, i32, i32, i32) {
    #[cfg(target_os = "macos")]
    {
        let left = (pos_x as f64 / scale).round() as i32;
        let top = (pos_y as f64 / scale).round() as i32;
        let right = left + (width as f64 / scale).round() as i32;
        let bottom = top + (height as f64 / scale).round() as i32;
        (left, top, right, bottom)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = scale;
        (pos_x, pos_y, pos_x + width as i32, pos_y + height as i32)
    }
}

/// True if `(x, y)` lies within `[left, right) x [top, bottom)`.
fn point_in_rect(x: i32, y: i32, left: i32, top: i32, right: i32, bottom: i32) -> bool {
    x >= left && x < right && y >= top && y < bottom
}

/// Check if a position (logical on macOS, physical elsewhere) falls within any
/// connected monitor's bounds.
fn is_position_on_monitor(app: &AppHandle, x: i32, y: i32) -> bool {
    let Ok(monitors) = app.available_monitors() else {
        return false;
    };

    monitors.iter().any(|monitor| {
        let pos = monitor.position();
        let size = monitor.size();
        let (left, top, right, bottom) = monitor_compare_bounds(
            pos.x,
            pos.y,
            size.width,
            size.height,
            monitor.scale_factor(),
        );
        point_in_rect(x, y, left, top, right, bottom)
    })
}

// ════════════════════════════════════════════════════════════════════════════
// CONTENT-SIZED WINDOW PLACEMENT
// ════════════════════════════════════════════════════════════════════════════

/// Size a content-sized window (e.g. a mermaid diagram viewer) to the given
/// logical size and center it on the monitor where this window type was last
/// seen — or the primary monitor if there's no usable memory.
///
/// Unlike [`restore_window_state`], this never restores an exact position:
/// content-sized windows always fit their content and re-center; only the
/// *monitor* is remembered (derived from the last-saved position).
pub fn place_content_window(
    window: &WebviewWindow,
    window_type: WindowType,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let app = window.app_handle();

    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;

    let target = saved_monitor(app, window_type).or_else(|| app.primary_monitor().ok().flatten());

    match target {
        Some(monitor) => {
            let scale = monitor.scale_factor();
            let mpos = monitor.position();
            let msize = monitor.size();
            let (x, y) = center_in(
                mpos.x as f64 / scale,
                mpos.y as f64 / scale,
                msize.width as f64 / scale,
                msize.height as f64 / scale,
                width,
                height,
            );
            window
                .set_position(LogicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
        }
        None => {
            let _ = window.center();
        }
    }

    Ok(())
}

/// The monitor that contained this window type's last-saved position, if it is
/// still connected.
fn saved_monitor(app: &AppHandle, window_type: WindowType) -> Option<Monitor> {
    let state = load_state(app, window_type)?;
    let monitors = app.available_monitors().ok()?;
    monitors.into_iter().find(|monitor| {
        let pos = monitor.position();
        let size = monitor.size();
        let (left, top, right, bottom) = monitor_compare_bounds(
            pos.x,
            pos.y,
            size.width,
            size.height,
            monitor.scale_factor(),
        );
        point_in_rect(state.x, state.y, left, top, right, bottom)
    })
}

/// Logical top-left position to center a `width` x `height` window within a
/// monitor whose logical bounds start at `(mx, my)` and span `mw` x `mh`.
fn center_in(mx: f64, my: f64, mw: f64, mh: f64, width: f64, height: f64) -> (f64, f64) {
    (mx + (mw - width) / 2.0, my + (mh - height) / 2.0)
}

// ════════════════════════════════════════════════════════════════════════════
// STATE FILE I/O
// ════════════════════════════════════════════════════════════════════════════

/// Path to the state file, if the config directory can be determined.
fn state_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(STATE_FILE))
}

/// Parse file content as a v2 `StateFile`, migrating the v1 flat format
/// (`{ "main": {...}, ... }`) under `current_config_id` if that's what's found.
///
/// Pure (no I/O) so the migration path is unit-testable. Corrupt or empty
/// content yields a default (empty) `StateFile` — never an error.
fn parse_state_file(content: &str, current_config_id: Option<&str>) -> StateFile {
    // v2: has the `version` field.
    if let Ok(file) = serde_json::from_str::<StateFile>(content) {
        return file;
    }

    // v1: flat map of window-type -> geometry. Adopt it under the current
    // display configuration (the friendliest assumption: the last-known
    // geometry belongs to whatever setup the user is on right now).
    if let Ok(flat) = serde_json::from_str::<ConfigSlot>(content) {
        let mut file = StateFile::default();
        if let Some(id) = current_config_id {
            file.configs.insert(id.to_string(), flat);
        }
        return file;
    }

    StateFile::default()
}

/// Load the whole state file, migrating v1 -> v2 on disk if needed.
fn load_state_file(app: &AppHandle) -> StateFile {
    let Some(path) = state_file_path() else {
        return StateFile::default();
    };
    if !path.exists() {
        return StateFile::default();
    }
    let Ok(content) = fs::read_to_string(&path) else {
        return StateFile::default();
    };

    let was_v2 = serde_json::from_str::<StateFile>(&content).is_ok();
    let config_id = display_config_id(app);
    let file = parse_state_file(&content, config_id.as_deref());

    // If we migrated a v1 file (and had a config to key it under), persist v2.
    if !was_v2 && !file.configs.is_empty() {
        let _ = save_state_file(&file);
    }

    file
}

/// Write the whole state file atomically.
fn save_state_file(file: &StateFile) -> Result<(), String> {
    let dir = ensure_config_dir().map_err(|e| e.to_string())?;
    let path = dir.join(STATE_FILE);
    let content = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    crate::config::atomic_write(&path, &content).map_err(|e| e.to_string())
}

/// Load state for a window type under the current display configuration.
fn load_state(app: &AppHandle, window_type: WindowType) -> Option<WindowState> {
    let config_id = display_config_id(app)?;
    let file = load_state_file(app);
    file.configs
        .get(&config_id)?
        .get(window_type.as_str())
        .cloned()
}

/// Save state for a window type under the current display configuration.
fn save_state(app: &AppHandle, window_type: WindowType, state: &WindowState) -> Result<(), String> {
    let config_id = display_config_id(app)
        .ok_or_else(|| "no connected monitors to key state under".to_string())?;

    let mut file = load_state_file(app);
    file.configs
        .entry(config_id)
        .or_default()
        .insert(window_type.as_str().to_string(), state.clone());

    save_state_file(&file)
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

// ════════════════════════════════════════════════════════════════════════════
// TESTS
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(x: i32) -> WindowState {
        WindowState {
            x,
            y: 0,
            width: 100,
            height: 100,
            maximized: false,
        }
    }

    #[test]
    fn monitor_key_format() {
        assert_eq!(monitor_key(1512, 982, 2.0), "1512x982@2");
        assert_eq!(monitor_key(1920, 1080, 1.0), "1920x1080@1");
    }

    #[test]
    fn config_id_is_order_independent() {
        let a = config_id_from(vec!["1920x1080@1".into(), "1512x982@2".into()]);
        let b = config_id_from(vec!["1512x982@2".into(), "1920x1080@1".into()]);
        assert_eq!(a, b);
        assert_eq!(a, "1512x982@2;1920x1080@1");
    }

    #[test]
    fn config_id_keeps_identical_monitors_distinct_from_one() {
        // Two identical externals form a multiset with two equal entries —
        // still distinguishable from a single-monitor configuration.
        let two = config_id_from(vec!["1920x1080@1".into(), "1920x1080@1".into()]);
        let one = config_id_from(vec!["1920x1080@1".into()]);
        assert_ne!(two, one);
        assert_eq!(two, "1920x1080@1;1920x1080@1");
    }

    #[test]
    fn parse_v2_passthrough() {
        let json = r#"{"version":2,"configs":{"cfg":{"main":{"x":5,"y":0,"width":100,"height":100,"maximized":false}}}}"#;
        let file = parse_state_file(json, None);
        assert_eq!(file.version, 2);
        assert_eq!(file.configs["cfg"]["main"].x, 5);
    }

    #[test]
    fn parse_v1_migrates_under_current_config() {
        // Old flat format — what existing users have on disk today.
        let json = r#"{"main":{"x":7,"y":0,"width":100,"height":100,"maximized":false}}"#;
        let file = parse_state_file(json, Some("current-cfg"));
        assert_eq!(file.version, STATE_VERSION);
        assert_eq!(file.configs["current-cfg"]["main"].x, 7);
    }

    #[test]
    fn parse_v1_without_config_id_drops_gracefully() {
        // No monitors reported -> nothing to key the migrated geometry under.
        let json = r#"{"main":{"x":7,"y":0,"width":100,"height":100,"maximized":false}}"#;
        let file = parse_state_file(json, None);
        assert!(file.configs.is_empty());
        assert_eq!(file.version, STATE_VERSION);
    }

    #[test]
    fn parse_corrupt_returns_default() {
        let file = parse_state_file("not json at all", Some("cfg"));
        assert!(file.configs.is_empty());
        assert_eq!(file.version, STATE_VERSION);
    }

    #[test]
    fn distinct_configs_do_not_clobber() {
        // The core of the feature: a fallback layout under one config must not
        // overwrite the geometry remembered for another.
        let mut file = StateFile::default();
        file.configs
            .entry("cfg-A".into())
            .or_default()
            .insert("main".into(), ws(1));
        file.configs
            .entry("cfg-B".into())
            .or_default()
            .insert("main".into(), ws(2));
        assert_eq!(file.configs["cfg-A"]["main"].x, 1);
        assert_eq!(file.configs["cfg-B"]["main"].x, 2);
    }

    #[test]
    fn center_in_centers() {
        // 1000x700 window on a 1512x982 monitor at the origin.
        assert_eq!(
            center_in(0.0, 0.0, 1512.0, 982.0, 1000.0, 700.0),
            (256.0, 141.0)
        );
        // Same window on a monitor offset to logical x=1512.
        assert_eq!(
            center_in(1512.0, 0.0, 2560.0, 1440.0, 1000.0, 700.0),
            (2292.0, 370.0)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn position_check_uses_logical_bounds() {
        // Retina primary: physical 3024x1964 @ scale 2 -> logical 1512x982.
        let (l, t, r, b) = monitor_compare_bounds(0, 0, 3024, 1964, 2.0);
        assert_eq!((l, t, r, b), (0, 0, 1512, 982));

        // x=2964 is off-screen logically (> 1512). The old check compared it
        // against the physical width 3024 and wrongly accepted it.
        assert!(!point_in_rect(2964, 376, l, t, r, b));

        // A genuinely on-screen logical point still passes.
        assert!(point_in_rect(256, 80, l, t, r, b));
    }
}
