use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::config::{self, Config};
use crate::output::{format_output, OutputMode};
use crate::review::ActiveReview;
use crate::state::{ContentNode, ContentResponse, ExitMode, Tag};
use crate::ShouldExit;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyMode {
    Content,
    Annotations,
    All,
}

/// Helper macro for commands that need mutable access to the review (but not a specific file).
macro_rules! with_review {
    ($review_state:expr, |$review:ident| $body:expr) => {{
        let mut guard = $review_state.lock();
        let $review = guard.as_mut().ok_or("No active review")?;
        $body
    }};
}

#[tauri::command]
pub fn get_content(
    window: WebviewWindow,
    review_state: State<ActiveReview>,
) -> Result<ContentResponse, String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    review
        .to_response_for_window(window.label())
        .ok_or_else(|| "Cannot get content for this window type".into())
}

#[tauri::command]
pub fn upsert_annotation(
    review_state: State<ActiveReview>,
    path: String,
    start_line: u32,
    end_line: u32,
    content: Vec<ContentNode>,
) -> Result<(), String> {
    with_review!(review_state, |review| {
        let target = review.resolve_target_mut(&path)?;
        target.upsert_annotation(start_line, end_line, content);
        Ok(())
    })
}

#[tauri::command]
pub fn delete_annotation(
    review_state: State<ActiveReview>,
    path: String,
    start_line: u32,
    end_line: u32,
) -> Result<(), String> {
    with_review!(review_state, |review| {
        let target = review.resolve_target_mut(&path)?;
        target.delete_annotation(start_line, end_line);
        Ok(())
    })
}

/// Unified finish command - handles both CLI and MCP modes.
#[tauri::command]
pub fn finish_review(
    review_state: State<ActiveReview>,
    should_exit: State<ShouldExit>,
    app: AppHandle,
) -> Result<(), String> {
    let mut guard = review_state.lock();
    let mut review = guard.take().ok_or("No active review")?;

    let is_mcp = review.is_mcp();
    let result = format_output(
        &review,
        if is_mcp {
            OutputMode::Mcp
        } else {
            OutputMode::Cli
        },
    );

    // Close all windows
    let labels: Vec<_> = review.window_labels().map(|s| s.to_string()).collect();
    for label in &labels {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.destroy();
        }
    }

    if let Some(tx) = review.take_result_sender() {
        // MCP mode: send result via channel
        tx.send(result).map_err(|_| "Failed to send result")?;
    } else {
        // CLI mode: print and exit
        if !result.text.is_empty() {
            print!("{}", result.text);
        }
        should_exit.store(true, Ordering::SeqCst);
        app.exit(0);
    }

    Ok(())
}

#[tauri::command]
pub fn set_exit_mode(
    review_state: State<ActiveReview>,
    mode_id: Option<String>,
) -> Result<(), String> {
    with_review!(review_state, |review| {
        review.selected_exit_mode_id = mode_id;
        Ok(())
    })
}

#[tauri::command]
pub fn cycle_exit_mode(
    review_state: State<ActiveReview>,
    direction: i32,
) -> Result<Option<ExitMode>, String> {
    with_review!(review_state, |review| {
        let exit_modes = review.config.exit_modes();
        if exit_modes.is_empty() {
            return Ok(None);
        }

        // Find current index
        let current_index = review
            .selected_exit_mode_id
            .as_ref()
            .and_then(|id| exit_modes.iter().position(|m| &m.id == id))
            .unwrap_or(0);

        // Calculate new index with wrapping
        let len = exit_modes.len() as i32;
        let new_index = ((current_index as i32 + direction) % len + len) % len;

        let new_mode = exit_modes[new_index as usize].clone();
        review.selected_exit_mode_id = Some(new_mode.id.clone());

        Ok(Some(new_mode))
    })
}

#[tauri::command]
pub fn set_session_comment(
    review_state: State<ActiveReview>,
    content: Option<Vec<ContentNode>>,
) -> Result<(), String> {
    with_review!(review_state, |review| {
        review.session_comment = content;
        Ok(())
    })
}

#[tauri::command]
pub fn get_tags(review_state: State<ActiveReview>) -> Result<Vec<Tag>, String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    Ok(review.config.tags().to_vec())
}

#[tauri::command]
pub fn upsert_tag(review_state: State<ActiveReview>, tag: Tag) -> Result<Vec<Tag>, String> {
    with_review!(review_state, |review| {
        review.config.upsert_tag(tag);
        Ok(review.config.tags().to_vec())
    })
}

#[tauri::command]
pub fn delete_tag(review_state: State<ActiveReview>, id: String) -> Result<Vec<Tag>, String> {
    with_review!(review_state, |review| {
        review.config.delete_tag(&id);
        Ok(review.config.tags().to_vec())
    })
}

#[tauri::command]
pub fn get_exit_modes(review_state: State<ActiveReview>) -> Result<Vec<ExitMode>, String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    Ok(review.config.exit_modes().to_vec())
}

#[tauri::command]
pub fn upsert_exit_mode(
    review_state: State<ActiveReview>,
    mode: ExitMode,
) -> Result<Vec<ExitMode>, String> {
    with_review!(review_state, |review| {
        review.config.upsert_exit_mode(mode);
        Ok(review.config.exit_modes().to_vec())
    })
}

#[tauri::command]
pub fn delete_exit_mode(
    review_state: State<ActiveReview>,
    id: String,
) -> Result<Vec<ExitMode>, String> {
    with_review!(review_state, |review| {
        review.config.delete_exit_mode(&id);
        Ok(review.config.exit_modes().to_vec())
    })
}

#[tauri::command]
pub fn reorder_exit_modes(
    review_state: State<ActiveReview>,
    ids: Vec<String>,
) -> Result<Vec<ExitMode>, String> {
    with_review!(review_state, |review| {
        review.config.reorder_exit_modes(ids);
        Ok(review.config.exit_modes().to_vec())
    })
}

#[tauri::command]
pub fn copy_to_clipboard(
    window: WebviewWindow,
    review_state: State<ActiveReview>,
    mode: CopyMode,
) -> Result<(), String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    review.verify_window(window.label())?;

    // Get content from root_view
    let content = review.root_view.content();

    // Reconstruct raw content from lines
    let raw_content: String = content
        .lines
        .iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let text = match mode {
        CopyMode::Content => raw_content,
        CopyMode::Annotations => format_output(review, OutputMode::Clipboard).text,
        CopyMode::All => {
            let annotations = format_output(review, OutputMode::Clipboard).text;
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
    window: WebviewWindow,
    review_state: State<ActiveReview>,
    path: String,
) -> Result<SaveContentResponse, String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    review.verify_window(window.label())?;

    // Get content from root_view
    let content = review.root_view.content();

    // Reconstruct raw content from lines
    let raw_content: String = content
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
    std::fs::write(&path, &raw_content).map_err(|e| format!("Failed to write file: {}", e))?;

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
    window: WebviewWindow,
    review_state: State<ActiveReview>,
    vault_name: String,
) -> Result<ObsidianExportResponse, String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    review.verify_window(window.label())?;

    // Get content from root_view
    let content_model = review.root_view.content();

    // Reconstruct raw content from lines
    let content: String = content_model
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
    let note_name = content_model
        .lines
        .iter()
        .find(|l| l.content.starts_with("# "))
        .map(|l| l.content.trim_start_matches("# ").trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(&content_model.label);

    // Build Obsidian URI with clipboard parameter
    // Sanitize note name to remove characters invalid in filenames (\ / :)
    let sanitized_name = sanitize_obsidian_filename(note_name);
    let url = format!(
        "obsidian://new?vault={}&name={}&clipboard=true",
        urlencoding::encode(&vault_name),
        urlencoding::encode(&sanitized_name)
    );

    Ok(ObsidianExportResponse { url })
}

/// Sanitize a filename for Obsidian by removing characters that are invalid in filenames.
/// Obsidian (and most filesystems) don't allow: \ / :
fn sanitize_obsidian_filename(name: &str) -> String {
    name.chars().filter(|c| !matches!(c, '\\' | '/' | ':')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_obsidian_filename_removes_backslash() {
        assert_eq!(
            sanitize_obsidian_filename(r"Title\with\backslashes"),
            "Titlewithbackslashes"
        );
    }

    #[test]
    fn sanitize_obsidian_filename_removes_forward_slash() {
        assert_eq!(
            sanitize_obsidian_filename("Title/with/slashes"),
            "Titlewithslashes"
        );
    }

    #[test]
    fn sanitize_obsidian_filename_removes_colon() {
        assert_eq!(
            sanitize_obsidian_filename("Title: An example"),
            "Title An example"
        );
    }

    #[test]
    fn sanitize_obsidian_filename_removes_all_special_chars() {
        assert_eq!(
            sanitize_obsidian_filename(r"C:\path/to:file"),
            "Cpathtofile"
        );
    }

    #[test]
    fn sanitize_obsidian_filename_preserves_normal_text() {
        assert_eq!(
            sanitize_obsidian_filename("Normal Title Here"),
            "Normal Title Here"
        );
    }
}
