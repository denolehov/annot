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

use serde::Serialize;

use crate::output::FormatResult;
use crate::state::{Annotation, ContentMetadata, ContentModel, ContentNode, ContentResponse, FileMetadata, LineRange, UserConfig};

/// Key for annotation targets in Review.files.
/// Distinguishes real file paths from synthetic diff file references.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileKey {
    /// A real file path.
    Path(PathBuf),
    /// A file within a diff, identified by index.
    DiffFile { index: usize },
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
}

impl fmt::Display for FileKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileKey::Path(p) => write!(f, "{}", p.display()),
            FileKey::DiffFile { index } => write!(f, "diff file {}", index),
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
    /// Session-level comment (not tied to specific lines/files).
    pub session_comment: Option<Vec<ContentNode>>,
    /// Currently selected exit mode ID.
    pub selected_exit_mode_id: Option<String>,
    /// User configuration (tags, exit modes).
    pub config: UserConfig,

    //--- Result delivery ---
    /// Channel to send result when review ends. `None` for CLI mode.
    result_channel: Option<Sender<FormatResult>>,
}

/// Annotation target — a file that can receive annotations.
/// Contains annotations and file-specific metadata, but NOT content.
/// Content lives in `View` (the root_view field on Review).
pub struct AnnotationTarget {
    /// Annotations keyed by normalized line range.
    pub annotations: HashMap<LineRange, Annotation>,
    /// File-specific metadata (language, etc.).
    pub metadata: FileMetadata,
}

impl AnnotationTarget {
    /// Create an empty annotation target.
    pub fn new() -> Self {
        Self {
            annotations: HashMap::new(),
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
    pub fn cli(content: ContentModel, config: UserConfig, root_window: String) -> Self {
        Self::new(content, config, root_window, None)
    }

    /// Create an MCP review (auto-detects file vs diff mode).
    pub fn mcp(
        content: ContentModel,
        config: UserConfig,
        root_window: String,
        tx: Sender<FormatResult>,
    ) -> Self {
        Self::new(content, config, root_window, Some(tx))
    }

    /// Internal constructor that auto-detects content type.
    fn new(
        content: ContentModel,
        config: UserConfig,
        root_window: String,
        result_channel: Option<Sender<FormatResult>>,
    ) -> Self {
        // Extract diff metadata before moving content
        let diff_meta = match &content.metadata {
            ContentMetadata::Diff(dm) => Some(dm.clone()),
            _ => None,
        };

        let (root_view, files, window_view) = if let Some(dm) = diff_meta {
            Self::build_diff_state(content, dm)
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
            session_comment: None,
            selected_exit_mode_id: None,
            config,
            result_channel,
        }
    }

    /// Build state for a single file.
    fn build_file_state(
        content: ContentModel,
    ) -> (View, HashMap<FileKey, AnnotationTarget>, WindowView) {
        let path = content.source_path();
        let key = FileKey::path(path.clone());

        let mut files = HashMap::new();
        files.insert(key.clone(), AnnotationTarget::new());

        // Register portal source files as annotation targets
        for portal in &content.portals {
            let portal_key = FileKey::path(portal.source_path.clone());
            if !files.contains_key(&portal_key) {
                files.insert(portal_key, AnnotationTarget::new());
            }
        }

        let root_view = View::File {
            path: path.clone(),
            content,
        };

        let window_view = WindowView::File { key };

        (root_view, files, window_view)
    }

    /// Build state for a diff (multiple files).
    fn build_diff_state(
        content: ContentModel,
        diff_meta: crate::diff::DiffMetadata,
    ) -> (View, HashMap<FileKey, AnnotationTarget>, WindowView) {
        let window_label = content.label.clone();
        let mut diff_files = Vec::new();
        let mut files = HashMap::new();

        for (index, file_info) in diff_meta.files.iter().enumerate() {
            // Use new_name if available, otherwise old_name (for display)
            let display_path = file_info
                .new_name
                .as_ref()
                .or(file_info.old_name.as_ref())
                .map(|s| PathBuf::from(s))
                .unwrap_or_else(|| PathBuf::from("unknown"));

            let old_path = file_info.old_name.as_ref().map(PathBuf::from);

            diff_files.push(DiffFileView {
                path: display_path,
                old_path,
            });

            // Key by index (type-safe)
            let key = FileKey::diff_file(index);

            // Create annotation target for this file
            let mut target = AnnotationTarget::new();
            target.metadata.language = Some(file_info.language.clone());
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
        let view = self.windows.get(window_label)
            .ok_or_else(|| format!("Unknown window: {}", window_label))?;
        match view {
            WindowView::File { key } => {
                self.files.get(key).ok_or_else(|| "Target not loaded".into())
            }
            WindowView::Diff { .. } => Err("Diff window: use resolve_target_mut with file_index".into()),
            _ => Err("Window type does not have a single target".into()),
        }
    }

    /// Get mutable annotation target for a single-file window.
    /// Returns None for diff/mermaid windows — use resolve_target_mut() for commands.
    pub fn get_target_for_window_mut(&mut self, window_label: &str) -> Option<&mut AnnotationTarget> {
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
                    lines: content.lines.clone(),
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
    /// Insert or update an annotation.
    pub fn upsert_annotation(&mut self, start_line: u32, end_line: u32, content: Vec<ContentNode>) {
        let key = LineRange::new(start_line, end_line);
        self.annotations.insert(
            key,
            Annotation {
                start_line: key.start,
                end_line: key.end,
                content,
            },
        );
    }

    /// Delete an annotation by range.
    pub fn delete_annotation(&mut self, start_line: u32, end_line: u32) {
        self.annotations.remove(&LineRange::new(start_line, end_line));
    }
}

impl ContentModel {
    /// Get a path for keying this content.
    /// For file-based content, returns the actual path.
    /// For ephemeral/stdin content, returns a synthetic path.
    pub fn source_path(&self) -> PathBuf {
        use crate::input::{CliSource, ContentSource, McpSource};
        match &self.source {
            ContentSource::Cli(CliSource::File { path }) => path.clone(),
            ContentSource::Mcp(McpSource::File { path }) => path.clone(),
            ContentSource::Cli(CliSource::Stdin { label }) => {
                PathBuf::from(format!("__stdin__/{}", label))
            }
            ContentSource::Mcp(McpSource::Content { label }) => {
                PathBuf::from(format!("__ephemeral__/{}", label))
            }
            ContentSource::Mcp(McpSource::Diff { label, .. }) => {
                let name = label.as_deref().unwrap_or("diff");
                PathBuf::from(format!("__diff__/{}", name))
            }
        }
    }
}

/// Type alias for the managed state.
pub type ActiveReview = parking_lot::Mutex<Option<Review>>;
