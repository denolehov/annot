//! Review abstraction for state management.
//!
//! A Review represents an active annotation task. It owns:
//! - Content (root_view with the document being reviewed)
//! - Annotation targets (files that can receive annotations)
//! - Windows (how content is displayed)
//! - Session-level state (comment, exit mode, result channel)
//!
//! Content and annotations are orthogonal:
//! - Content lives in `View` (root_view field)
//! - Annotations live on `AnnotationTarget` (files map)
//! - A window is a viewport that can display content
//! - Two windows showing the same file share annotations

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use indexmap::IndexMap;
use serde::Serialize;

use crate::anchor::{Anchor, Annotation};
use crate::output::FormatResult;
use crate::state::{ContentModel, ContentNode, ContentResponse, FileMetadata, UserConfig};

/// Key for annotation targets in Review.files.
/// Distinguishes real file paths from ephemeral/synthetic content.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileKey {
    /// A real file path.
    Path(PathBuf),
    /// A file within a diff, identified by index.
    DiffFile { index: usize },
    /// Ephemeral content (MCP review_content, stdin pipe).
    Ephemeral { label: String },
}

impl FileKey {
    /// Create a key for a real file path.
    pub fn path(p: impl Into<PathBuf>) -> Self {
        FileKey::Path(p.into())
    }

    /// Create a key for a diff file by index.
    pub fn diff_file(index: usize) -> Self {
        FileKey::DiffFile { index }
    }

    /// Create a key for ephemeral content.
    pub fn ephemeral(label: impl Into<String>) -> Self {
        FileKey::Ephemeral {
            label: label.into(),
        }
    }

    /// Get the routing path string for this key.
    /// This is stored in LineOrigin.path and used for annotation routing.
    pub fn routing_path(&self) -> String {
        match self {
            FileKey::Path(p) => p.to_string_lossy().to_string(),
            FileKey::Ephemeral { label } => label.clone(),
            FileKey::DiffFile { .. } => {
                // Diff files use index-based routing, not path-based
                unreachable!("DiffFile routes by index into the diff view's documents")
            }
        }
    }
}

impl fmt::Display for FileKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileKey::Path(p) => write!(f, "{}", p.display()),
            FileKey::DiffFile { index } => write!(f, "diff file {}", index),
            FileKey::Ephemeral { label } => write!(f, "{}", label),
        }
    }
}

/// An active review. Wrapped in `Option`: `Some` = active, `None` = idle.
pub struct Review {
    //--- Content (what exists) ---
    /// The root view — what content is being reviewed.
    /// Content lives here, separate from annotation storage.
    pub root_view: View,
    /// Annotation targets keyed by FileKey.
    pub files: HashMap<FileKey, AnnotationTarget>,

    //--- Windows (how content is displayed) ---
    /// Root window label - review lifecycle is tied to this window.
    pub root_window: String,
    /// All windows and what they're showing.
    pub windows: HashMap<String, WindowView>,

    //--- Session-level state ---
    /// Directory this review is rooted at.
    ///
    /// Everything scoped to *the thing under review* resolves from here: the
    /// diff's repository, the file picker's tree, `:save`'s relative paths.
    /// Deliberately not the root for `.claude/commands` — those exit modes are
    /// handed back to the *calling agent*, whose project is its own launch
    /// directory, not whatever repo we happen to be showing.
    pub root: PathBuf,
    /// Session-level comment (not tied to specific lines/files).
    pub session_comment: Option<Vec<ContentNode>>,
    /// Currently selected exit mode ID.
    pub selected_exit_mode_id: Option<String>,
    /// User configuration (tags, exit modes).
    pub config: UserConfig,

    //--- Result delivery ---
    /// Channel to send result when review ends. `None` for CLI mode.
    result_channel: Option<Sender<FormatResult>>,

    //--- Save tracking ---
    /// Path where content was saved (if user saved during session).
    pub saved_to: Option<PathBuf>,
}

/// Annotation target — a file that can receive annotations.
/// Contains annotations and file-specific metadata, but NOT content.
/// Content lives in `View` (the root_view field on Review).
pub struct AnnotationTarget {
    /// Annotations keyed by id, in insertion order.
    pub annotations: IndexMap<String, Annotation>,
    /// File-specific metadata (language, etc.).
    pub metadata: FileMetadata,
}

impl Default for AnnotationTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnotationTarget {
    /// Create an empty annotation target.
    pub fn new() -> Self {
        Self {
            annotations: IndexMap::new(),
            metadata: FileMetadata::default(),
        }
    }
}

/// What a window is displaying.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowView {
    /// Window showing a file for annotation.
    File { key: FileKey },
    /// Window showing a diff for annotation.
    /// File keys are derived from line origins (FileKey::diff_file(index)).
    Diff { label: String },
    /// Window showing a rendered Mermaid diagram.
    Mermaid {
        file_key: FileKey,
        start_line: u32,
        end_line: u32,
    },
    // Future: FilePicker, Portal, Table, etc.
}

/// A file participating in a diff review.
/// Contains display metadata; annotations stored in Review.files by array position.
#[derive(Clone, Debug, Serialize)]
pub struct DiffFileView {
    /// Display path (new_name or old_name).
    pub path: PathBuf,
    /// Original path (for renames).
    pub old_path: Option<PathBuf>,
}

/// The root view — what content is being reviewed in this session.
/// Content lives here, separate from annotation storage.
#[derive(Clone)]
pub enum View {
    /// Single file review.
    File {
        path: PathBuf,
        content: ContentModel,
    },
    /// Diff review — multiple files participating.
    Diff {
        files: Vec<DiffFileView>,
        content: ContentModel,
    },
    // Future: Markdown { path, content, portals }
}

impl View {
    /// Get the content model.
    pub fn content(&self) -> &ContentModel {
        match self {
            View::File { content, .. } => content,
            View::Diff { content, .. } => content,
        }
    }

    /// Get the label for display.
    pub fn label(&self) -> &str {
        match self {
            View::File { content, .. } | View::Diff { content, .. } => &content.label,
        }
    }

    /// Get diff files if this is a diff view.
    pub fn diff_files(&self) -> Option<&[DiffFileView]> {
        match self {
            View::Diff { files, .. } => Some(files),
            _ => None,
        }
    }
}

impl Review {
    /// Create a CLI review (auto-detects file vs diff mode).
    ///
    /// Takes no root: a human standing in a directory *is* the assertion of
    /// where the review lives, so the process cwd is the honest answer.
    pub fn cli(content: ContentModel, config: UserConfig, root_window: String) -> Self {
        Self::new(
            content,
            config,
            crate::state::process_cwd(),
            root_window,
            None,
        )
    }

    /// Create an MCP review (auto-detects file vs diff mode).
    ///
    /// Takes a root explicitly: the sidecar's cwd is a spawn artifact, so the
    /// only party who knows where the review belongs is the caller.
    pub fn mcp(
        content: ContentModel,
        config: UserConfig,
        root: PathBuf,
        root_window: String,
        tx: Sender<FormatResult>,
    ) -> Self {
        Self::new(content, config, root, root_window, Some(tx))
    }

    /// Internal constructor that auto-detects content type.
    fn new(
        content: ContentModel,
        config: UserConfig,
        root: PathBuf,
        root_window: String,
        result_channel: Option<Sender<FormatResult>>,
    ) -> Self {
        let (root_view, files, window_view) =
            if matches!(content.view, crate::state::ContentView::Diff { .. }) {
                Self::build_diff_state(content)
            } else {
                Self::build_file_state(content)
            };

        let mut windows = HashMap::new();
        windows.insert(root_window.clone(), window_view);

        Self {
            root_view,
            files,
            root_window,
            windows,
            root,
            session_comment: None,
            selected_exit_mode_id: None,
            config,
            result_channel,
            saved_to: None,
        }
    }

    /// Build state for a single file.
    fn build_file_state(
        content: ContentModel,
    ) -> (View, HashMap<FileKey, AnnotationTarget>, WindowView) {
        let key = content.file_key();

        // Extract file extension for language metadata
        let extension = content
            .source
            .path_hint()
            .and_then(|p| std::path::Path::new(p).extension())
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string());

        let mut target = AnnotationTarget::new();
        target.metadata.language = extension;

        let mut files = HashMap::new();
        files.insert(key.clone(), target);

        // Register portal source files as annotation targets
        for portal in &content.portals {
            let portal_key = FileKey::path(portal.source_path.clone());
            files.entry(portal_key).or_insert_with(|| {
                // Extract extension from portal source path
                let portal_ext = portal
                    .source_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_string());
                let mut portal_target = AnnotationTarget::new();
                portal_target.metadata.language = portal_ext;
                portal_target
            });
        }

        // Note: View::File.path is not used anywhere, passing label as placeholder
        let path = PathBuf::from(content.label.clone());
        let root_view = View::File { path, content };

        let window_view = WindowView::File { key };

        (root_view, files, window_view)
    }

    /// Build state for a diff (multiple files).
    fn build_diff_state(
        content: ContentModel,
    ) -> (View, HashMap<FileKey, AnnotationTarget>, WindowView) {
        let window_label = content.label.clone();
        let mut diff_files = Vec::new();
        let mut files = HashMap::new();

        let crate::state::ContentView::Diff { documents } = &content.view else {
            unreachable!("build_diff_state requires a diff view");
        };
        for (index, doc) in documents.iter().enumerate() {
            diff_files.push(DiffFileView {
                path: PathBuf::from(&doc.path),
                old_path: doc.old_path.as_ref().map(PathBuf::from),
            });

            // Key by index (type-safe): `documents[index]` is the identity.
            let key = FileKey::diff_file(index);

            let mut target = AnnotationTarget::new();
            target.metadata.language = Some(doc.language.clone());
            files.insert(key, target);
        }

        let root_view = View::Diff {
            files: diff_files,
            content,
        };

        let window_view = WindowView::Diff {
            label: window_label,
        };

        (root_view, files, window_view)
    }

    /// Whether this is an MCP review (has result channel).
    pub fn is_mcp(&self) -> bool {
        self.result_channel.is_some()
    }

    /// Take the result channel (consumes it).
    pub fn take_result_sender(&mut self) -> Option<Sender<FormatResult>> {
        self.result_channel.take()
    }

    /// Register a new window.
    pub fn register_window(&mut self, label: String, view: WindowView) {
        self.windows.insert(label, view);
    }

    /// Unregister a window. Returns true if it was the root window.
    pub fn unregister_window(&mut self, label: &str) -> bool {
        self.windows.remove(label);
        label == self.root_window
    }

    /// Get all window labels (for cleanup).
    pub fn window_labels(&self) -> impl Iterator<Item = &str> {
        self.windows.keys().map(|s| s.as_str())
    }

    /// Verify a window exists in this review.
    /// Use this for commands that work on any window type (copy, save, export).
    pub fn verify_window(&self, window_label: &str) -> Result<(), String> {
        if self.windows.contains_key(window_label) {
            Ok(())
        } else {
            Err(format!("Unknown window: {}", window_label))
        }
    }

    /// Get the annotation target for a single-file window.
    /// Returns None for diff/mermaid windows — use resolve_target_mut() for commands.
    pub fn get_target_for_window(&self, window_label: &str) -> Option<&AnnotationTarget> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { key } => self.files.get(key),
            _ => None,
        }
    }

    /// Get the annotation target for a single-file window with detailed errors.
    /// For diff windows, use resolve_target_mut() which accepts explicit file_index.
    pub fn target_for_window(&self, window_label: &str) -> Result<&AnnotationTarget, String> {
        let view = self
            .windows
            .get(window_label)
            .ok_or_else(|| format!("Unknown window: {}", window_label))?;
        match view {
            WindowView::File { key } => self
                .files
                .get(key)
                .ok_or_else(|| "Target not loaded".into()),
            WindowView::Diff { .. } => {
                Err("Diff window: use resolve_target_mut with file_index".into())
            }
            _ => Err("Window type does not have a single target".into()),
        }
    }

    /// Get mutable annotation target for a single-file window.
    /// Returns None for diff/mermaid windows — use resolve_target_mut() for commands.
    pub fn get_target_for_window_mut(
        &mut self,
        window_label: &str,
    ) -> Option<&mut AnnotationTarget> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { key } => {
                let key = key.clone();
                self.files.get_mut(&key)
            }
            _ => None,
        }
    }

    /// Get annotation target by key.
    pub fn get_target(&self, key: &FileKey) -> Option<&AnnotationTarget> {
        self.files.get(key)
    }

    /// Get mutable annotation target by key.
    pub fn get_target_mut(&mut self, key: &FileKey) -> Option<&mut AnnotationTarget> {
        self.files.get_mut(key)
    }

    /// Resolve the annotation target for a command.
    /// Uses path to identify the target file. For diff mode, maps path to file index.
    pub fn resolve_target_mut(&mut self, path: &str) -> Result<&mut AnnotationTarget, String> {
        // First try direct path lookup (file mode, portal files)
        let path_key = FileKey::path(PathBuf::from(path));
        if self.files.contains_key(&path_key) {
            return self
                .files
                .get_mut(&path_key)
                .ok_or_else(|| format!("File not found: {}", path));
        }

        // Try ephemeral key (MCP review_content, stdin)
        let ephemeral_key = FileKey::ephemeral(path);
        if self.files.contains_key(&ephemeral_key) {
            return self
                .files
                .get_mut(&ephemeral_key)
                .ok_or_else(|| format!("Ephemeral content not found: {}", path));
        }

        // For diff mode, find the file by path
        if let Some(diff_files) = self.root_view.diff_files() {
            for (index, diff_file) in diff_files.iter().enumerate() {
                if diff_file.path.to_string_lossy() == path {
                    let key = FileKey::diff_file(index);
                    return self
                        .files
                        .get_mut(&key)
                        .ok_or_else(|| format!("Diff file not found: {}", path));
                }
            }
        }

        Err(format!("File not found: {}", path))
    }

    /// Check if image paste is allowed (MCP mode only).
    pub fn allows_image_paste(&self) -> bool {
        self.is_mcp()
    }

    /// Convert to ContentResponse for frontend (for a specific window).
    pub fn to_response_for_window(&self, window_label: &str) -> Option<ContentResponse> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { .. } | WindowView::Diff { .. } => {
                // Get content from root_view
                let content = self.root_view.content();
                Some(ContentResponse {
                    label: content.label.clone(),
                    view: content.view.clone(),
                    tags: self.config.tags().to_vec(),
                    exit_modes: self.config.exit_modes().to_vec(),
                    selected_exit_mode_id: self.selected_exit_mode_id.clone(),
                    session_comment: self.session_comment.clone(),
                    metadata: content.metadata.clone(),
                    allows_image_paste: content.source.allows_image_paste(),
                })
            }
            WindowView::Mermaid { .. } => None, // Mermaid windows don't use ContentResponse
        }
    }
}

impl AnnotationTarget {
    /// Insert or update an annotation by id. An anchor can only be claimed by
    /// one id at a time — upserting a new id at an anchor already held by a
    /// different id displaces the old one.
    pub fn upsert_annotation(&mut self, id: String, anchor: Anchor, content: Vec<ContentNode>) {
        self.annotations
            .retain(|existing_id, ann| *existing_id == id || ann.anchor != anchor);
        self.annotations.insert(
            id.clone(),
            Annotation {
                id,
                anchor,
                content,
            },
        );
    }

    /// Delete an annotation by id.
    pub fn delete_annotation(&mut self, id: &str) {
        self.annotations.shift_remove(id);
    }
}

impl ContentModel {
    /// Get the FileKey for this content.
    pub fn file_key(&self) -> FileKey {
        self.source.file_key()
    }
}

/// Type alias for the managed state.
pub type ActiveReview = parking_lot::Mutex<Option<Review>>;

#[cfg(test)]
mod tests {
    use super::*;
    fn anchor(line: u32) -> Anchor {
        Anchor::Source {
            path: "test.rs".to_string(),
            start: line,
            end: line,
        }
    }

    #[test]
    fn upserting_a_new_id_at_an_existing_anchor_displaces_the_old_one() {
        let mut target = AnnotationTarget::new();
        target.upsert_annotation("a".to_string(), anchor(5), vec![]);
        target.upsert_annotation("b".to_string(), anchor(5), vec![]);

        assert_eq!(target.annotations.len(), 1);
        assert!(target.annotations.contains_key("b"));
        assert!(!target.annotations.contains_key("a"));
    }

    #[test]
    fn upserting_the_same_id_at_a_new_anchor_moves_it_without_displacing_others() {
        let mut target = AnnotationTarget::new();
        target.upsert_annotation("a".to_string(), anchor(5), vec![]);
        target.upsert_annotation("b".to_string(), anchor(10), vec![]);
        target.upsert_annotation("a".to_string(), anchor(20), vec![]);

        assert_eq!(target.annotations.len(), 2);
        assert_eq!(target.annotations["a"].anchor.start_line(), 20);
        assert_eq!(target.annotations["b"].anchor.start_line(), 10);
    }

    #[test]
    fn delete_annotation_removes_by_id() {
        let mut target = AnnotationTarget::new();
        target.upsert_annotation("a".to_string(), anchor(5), vec![]);
        target.delete_annotation("a");

        assert!(target.annotations.is_empty());
    }
}
