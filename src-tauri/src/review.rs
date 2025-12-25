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
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use serde::Serialize;

use crate::output::FormatResult;
use crate::state::{Annotation, ContentModel, ContentNode, ContentResponse, FileMetadata, LineRange, UserConfig};

/// An active review. Wrapped in `Option`: `Some` = active, `None` = idle.
pub struct Review {
    //--- Content (what exists) ---
    /// The root view — what content is being reviewed.
    /// Content lives here, separate from annotation storage.
    pub root_view: View,
    /// Annotation targets keyed by path.
    /// For ephemeral content or stdin, uses a synthetic path like "__ephemeral__".
    pub files: HashMap<PathBuf, AnnotationTarget>,

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

/// Type alias for backwards compatibility during migration.
#[deprecated(note = "Use AnnotationTarget instead")]
pub type FileState = AnnotationTarget;

/// What a window is displaying.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowView {
    /// Window showing a file for annotation.
    File { path: PathBuf },
    /// Window showing a rendered Mermaid diagram.
    Mermaid {
        file_path: PathBuf,
        start_line: u32,
        end_line: u32,
    },
    // Future: FilePicker, Portal, Table, etc.
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
    // Future: Diff { label, files }, Markdown { path, content, portals }
}

impl View {
    /// Get the content model.
    pub fn content(&self) -> &ContentModel {
        match self {
            View::File { content, .. } => content,
        }
    }

    /// Get the path for the primary file.
    pub fn path(&self) -> &PathBuf {
        match self {
            View::File { path, .. } => path,
        }
    }

    /// Get the label for display.
    pub fn label(&self) -> &str {
        match self {
            View::File { content, .. } => &content.label,
        }
    }
}

impl Review {
    /// Create a CLI review (single file, no result channel).
    pub fn cli(content: ContentModel, config: UserConfig, root_window: String) -> Self {
        let path = content.source_path();

        // Create root view with content
        let root_view = View::File {
            path: path.clone(),
            content,
        };

        // Create annotation target for this file
        let mut files = HashMap::new();
        files.insert(path.clone(), AnnotationTarget::new());

        let mut windows = HashMap::new();
        windows.insert(root_window.clone(), WindowView::File { path });

        Self {
            root_view,
            files,
            root_window,
            windows,
            session_comment: None,
            selected_exit_mode_id: None,
            config,
            result_channel: None,
        }
    }

    /// Create an MCP review (single file with result channel).
    pub fn mcp(
        content: ContentModel,
        config: UserConfig,
        root_window: String,
        tx: Sender<FormatResult>,
    ) -> Self {
        let path = content.source_path();

        // Create root view with content
        let root_view = View::File {
            path: path.clone(),
            content,
        };

        // Create annotation target for this file
        let mut files = HashMap::new();
        files.insert(path.clone(), AnnotationTarget::new());

        let mut windows = HashMap::new();
        windows.insert(root_window.clone(), WindowView::File { path });

        Self {
            root_view,
            files,
            root_window,
            windows,
            session_comment: None,
            selected_exit_mode_id: None,
            config,
            result_channel: Some(tx),
        }
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

    /// Get the annotation target for a window, if any.
    pub fn get_target_for_window(&self, window_label: &str) -> Option<&AnnotationTarget> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { path } => self.files.get(path),
            WindowView::Mermaid { file_path, .. } => self.files.get(file_path),
        }
    }

    /// Get the annotation target for a window with detailed errors.
    pub fn target_for_window(&self, window_label: &str) -> Result<&AnnotationTarget, String> {
        let view = self.windows.get(window_label)
            .ok_or_else(|| format!("Unknown window: {}", window_label))?;
        match view {
            WindowView::File { path } => {
                self.files.get(path).ok_or_else(|| "Target not loaded".into())
            }
            _ => Err("Window is not showing a file".into()),
        }
    }

    /// Get mutable annotation target for a window, if any.
    pub fn get_target_for_window_mut(&mut self, window_label: &str) -> Option<&mut AnnotationTarget> {
        let view = self.windows.get(window_label)?;
        let path = match view {
            WindowView::File { path } => path.clone(),
            WindowView::Mermaid { file_path, .. } => file_path.clone(),
        };
        self.files.get_mut(&path)
    }

    /// Get annotation target by path.
    pub fn get_target(&self, path: &PathBuf) -> Option<&AnnotationTarget> {
        self.files.get(path)
    }

    /// Get mutable annotation target by path.
    pub fn get_target_mut(&mut self, path: &PathBuf) -> Option<&mut AnnotationTarget> {
        self.files.get_mut(path)
    }

    // Deprecated aliases for backwards compatibility
    #[deprecated(note = "Use get_target_for_window instead")]
    pub fn get_file_for_window(&self, window_label: &str) -> Option<&AnnotationTarget> {
        self.get_target_for_window(window_label)
    }

    #[deprecated(note = "Use target_for_window instead")]
    pub fn file_for_window(&self, window_label: &str) -> Result<&AnnotationTarget, String> {
        self.target_for_window(window_label)
    }

    #[deprecated(note = "Use get_target_for_window_mut instead")]
    pub fn get_file_for_window_mut(&mut self, window_label: &str) -> Option<&mut AnnotationTarget> {
        self.get_target_for_window_mut(window_label)
    }

    /// Check if image paste is allowed (MCP mode only).
    pub fn allows_image_paste(&self) -> bool {
        self.is_mcp()
    }

    /// Convert to ContentResponse for frontend (for a specific window).
    pub fn to_response_for_window(&self, window_label: &str) -> Option<ContentResponse> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { path: _ } => {
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
