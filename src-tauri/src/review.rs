//! Review abstraction for state management.
//!
//! A Review represents an active annotation task. It owns:
//! - Content (files loaded for annotation)
//! - Windows (how content is displayed)
//! - Session-level state (comment, exit mode, result channel)
//!
//! Content and windows are orthogonal:
//! - A window is a viewport that can display any content type
//! - Annotations live on content (FileState), not windows
//! - Two windows showing the same file share annotations

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use serde::Serialize;

use crate::output::FormatResult;
use crate::state::{Annotation, ContentModel, ContentNode, ContentResponse, UserConfig};

/// An active review. Wrapped in `Option`: `Some` = active, `None` = idle.
pub struct Review {
    //--- Content (what exists) ---
    /// Files loaded in this review, keyed by path.
    /// For ephemeral content or stdin, uses a synthetic path like "__ephemeral__".
    pub files: HashMap<PathBuf, FileState>,

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

/// State for a single file within the review.
pub struct FileState {
    /// The content model (lines, metadata, source info).
    pub content: ContentModel,
    /// Annotations keyed by "start-end" range string (e.g., "10-15").
    pub annotations: HashMap<String, Annotation>,
}

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

impl Review {
    /// Create a CLI review (single file, no result channel).
    pub fn cli(content: ContentModel, config: UserConfig, root_window: String) -> Self {
        let path = content.source_path();
        let mut files = HashMap::new();
        files.insert(
            path.clone(),
            FileState {
                content,
                annotations: HashMap::new(),
            },
        );

        let mut windows = HashMap::new();
        windows.insert(root_window.clone(), WindowView::File { path });

        Self {
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
        let mut files = HashMap::new();
        files.insert(
            path.clone(),
            FileState {
                content,
                annotations: HashMap::new(),
            },
        );

        let mut windows = HashMap::new();
        windows.insert(root_window.clone(), WindowView::File { path });

        Self {
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

    /// Get the file for a window, if any.
    pub fn get_file_for_window(&self, window_label: &str) -> Option<&FileState> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { path } => self.files.get(path),
            WindowView::Mermaid { file_path, .. } => self.files.get(file_path),
        }
    }

    /// Get the file for a window with detailed errors.
    pub fn file_for_window(&self, window_label: &str) -> Result<&FileState, String> {
        let view = self.windows.get(window_label)
            .ok_or_else(|| format!("Unknown window: {}", window_label))?;
        match view {
            WindowView::File { path } => {
                self.files.get(path).ok_or_else(|| "File not loaded".into())
            }
            _ => Err("Window is not showing a file".into()),
        }
    }

    /// Get mutable file for a window, if any.
    pub fn get_file_for_window_mut(&mut self, window_label: &str) -> Option<&mut FileState> {
        let view = self.windows.get(window_label)?;
        let path = match view {
            WindowView::File { path } => path.clone(),
            WindowView::Mermaid { file_path, .. } => file_path.clone(),
        };
        self.files.get_mut(&path)
    }

    /// Get file by path.
    pub fn get_file(&self, path: &PathBuf) -> Option<&FileState> {
        self.files.get(path)
    }

    /// Get mutable file by path.
    pub fn get_file_mut(&mut self, path: &PathBuf) -> Option<&mut FileState> {
        self.files.get_mut(path)
    }

    /// Check if image paste is allowed (MCP mode only).
    pub fn allows_image_paste(&self) -> bool {
        self.is_mcp()
    }

    /// Convert to ContentResponse for frontend (for a specific window).
    pub fn to_response_for_window(&self, window_label: &str) -> Option<ContentResponse> {
        let view = self.windows.get(window_label)?;
        match view {
            WindowView::File { path } => {
                let file = self.files.get(path)?;
                Some(ContentResponse {
                    label: file.content.label.clone(),
                    lines: file.content.lines.clone(),
                    tags: self.config.tags().to_vec(),
                    exit_modes: self.config.exit_modes().to_vec(),
                    selected_exit_mode_id: self.selected_exit_mode_id.clone(),
                    session_comment: self.session_comment.clone(),
                    metadata: file.content.metadata.clone(),
                    allows_image_paste: file.content.source.allows_image_paste(),
                })
            }
            WindowView::Mermaid { .. } => None, // Mermaid windows don't use ContentResponse
        }
    }
}

impl FileState {
    /// Create a normalized range key (smaller line first).
    pub fn range_key(start_line: u32, end_line: u32) -> String {
        let (min, max) = if start_line <= end_line {
            (start_line, end_line)
        } else {
            (end_line, start_line)
        };
        format!("{}-{}", min, max)
    }

    /// Insert or update an annotation.
    pub fn upsert_annotation(&mut self, start_line: u32, end_line: u32, content: Vec<ContentNode>) {
        let key = Self::range_key(start_line, end_line);
        let (min, max) = if start_line <= end_line {
            (start_line, end_line)
        } else {
            (end_line, start_line)
        };
        self.annotations.insert(
            key,
            Annotation {
                start_line: min,
                end_line: max,
                content,
            },
        );
    }

    /// Delete an annotation by range.
    pub fn delete_annotation(&mut self, start_line: u32, end_line: u32) {
        let key = Self::range_key(start_line, end_line);
        self.annotations.remove(&key);
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
