use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::config::{self, Config, Theme};
use crate::lang::extension_to_fence_language;
use crate::output::{export_content, export_section, format_output, OutputMode};
use crate::review::ActiveReview;
use crate::input::{ContentSource, McpSource};
use crate::state::{
    Bookmark, BookmarkSnapshot, ContentMetadata, ContentNode, ContentResponse, ExitMode,
    SessionType, Tag, TagUsageStats,
};

/// Snapshot of config data for reload_config command.
#[derive(Serialize)]
pub struct ConfigSnapshot {
    pub tags: Vec<Tag>,
    pub exit_modes: Vec<ExitMode>,
    pub bookmarks: Vec<Bookmark>,
}
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

    // Collect tag usage stats from all annotations
    collect_tag_usage(&mut review);

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
            let _ = std::io::stdout().flush();
        }
        should_exit.store(true, Ordering::SeqCst);
        app.exit(0);
    }

    Ok(())
}

/// Collect tag usage from all annotations and save to disk.
fn collect_tag_usage(review: &mut crate::review::Review) {
    let mut session_stats = TagUsageStats::default();

    // Walk all annotation targets
    for (_file_key, target) in &review.files {
        // Get language for this file from metadata
        let language = target
            .metadata
            .language
            .as_ref()
            .map(|ext| extension_to_fence_language(ext))
            .filter(|s| !s.is_empty());

        // Walk all annotations
        for annotation in target.annotations.values() {
            // Extract tag IDs from content nodes
            for node in &annotation.content {
                if let ContentNode::Tag { id, .. } = node {
                    session_stats.increment(id, language);
                }
            }
        }
    }

    // Also check session comment for tags (no language context)
    if let Some(ref comment) = review.session_comment {
        for node in comment {
            if let ContentNode::Tag { id, .. } = node {
                session_stats.increment(id, None);
            }
        }
    }

    // Only save if we have stats to record
    if !session_stats.tags.is_empty() {
        // Update config and save
        *review.config.usage_stats_mut() = session_stats;
        review.config.save_usage_stats();
    }
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

// --- Bookmark commands ---

#[tauri::command]
pub fn get_bookmarks(review_state: State<ActiveReview>) -> Result<Vec<Bookmark>, String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    Ok(review.config.bookmarks().to_vec())
}

/// Reload config from disk, merging new items while preserving in-session edits.
/// Called on window focus to pick up changes from other annot windows.
#[tauri::command]
pub fn reload_config(review_state: State<ActiveReview>) -> Result<ConfigSnapshot, String> {
    let mut guard = review_state.lock();
    let review = guard.as_mut().ok_or("No active review")?;

    review.config.reload_from_disk();

    Ok(ConfigSnapshot {
        tags: review.config.tags().to_vec(),
        exit_modes: review.config.exit_modes().to_vec(),
        bookmarks: review.config.bookmarks().to_vec(),
    })
}

#[tauri::command]
pub fn create_bookmark(
    review_state: State<ActiveReview>,
    label: Option<String>,
) -> Result<Bookmark, String> {
    with_review!(review_state, |review| {
        // Determine session type from the content source
        let source = &review.root_view.content().source;
        let source_type = match source {
            ContentSource::Mcp(McpSource::Diff { .. }) => SessionType::Diff,
            ContentSource::Mcp(McpSource::Content { .. }) => SessionType::Content,
            _ => SessionType::File,
        };

        // Get source title (label) and full content
        let source_title = review.root_view.content().label.clone();
        let context = review
            .root_view
            .content()
            .lines
            .iter()
            .map(|l| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let snapshot = BookmarkSnapshot::Session {
            source_type,
            source_title,
            context,
        };

        // Auto-derive label from H1 heading for markdown content if not provided
        let label = label.or_else(|| {
            if let ContentMetadata::Markdown(md) = &review.root_view.content().metadata {
                md.sections.iter().find(|s| s.level == 1).map(|s| s.title.clone())
            } else {
                None
            }
        });

        // Get current working directory as project path
        let project_path = std::env::current_dir().ok();

        let bookmark = Bookmark::new(label, project_path, snapshot);
        review.config.upsert_bookmark(bookmark.clone());
        review.session_created_bookmarks.insert(bookmark.id.clone());

        Ok(bookmark)
    })
}

#[tauri::command]
pub fn create_selection_bookmark(
    review_state: State<ActiveReview>,
    start_line: usize,
    end_line: usize,
    label: Option<String>,
) -> Result<Bookmark, String> {
    with_review!(review_state, |review| {
        // Determine session type from the content source
        let source = &review.root_view.content().source;
        let source_type = match source {
            ContentSource::Mcp(McpSource::Diff { .. }) => SessionType::Diff,
            ContentSource::Mcp(McpSource::Content { .. }) => SessionType::Content,
            _ => SessionType::File,
        };

        let lines = &review.root_view.content().lines;

        // Extract selected text (display indices are 1-indexed)
        let selected_text = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i + 1 >= start_line && *i + 1 <= end_line)
            .map(|(_, l)| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Get source title and full context
        let source_title = review.root_view.content().label.clone();
        let context = lines
            .iter()
            .map(|l| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let snapshot = BookmarkSnapshot::Selection {
            source_type,
            source_title,
            context,
            selected_text,
        };

        let project_path = std::env::current_dir().ok();
        let bookmark = Bookmark::new(label, project_path, snapshot);
        review.config.upsert_bookmark(bookmark.clone());
        review.session_created_bookmarks.insert(bookmark.id.clone());

        Ok(bookmark)
    })
}

#[tauri::command]
pub fn update_bookmark(
    review_state: State<ActiveReview>,
    id: String,
    label: String,
) -> Result<Bookmark, String> {
    with_review!(review_state, |review| {
        let bookmark = review
            .config
            .get_bookmark(&id)
            .ok_or_else(|| format!("Bookmark not found: {}", id))?
            .clone();

        let updated = Bookmark {
            label: Some(label),
            ..bookmark
        };
        review.config.upsert_bookmark(updated.clone());

        Ok(updated)
    })
}

#[tauri::command]
pub fn delete_bookmark(
    review_state: State<ActiveReview>,
    id: String,
) -> Result<Vec<Bookmark>, String> {
    with_review!(review_state, |review| {
        if !review.config.delete_bookmark(&id) {
            return Err(format!("Bookmark not found: {}", id));
        }
        Ok(review.config.bookmarks().to_vec())
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

    // Get content from root_view and export with portals embedded as code blocks
    let content = review.root_view.content();
    let raw_content = export_content(content);

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

#[tauri::command]
pub fn copy_section(
    window: WebviewWindow,
    review_state: State<ActiveReview>,
    start_line: u32,
    end_line: u32,
) -> Result<(), String> {
    let guard = review_state.lock();
    let review = guard.as_ref().ok_or("No active review")?;
    review.verify_window(window.label())?;

    let content = review.root_view.content();
    let section_text = export_section(content, start_line, end_line);

    if section_text.is_empty() {
        return Err("Section is empty".to_string());
    }

    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(section_text))
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
    let mut guard = review_state.lock();
    let review = guard.as_mut().ok_or("No active review")?;
    review.verify_window(window.label())?;

    // Get content from root_view and export with portals embedded as code blocks
    let content = review.root_view.content();
    let raw_content = export_content(content);

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

    // Track that we saved (for session output)
    review.saved_to = Some(path.clone());

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

    // Get content from root_view and export with portals embedded as code blocks
    let content_model = review.root_view.content();
    let content = export_content(content_model);

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

// --- Replace diff (word-level) ---

/// A span within a diff line, with emphasis marking changed words.
#[derive(Serialize, Debug, PartialEq)]
pub struct DiffSpan {
    pub text: String,
    pub emphasized: bool,
}

/// A line in the replace diff output.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ReplaceDiffLine {
    Equal { spans: Vec<DiffSpan> },
    Insert { spans: Vec<DiffSpan> },
    Delete { spans: Vec<DiffSpan> },
}

/// Compute a word-level diff between original and replacement text.
/// Returns interleaved lines: each changed line pair shows delete then insert.
pub fn compute_word_diff(original: &str, replacement: &str) -> Vec<ReplaceDiffLine> {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(original, replacement);
    let mut result = Vec::new();

    for op in diff.ops() {
        for change in diff.iter_inline_changes(op) {
            let spans: Vec<DiffSpan> = change
                .iter_strings_lossy()
                .map(|(emphasized, text)| DiffSpan {
                    text: text.to_string(),
                    emphasized,
                })
                .collect();

            let line = match change.tag() {
                ChangeTag::Equal => ReplaceDiffLine::Equal { spans },
                ChangeTag::Insert => ReplaceDiffLine::Insert { spans },
                ChangeTag::Delete => ReplaceDiffLine::Delete { spans },
            };
            result.push(line);
        }
    }

    result
}

/// Tauri command wrapper for compute_word_diff.
#[tauri::command]
pub fn compute_replace_diff(original: String, replacement: String) -> Vec<ReplaceDiffLine> {
    compute_word_diff(&original, &replacement)
}

// --- Theme commands ---

#[tauri::command]
pub fn get_theme() -> Theme {
    config::load_config().theme
}

#[tauri::command]
pub fn set_theme(theme: Theme) -> Result<(), String> {
    let mut cfg = config::load_config();
    cfg.theme = theme;
    config::save_config(&cfg).map_err(|e| e.to_string())
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

    #[test]
    fn compute_word_diff_identical_returns_equal() {
        let result = compute_word_diff("hello\n", "hello\n");
        assert_eq!(
            result,
            vec![ReplaceDiffLine::Equal {
                spans: vec![DiffSpan {
                    text: "hello\n".to_string(),
                    emphasized: false
                }]
            }]
        );
    }

    #[test]
    fn compute_word_diff_word_change_emphasizes_changed_words() {
        let result = compute_word_diff("The quick fox\n", "The slow fox\n");
        // Should have delete then insert (interleaved)
        assert_eq!(result.len(), 2);

        // First line: delete with "quick" emphasized
        match &result[0] {
            ReplaceDiffLine::Delete { spans } => {
                assert!(spans.iter().any(|s| s.text == "quick" && s.emphasized));
                assert!(spans.iter().any(|s| s.text == "The " && !s.emphasized));
            }
            _ => panic!("Expected Delete, got {:?}", result[0]),
        }

        // Second line: insert with "slow" emphasized
        match &result[1] {
            ReplaceDiffLine::Insert { spans } => {
                assert!(spans.iter().any(|s| s.text == "slow" && s.emphasized));
                assert!(spans.iter().any(|s| s.text == "The " && !s.emphasized));
            }
            _ => panic!("Expected Insert, got {:?}", result[1]),
        }
    }

    #[test]
    fn compute_word_diff_multiline_preserves_unchanged() {
        let result = compute_word_diff("line one\nline two\n", "line one\nline TWO\n");
        // First line unchanged (equal), second line changed (delete + insert)
        assert_eq!(result.len(), 3);

        match &result[0] {
            ReplaceDiffLine::Equal { .. } => {}
            _ => panic!("Expected Equal for first line"),
        }
        match &result[1] {
            ReplaceDiffLine::Delete { .. } => {}
            _ => panic!("Expected Delete for changed line"),
        }
        match &result[2] {
            ReplaceDiffLine::Insert { .. } => {}
            _ => panic!("Expected Insert for changed line"),
        }
    }
}
