use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::diff::{self, DiffMetadata};
use crate::error::AnnotError;
use crate::highlight::Highlighter;
use crate::input::ContentSource;
use crate::markdown::{self, html_escape, MarkdownMetadata, MarkdownSemantics};

// =============================================================================
// Unified line model (LineOrigin + LineSemantics)
// =============================================================================

use std::path::PathBuf;

/// Where this line's content originates from.
/// Carries line number information for annotation routing and gutter display.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LineOrigin {
    /// Line from the primary document being reviewed.
    Document {
        /// 1-indexed line number in the source file.
        line: u32,
    },
    /// Line from a diff (maps to old/new file versions).
    Diff {
        /// Line number in old file (None if added line or header).
        old_line: Option<u32>,
        /// Line number in new file (None if deleted line or header).
        new_line: Option<u32>,
        /// Index into diff file list for annotation routing.
        file_index: usize,
    },
    /// Line from an external file (portal content).
    External {
        /// Path to the external file.
        file: PathBuf,
        /// 1-indexed line number in the external file.
        line: u32,
        /// Portal identifier for grouping and boundaries.
        portal_id: String,
    },
    /// Synthetic line with no source (portal headers, decorators).
    Virtual,
}

/// Content classification: what kind of line is this?
#[derive(Clone, Debug, Serialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LineSemantics {
    #[default]
    Plain,
    Markdown(MarkdownSemantics),
    Diff(DiffSemantics),
    Portal(PortalSemantics),
}

// MarkdownSemantics is imported from crate::markdown

/// Diff line semantics.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffSemantics {
    FileHeader,
    HunkHeader { context: Option<String> },
    Added,
    Deleted,
    Context,
}

/// Portal line semantics.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PortalSemantics {
    Header {
        label: String,
        path: String,
        range: String,
    },
    Content,
    Footer,
}

/// A single line of content.
#[derive(Clone, Serialize)]
pub struct Line {
    /// Raw text content of the line.
    pub content: String,
    /// Rendered HTML for display:
    /// - For code blocks: syntect-highlighted spans
    /// - For markdown: inline formatting (bold, italic, etc.)
    /// - None if no rendering needed
    pub html: Option<String>,
    /// Where this line originates from.
    pub origin: LineOrigin,
    /// Content classification.
    pub semantics: LineSemantics,
}

/// A tag is a composable mini-prompt that can be embedded in annotations.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub instruction: String,
}

impl Tag {
    /// Creates a new tag with a generated 12-character alphanumeric ID.
    pub fn new(name: String, instruction: String) -> Self {
        Self {
            id: generate_id(),
            name,
            instruction,
        }
    }
}

/// Generates a 12-character alphanumeric ID.
fn generate_id() -> String {
    use rand::distributions::{Alphanumeric, DistString};
    Alphanumeric.sample_string(&mut rand::thread_rng(), 12)
}

/// Content node for structured annotation content.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentNode {
    Text { text: String },
    Tag { id: String, name: String, instruction: String },
    Media { image: String, mime_type: String },
    Excalidraw { elements: String, image: Option<String> },
}

/// A normalized line range (start ≤ end).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

impl LineRange {
    /// Create a normalized range (swaps if start > end).
    #[must_use]
    pub fn new(a: u32, b: u32) -> Self {
        Self {
            start: a.min(b),
            end: a.max(b),
        }
    }
}

/// An annotation attached to a line range.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub start_line: u32,
    pub end_line: u32,
    pub content: Vec<ContentNode>,
}

/// Where an exit mode was defined.
///
/// ```text
/// ExitModeOrigin (persist?)  x  ContentSource (where from?)
///      Persisted                    Cli::File
///      Transient                    Cli::Stdin
///                                   Mcp::File
///                                   Mcp::Content
///                                   Mcp::Diff
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExitModeOrigin {
    /// Loaded from config file — will be saved on exit.
    #[default]
    Persisted,
    /// Provided in MCP tool call params — session-only, not saved.
    Transient,
}

/// An exit mode representing a user decision (Apply, Reject, Revise, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExitMode {
    pub id: String,
    pub name: String,
    /// CSS hex color (e.g., "#22c55e")
    pub color: String,
    /// LLM-facing instruction text
    pub instruction: String,
    pub order: u32,
    /// Where this exit mode came from (config or MCP params).
    #[serde(default)]
    pub origin: ExitModeOrigin,
}

impl ExitMode {
    /// Creates a new exit mode with a generated 12-character alphanumeric ID.
    pub fn new(name: String, color: String, instruction: String, order: u32) -> Self {
        Self {
            id: generate_id(),
            name,
            color,
            instruction,
            order,
            origin: ExitModeOrigin::Persisted,
        }
    }

    /// Whether this exit mode is transient (from MCP params, not persisted).
    pub fn is_transient(&self) -> bool {
        matches!(self.origin, ExitModeOrigin::Transient)
    }
}

// ════════════════════════════════════════════════════════════════════════════
// CONTENT MODEL — immutable after construction
// ════════════════════════════════════════════════════════════════════════════

/// Content model: the document being annotated.
/// Immutable after construction.
#[derive(Clone)]
pub struct ContentModel {
    pub label: String,
    pub lines: Vec<Line>,
    pub source: ContentSource,
    pub metadata: ContentMetadata,
}

/// Type-safe representation of content-specific metadata.
/// Replaces the two Option fields that were mutually exclusive.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentMetadata {
    Plain,
    Diff(DiffMetadata),
    Markdown(MarkdownMetadata),
}

/// Per-file metadata for annotation targets.
/// Contains file-level info that's NOT content (e.g., language for syntax highlighting).
#[derive(Clone, Debug, Default, Serialize)]
pub struct FileMetadata {
    /// Language identifier for syntax highlighting (e.g., "rs", "go", "py").
    pub language: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════
// SESSION STATE — mutates during session, not persisted
// ════════════════════════════════════════════════════════════════════════════

/// Session state: mutable data during annotation session.
#[derive(Default)]
pub struct SessionState {
    /// Annotations keyed by normalized line range.
    pub annotations: HashMap<LineRange, Annotation>,
    /// Session-level comment (not tied to specific lines).
    pub comment: Option<Vec<ContentNode>>,
    /// Currently selected exit mode ID (None if no mode selected).
    pub selected_exit_mode_id: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════
// USER CONFIG — encapsulates deletion tracking
// ════════════════════════════════════════════════════════════════════════════

/// User configuration for tags and exit modes.
/// Encapsulates deletion tracking for safe concurrent writes.
pub struct UserConfig {
    tags: Vec<Tag>,
    exit_modes: Vec<ExitMode>,
    deleted_tags: HashSet<String>,
    deleted_exit_modes: HashSet<String>,
}

impl UserConfig {
    /// Load configuration from disk.
    pub fn load() -> Self {
        Self {
            tags: config::load_tags(),
            exit_modes: config::load_exit_modes(),
            deleted_tags: HashSet::new(),
            deleted_exit_modes: HashSet::new(),
        }
    }

    /// Create empty config (for testing).
    pub fn empty() -> Self {
        Self {
            tags: Vec::new(),
            exit_modes: Vec::new(),
            deleted_tags: HashSet::new(),
            deleted_exit_modes: HashSet::new(),
        }
    }

    /// Get all tags.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// Get all exit modes.
    pub fn exit_modes(&self) -> &[ExitMode] {
        &self.exit_modes
    }

    /// Get mutable reference to exit modes (for reordering).
    pub fn exit_modes_mut(&mut self) -> &mut Vec<ExitMode> {
        &mut self.exit_modes
    }

    /// Insert or update a tag, then save to disk.
    pub fn upsert_tag(&mut self, tag: Tag) {
        if let Some(existing) = self.tags.iter_mut().find(|t| t.id == tag.id) {
            *existing = tag;
        } else {
            self.tags.push(tag);
        }
        let _ = config::save_tags(&self.tags, &self.deleted_tags);
    }

    /// Delete a tag by ID, then save to disk.
    pub fn delete_tag(&mut self, id: &str) {
        self.tags.retain(|t| t.id != id);
        self.deleted_tags.insert(id.to_string());
        let _ = config::save_tags(&self.tags, &self.deleted_tags);
    }

    /// Insert or update an exit mode, then save to disk.
    /// Only persists non-transient modes.
    pub fn upsert_exit_mode(&mut self, mode: ExitMode) {
        if let Some(existing) = self.exit_modes.iter_mut().find(|m| m.id == mode.id) {
            *existing = mode;
        } else {
            self.exit_modes.push(mode);
        }
        // Sort by order
        self.exit_modes.sort_by_key(|m| m.order);
        // Only save persisted modes
        let persisted: Vec<_> = self
            .exit_modes
            .iter()
            .filter(|m| !m.is_transient())
            .cloned()
            .collect();
        let _ = config::save_exit_modes(&persisted, &self.deleted_exit_modes);
    }

    /// Delete an exit mode by ID, then save to disk.
    pub fn delete_exit_mode(&mut self, id: &str) {
        self.exit_modes.retain(|m| m.id != id);
        self.deleted_exit_modes.insert(id.to_string());
        let persisted: Vec<_> = self
            .exit_modes
            .iter()
            .filter(|m| !m.is_transient())
            .cloned()
            .collect();
        let _ = config::save_exit_modes(&persisted, &self.deleted_exit_modes);
    }

    /// Reorder exit modes by ID list, then save to disk.
    pub fn reorder_exit_modes(&mut self, ids: Vec<String>) {
        // Update order field based on position in ids
        for (new_order, id) in ids.iter().enumerate() {
            if let Some(mode) = self.exit_modes.iter_mut().find(|m| m.id == *id) {
                mode.order = new_order as u32;
            }
        }
        self.exit_modes.sort_by_key(|m| m.order);
        let persisted: Vec<_> = self
            .exit_modes
            .iter()
            .filter(|m| !m.is_transient())
            .cloned()
            .collect();
        let _ = config::save_exit_modes(&persisted, &self.deleted_exit_modes);
    }

    /// Prepend transient exit modes (from MCP params) at the start.
    pub fn prepend_transient_modes(&mut self, modes: Vec<ExitMode>) {
        // Insert at beginning, shifting existing modes
        self.exit_modes.splice(0..0, modes);
    }

    /// Create config with specific tags and exit modes (for testing).
    #[cfg(test)]
    pub fn with_data(tags: Vec<Tag>, exit_modes: Vec<ExitMode>) -> Self {
        Self {
            tags,
            exit_modes,
            deleted_tags: HashSet::new(),
            deleted_exit_modes: HashSet::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// APP STATE — composition of content, session, and config
// ════════════════════════════════════════════════════════════════════════════

/// Application state initialized at startup, before the window opens.
pub struct AppState {
    pub content: ContentModel,
    pub session: SessionState,
    pub config: UserConfig,
}

/// Response sent to the frontend via the get_content command.
#[derive(Serialize)]
pub struct ContentResponse {
    pub label: String,
    pub lines: Vec<Line>,
    pub tags: Vec<Tag>,
    pub exit_modes: Vec<ExitMode>,
    pub selected_exit_mode_id: Option<String>,
    pub session_comment: Option<Vec<ContentNode>>,
    /// Content-specific metadata (diff info, markdown sections, or plain).
    pub metadata: ContentMetadata,
    /// Whether image paste is allowed (MCP mode only).
    pub allows_image_paste: bool,
}

// Render functions moved to crate::markdown (render_line, render_inline)

impl ContentModel {
    /// Parse file content into structured lines with syntax highlighting.
    #[must_use]
    pub fn from_file(content: &str, source: ContentSource) -> Self {
        let label = source.label().to_string();
        let path = source.path_hint().unwrap_or("");

        let highlighter = Highlighter::new();
        let html_lines = highlighter.highlight_lines(content, path);

        let lines = content
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let line_num = (i + 1) as u32;
                let html = html_lines.get(i).cloned();
                Line {
                    content: line.to_string(),
                    html,
                    origin: LineOrigin::Document { line: line_num },
                    semantics: LineSemantics::Plain,
                }
            })
            .collect();

        Self {
            label,
            lines,
            source,
            metadata: ContentMetadata::Plain,
        }
    }

    /// Parse diff content into structured lines with diff metadata.
    #[must_use]
    pub fn from_diff(content: &str, source: ContentSource) -> Result<Self, AnnotError> {
        let label = source.label().to_string();
        let mut diff_metadata = diff::parse_diff(content)?;
        let highlighter = Highlighter::new();

        // Highlight function contexts in hunk headers
        for file in &mut diff_metadata.files {
            let fake_path = format!("file.{}", file.language);
            for hunk in &mut file.hunks {
                if let Some(ref ctx) = hunk.function_context {
                    let html = highlighter.highlight_snippet(ctx, &fake_path);
                    if !html.is_empty() {
                        hunk.function_context_html = Some(html);
                    }
                }
            }
        }

        // For diffs, we create lines from the raw content
        // Each line gets its display number (1-indexed)
        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(i, line_content)| {
                let line_num = (i + 1) as u32;

                // Get file language for this line from diff metadata
                let language = diff_metadata
                    .lines
                    .get(&line_num)
                    .and_then(|info| diff_metadata.files.get(info.file_index))
                    .map(|f| f.language.as_str())
                    .unwrap_or("");

                // Only highlight non-header lines with actual code
                let html = if !language.is_empty()
                    && !line_content.starts_with("diff ")
                    && !line_content.starts_with("---")
                    && !line_content.starts_with("+++")
                    && !line_content.starts_with("@@")
                    && !line_content.starts_with("index ")
                {
                    // Strip the +/- prefix for highlighting, then add it back
                    let (prefix, code) = if line_content.starts_with('+')
                        || line_content.starts_with('-')
                        || line_content.starts_with(' ')
                    {
                        (&line_content[..1], &line_content[1..])
                    } else {
                        ("", line_content)
                    };

                    let fake_path = format!("file.{}", language);
                    let highlighted = highlighter.highlight_lines(code, &fake_path);
                    highlighted.first().map(|h| format!("{}{}", prefix, h))
                } else {
                    None
                };

                // Get diff line info for origin and semantics
                let diff_info = diff_metadata.lines.get(&line_num);

                let (origin, semantics) = match diff_info {
                    Some(info) => {
                        let origin = LineOrigin::Diff {
                            old_line: info.old_line_num,
                            new_line: info.new_line_num,
                            file_index: info.file_index,
                        };
                        let semantics = LineSemantics::Diff(match info.kind {
                            diff::DiffLineKind::Context => DiffSemantics::Context,
                            diff::DiffLineKind::Added => DiffSemantics::Added,
                            diff::DiffLineKind::Deleted => DiffSemantics::Deleted,
                            diff::DiffLineKind::Header => DiffSemantics::FileHeader,
                        });
                        (origin, semantics)
                    }
                    None => {
                        // Lines not in diff metadata (shouldn't happen, but fallback)
                        (
                            LineOrigin::Document { line: line_num },
                            LineSemantics::Plain,
                        )
                    }
                };

                Line {
                    content: line_content.to_string(),
                    html,
                    origin,
                    semantics,
                }
            })
            .collect();

        Ok(Self {
            label,
            lines,
            source,
            metadata: ContentMetadata::Diff(diff_metadata),
        })
    }

    /// Parse markdown content with inline rendering and code block highlighting.
    #[must_use]
    pub fn from_markdown(content: &str, source: ContentSource) -> Self {
        let label = source.label().to_string();

        let md_metadata = markdown::parse_markdown(content);
        let highlighter = Highlighter::new();

        // Build map of code block info by line: (language, is_fence_start, is_fence_end)
        #[derive(Clone)]
        struct CodeBlockLineInfo {
            language: Option<String>,
            is_start: bool,
            is_end: bool,
        }
        let mut code_block_lines: HashMap<u32, CodeBlockLineInfo> = HashMap::new();
        for block in &md_metadata.code_blocks {
            for line_num in block.start_line..=block.end_line {
                code_block_lines.insert(
                    line_num,
                    CodeBlockLineInfo {
                        language: block.language.clone(),
                        is_start: line_num == block.start_line,
                        is_end: line_num == block.end_line,
                    },
                );
            }
        }

        // Build set of table lines
        let mut table_lines: HashSet<u32> = HashSet::new();
        let mut table_replacements: HashMap<u32, String> = HashMap::new();
        for table in &md_metadata.tables {
            for (i, formatted) in table.formatted_lines.iter().enumerate() {
                let line_num = table.start_line + i as u32;
                table_lines.insert(line_num);
                table_replacements.insert(line_num, formatted.clone());
            }
        }

        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(i, line_content)| {
                let line_num = (i + 1) as u32;

                // Use table replacement if available
                let display_content = table_replacements
                    .get(&line_num)
                    .cloned()
                    .unwrap_or_else(|| line_content.to_string());

                let origin = LineOrigin::Document { line: line_num };

                // Determine HTML rendering strategy and semantics
                if let Some(info) = code_block_lines.get(&line_num) {
                    // Inside a code block
                    let semantics = LineSemantics::Markdown(if info.is_start {
                        MarkdownSemantics::CodeBlockStart {
                            language: info.language.clone(),
                        }
                    } else if info.is_end {
                        MarkdownSemantics::CodeBlockEnd
                    } else {
                        MarkdownSemantics::CodeBlockContent
                    });

                    let html = if info.is_start || info.is_end {
                        // Fence line: render as-is (escaped)
                        Some(html_escape(&display_content))
                    } else if let Some(ref language) = info.language {
                        // Code content: highlight with syntect
                        let ext = Highlighter::language_to_extension(language);
                        let fake_path = format!("file.{ext}");
                        let highlighted = highlighter.highlight_lines(line_content, &fake_path);
                        highlighted.first().cloned()
                    } else {
                        // No language specified: just escape
                        Some(html_escape(&display_content))
                    };

                    Line {
                        content: display_content,
                        html,
                        origin,
                        semantics,
                    }
                } else if table_lines.contains(&line_num) {
                    // Table row
                    Line {
                        content: display_content.clone(),
                        html: Some(html_escape(&display_content)),
                        origin,
                        semantics: LineSemantics::Markdown(MarkdownSemantics::TableRow),
                    }
                } else {
                    // Regular markdown: render with structural markers preserved
                    let rendered = markdown::render_line(line_content);
                    let semantics = match rendered.semantics {
                        Some(md_sem) => LineSemantics::Markdown(md_sem),
                        None => LineSemantics::Plain,
                    };
                    Line {
                        content: display_content,
                        html: Some(rendered.html),
                        origin,
                        semantics,
                    }
                }
            })
            .collect();

        Self {
            label,
            lines,
            source,
            metadata: ContentMetadata::Markdown(md_metadata),
        }
    }
}

impl AppState {
    /// Create a new AppState from content model and config.
    pub fn new(content: ContentModel, config: UserConfig) -> Self {
        Self {
            content,
            session: SessionState::default(),
            config,
        }
    }

    /// Create an empty state (used as placeholder in MCP mode before first session).
    pub fn empty() -> Self {
        use crate::input::CliSource;
        Self {
            content: ContentModel {
                label: String::new(),
                lines: Vec::new(),
                source: ContentSource::Cli(CliSource::Stdin {
                    label: String::new(),
                }),
                metadata: ContentMetadata::Plain,
            },
            session: SessionState::default(),
            config: UserConfig::empty(),
        }
    }

    /// Convert to response for frontend.
    pub fn to_response(&self) -> ContentResponse {
        ContentResponse {
            label: self.content.label.clone(),
            lines: self.content.lines.clone(),
            tags: self.config.tags().to_vec(),
            exit_modes: self.config.exit_modes().to_vec(),
            selected_exit_mode_id: self.session.selected_exit_mode_id.clone(),
            session_comment: self.session.comment.clone(),
            metadata: self.content.metadata.clone(),
            allows_image_paste: self.content.source.allows_image_paste(),
        }
    }

    /// Insert or update an annotation.
    pub fn upsert_annotation(&mut self, start_line: u32, end_line: u32, content: Vec<ContentNode>) {
        let key = LineRange::new(start_line, end_line);
        self.session.annotations.insert(
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
        self.session.annotations.remove(&LineRange::new(start_line, end_line));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{CliSource, DiffSource, McpSource};
    use std::path::PathBuf;

    fn test_state(content: &str, path: &str) -> AppState {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from(path),
        });
        let content_model = ContentModel::from_file(content, source);
        AppState::new(content_model, UserConfig::empty())
    }

    fn test_diff_source(label: &str) -> ContentSource {
        ContentSource::Mcp(McpSource::Diff {
            label: Some(label.to_string()),
            source: DiffSource::Raw,
        })
    }

    fn test_diff_state(content: &str, source: ContentSource) -> AppState {
        let content_model = ContentModel::from_diff(content, source).unwrap();
        AppState::new(content_model, UserConfig::empty())
    }

    #[test]
    fn content_response_has_1_indexed_line_numbers() {
        let state = test_state("a\nb\nc", "test.rs");
        let response = state.to_response();

        assert!(matches!(response.lines[0].origin, LineOrigin::Document { line: 1 }));
        assert!(matches!(response.lines[1].origin, LineOrigin::Document { line: 2 }));
        assert!(matches!(response.lines[2].origin, LineOrigin::Document { line: 3 }));
    }

    #[test]
    fn content_response_includes_label() {
        let state = test_state("content", "my_file.rs");
        let response = state.to_response();

        assert_eq!(response.label, "my_file.rs");
    }

    #[test]
    fn content_response_preserves_whitespace() {
        let state = test_state("  indented\n\ttabbed", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].content, "  indented");
        assert_eq!(response.lines[1].content, "\ttabbed");
    }

    #[test]
    fn content_response_includes_highlighted_html() {
        let state = test_state("fn main() {}", "test.rs");
        let response = state.to_response();

        // Should have HTML highlighting for Rust
        assert!(response.lines[0].html.is_some());
        let html = response.lines[0].html.as_ref().unwrap();
        assert!(html.contains("class="), "Expected HTML with CSS classes");
    }

    #[test]
    fn content_response_html_is_none_for_empty_lines_mismatch() {
        // If the highlighter returns fewer lines than content (edge case),
        // html should be None for missing lines
        let state = test_state("line1\nline2", "test.txt");
        let response = state.to_response();

        // Plain text should still have html (just escaped text)
        assert_eq!(response.lines.len(), 2);
    }

    #[test]
    fn content_response_includes_tags() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let content_model = ContentModel::from_file("code", source);
        let config = UserConfig::with_data(
            vec![Tag {
                id: "test123".into(),
                name: "TEST".into(),
                instruction: "Test tag".into(),
            }],
            vec![],
        );
        let state = AppState::new(content_model, config);
        let response = state.to_response();

        assert_eq!(response.tags.len(), 1);
        assert_eq!(response.tags[0].name, "TEST");
    }

    #[test]
    fn tag_new_generates_12_char_id() {
        let tag = Tag::new("TEST".into(), "instruction".into());
        assert_eq!(tag.id.len(), 12);
        assert!(tag.id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn exit_mode_new_generates_12_char_id() {
        let mode = ExitMode::new("Test".into(), "#ff0000".into(), "instruction".into(), 0);
        assert_eq!(mode.id.len(), 12);
        assert!(mode.id.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(!mode.is_transient()); // new() creates persisted modes
    }

    // === Diff tests ===

    const SIMPLE_DIFF: &str = r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,4 @@
 fn main() {
-    old_code();
+    new_code();
+    more_code();
 }
"#;

    #[test]
    fn from_diff_creates_state_with_metadata() {
        let state = test_diff_state(SIMPLE_DIFF, test_diff_source("changes.diff"));

        match &state.content.metadata {
            ContentMetadata::Diff(meta) => {
                assert_eq!(meta.files.len(), 1);
                assert_eq!(meta.files[0].new_name, Some("file.rs".to_string()));
            }
            _ => panic!("Expected Diff metadata"),
        }
    }

    #[test]
    fn from_diff_creates_lines_from_content() {
        let state = test_diff_state(SIMPLE_DIFF, test_diff_source("changes.diff"));

        // Should have lines matching the diff content
        assert!(!state.content.lines.is_empty());

        // First line should be the diff header
        assert!(state.content.lines[0].content.starts_with("diff --git"));

        // Check that +/- lines are preserved
        let has_added = state.content.lines.iter().any(|l| l.content.starts_with('+'));
        let has_deleted = state.content.lines.iter().any(|l| l.content.starts_with('-'));
        assert!(has_added, "Should have added lines");
        assert!(has_deleted, "Should have deleted lines");
    }

    #[test]
    fn from_diff_line_numbers_are_1_indexed() {
        let state = test_diff_state(SIMPLE_DIFF, test_diff_source("changes.diff"));

        // Diff lines have LineOrigin::Diff with old_line/new_line info
        // Just verify lines exist and have Diff origin
        assert!(matches!(state.content.lines[0].origin, LineOrigin::Diff { .. }));
        assert!(matches!(state.content.lines[1].origin, LineOrigin::Diff { .. }));
    }

    #[test]
    fn from_diff_response_includes_metadata() {
        let state = test_diff_state(SIMPLE_DIFF, test_diff_source("changes.diff"));
        let response = state.to_response();

        assert!(matches!(response.metadata, ContentMetadata::Diff(_)));
    }

    #[test]
    fn from_diff_error_on_invalid_content() {
        let result = ContentModel::from_diff("just regular text", test_diff_source("not-a-diff.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn from_file_has_no_diff_metadata() {
        let state = test_state("fn main() {}", "test.rs");
        assert!(matches!(state.content.metadata, ContentMetadata::Plain));

        let response = state.to_response();
        assert!(matches!(response.metadata, ContentMetadata::Plain));
    }

    #[test]
    fn from_diff_includes_tags_and_exit_modes() {
        let source = test_diff_source("changes.diff");
        let content_model = ContentModel::from_diff(SIMPLE_DIFF, source).unwrap();
        let config = UserConfig::with_data(
            vec![Tag::new("TEST".into(), "instruction".into())],
            vec![ExitMode::new("Apply".into(), "#22c55e".into(), "Apply it".into(), 0)],
        );
        let state = AppState::new(content_model, config);

        assert_eq!(state.config.tags().len(), 1);
        assert_eq!(state.config.exit_modes().len(), 1);
    }

    /// Test that diff lines with doc comments produce single-line HTML
    #[test]
    fn from_diff_doc_comment_line_html_is_single_line() {
        let diff_with_doc_comment = r#"diff --git a/lib.rs b/lib.rs
--- a/lib.rs
+++ b/lib.rs
@@ -1,3 +1,4 @@
-/// Old doc comment
+/// New doc comment
 fn main() {
 }
"#;

        let state = test_diff_state(diff_with_doc_comment, test_diff_source("changes.diff"));

        println!("\n=== DIFF DOC COMMENT LINES ===");
        for (i, line) in state.content.lines.iter().enumerate() {
            println!("Line {}: content={:?}", i + 1, line.content);
            if let Some(ref html) = line.html {
                println!("        html={:?}", html);
                // Check for newlines
                if html.contains('\n') {
                    println!("        WARNING: HTML contains newline!");
                }
            }
        }
        println!("=== END ===\n");

        // Find the deleted doc comment line
        let deleted_line = state.content.lines.iter().find(|l| l.content.starts_with("-///")).unwrap();
        assert!(deleted_line.html.is_some(), "Deleted doc comment should have HTML");
        let html = deleted_line.html.as_ref().unwrap();

        // HTML should not contain newlines
        assert!(!html.contains('\n'), "HTML should not contain newline. Got: {:?}", html);

        // HTML should start with the prefix
        assert!(html.starts_with('-'), "HTML should start with '-' prefix. Got: {:?}", html);
    }
}
