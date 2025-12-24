use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::diff::{self, DiffMetadata};
use crate::highlight::Highlighter;
use crate::input::ContentSource;
use crate::markdown::{self, MarkdownMetadata};

/// Type of line in markdown content for structural styling and UI features.
#[derive(Clone, Debug, Serialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LineType {
    #[default]
    Plain,
    Header {
        level: u8,
    },
    CodeBlockStart {
        language: Option<String>,
    },
    CodeBlockContent,
    CodeBlockEnd,
    TableRow,
    ListItem {
        ordered: bool,
    },
    BlockQuote,
    HorizontalRule,
}

/// A single line of content with its line number.
#[derive(Clone, Serialize)]
pub struct Line {
    pub number: u32,
    pub content: String,
    /// Rendered HTML for display:
    /// - For code blocks: syntect-highlighted spans
    /// - For markdown: inline formatting (bold, italic, etc.)
    /// - None if no rendering needed
    pub html: Option<String>,
    /// Type of line for structural styling and UI features.
    #[serde(flatten)]
    pub line_type: LineType,
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
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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

// ════════════════════════════════════════════════════════════════════════════
// SESSION STATE — mutates during session, not persisted
// ════════════════════════════════════════════════════════════════════════════

/// Session state: mutable data during annotation session.
#[derive(Default)]
pub struct SessionState {
    /// Annotations keyed by "start-end" range string (e.g., "10-15").
    pub annotations: HashMap<String, Annotation>,
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

/// HTML-escape a string for safe display.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Render a markdown line while preserving structural markers.
/// Returns (html, line_type) tuple.
///
/// Structural markers (`#`, `-`, `>`, etc.) are preserved as escaped text.
/// Only inline content (bold, italic, links, code) is rendered as HTML.
fn render_markdown_line(line: &str, options: &comrak::Options) -> (String, LineType) {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let indent_str = &line[..indent];

    // Headers: # Title -> styled "# " + inline_render("Title")
    if let Some(level) = detect_header_level(trimmed) {
        let hashes = &trimmed[..level as usize]; // Just the "#" characters
        let content = &trimmed[level as usize..].trim_start(); // Title after space
        let html = format!(
            "<span class=\"md md-h{}\">{}<span class=\"md-header-level\">{}</span> {}</span>",
            level,
            html_escape(indent_str),
            html_escape(hashes),
            render_inline(content, options)
        );
        return (html, LineType::Header { level });
    }

    // Blockquotes: > text -> "> " + inline_render("text")
    if trimmed.starts_with('>') {
        // Handle "> text" or ">text"
        let content = if trimmed.starts_with("> ") {
            &trimmed[2..]
        } else {
            &trimmed[1..]
        };
        let marker = &trimmed[..trimmed.len() - content.len()];
        let html = format!(
            "<span class=\"md md-blockquote\">{}{}{}</span>",
            html_escape(indent_str),
            html_escape(marker),
            render_inline(content, options)
        );
        return (html, LineType::BlockQuote);
    }

    // Unordered list: - item or * item
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let marker = &trimmed[..2];
        let content = &trimmed[2..];
        let html = format!(
            "<span class=\"md md-list\">{}{}{}</span>",
            html_escape(indent_str),
            html_escape(marker),
            render_inline(content, options)
        );
        return (html, LineType::ListItem { ordered: false });
    }

    // Ordered list: 1. item, 2. item, etc.
    if let Some(marker_len) = detect_ordered_list_marker_len(trimmed) {
        let marker = &trimmed[..marker_len];
        let content = &trimmed[marker_len..];
        let html = format!(
            "<span class=\"md md-list\">{}{}{}</span>",
            html_escape(indent_str),
            html_escape(marker),
            render_inline(content, options)
        );
        return (html, LineType::ListItem { ordered: true });
    }

    // Horizontal rule: ---, ***, ___
    if is_horizontal_rule(trimmed) {
        let html = format!("<span class=\"md md-hr\">{}</span>", html_escape(line));
        return (html, LineType::HorizontalRule);
    }

    // Regular text: render inline markdown
    let html = format!("<span class=\"md\">{}</span>", render_inline(line, options));
    (html, LineType::Plain)
}

/// Get the length of an ordered list marker (e.g., "1. " = 3, "12. " = 4)
fn detect_ordered_list_marker_len(line: &str) -> Option<usize> {
    let mut chars = line.chars().peekable();
    let mut len = 0;

    // Must start with digit
    if !chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return None;
    }

    // Consume digits
    while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        chars.next();
        len += 1;
    }

    // Must be followed by ". "
    if chars.next() == Some('.') && chars.next() == Some(' ') {
        Some(len + 2) // digits + ". "
    } else {
        None
    }
}

/// Detect header level (1-6) from line, or None if not a header.
fn detect_header_level(line: &str) -> Option<u8> {
    let mut level = 0u8;
    for c in line.chars() {
        if c == '#' {
            level += 1;
            if level > 6 {
                return None;
            }
        } else if c == ' ' && level > 0 {
            return Some(level);
        } else {
            return None;
        }
    }
    None
}

/// Check if line is a horizontal rule (---, ***, ___)
fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let first = trimmed.chars().next().unwrap();
    if first != '-' && first != '*' && first != '_' {
        return false;
    }
    trimmed.chars().all(|c| c == first || c == ' ')
}

/// Render only inline markdown (bold, italic, links, inline code) using AST.
/// This preserves the source text and only renders inline formatting as HTML.
fn render_inline(text: &str, options: &comrak::Options) -> String {
    use comrak::{parse_document, Arena};

    if text.is_empty() {
        return String::new();
    }

    let arena = Arena::new();
    let root = parse_document(&arena, text, options);

    let mut output = String::new();
    render_node_inline(root, &mut output);
    output
}

/// Recursively render AST nodes, emitting HTML only for inline elements.
fn render_node_inline<'a>(node: &'a comrak::nodes::AstNode<'a>, output: &mut String) {
    use comrak::nodes::NodeValue;

    let data = node.data.borrow();

    match &data.value {
        // Block elements: skip wrapper, recurse into children
        NodeValue::Document
        | NodeValue::Paragraph
        | NodeValue::BlockQuote
        | NodeValue::List(_)
        | NodeValue::Item(_)
        | NodeValue::Heading(_) => {
            for child in node.children() {
                render_node_inline(child, output);
            }
        }

        // Text: escape and emit
        NodeValue::Text(t) => {
            output.push_str(&html_escape(t));
        }

        // Strong (bold): **text**
        NodeValue::Strong => {
            output.push_str("<strong>");
            for child in node.children() {
                render_node_inline(child, output);
            }
            output.push_str("</strong>");
        }

        // Emphasis (italic): *text*
        NodeValue::Emph => {
            output.push_str("<em>");
            for child in node.children() {
                render_node_inline(child, output);
            }
            output.push_str("</em>");
        }

        // Inline code: `code`
        NodeValue::Code(c) => {
            output.push_str("<code>");
            output.push_str(&html_escape(&c.literal));
            output.push_str("</code>");
        }

        // Links: [text](url)
        NodeValue::Link(link) => {
            output.push_str("<a href=\"");
            output.push_str(&html_escape(&link.url));
            output.push_str("\">");
            for child in node.children() {
                render_node_inline(child, output);
            }
            output.push_str("</a>");
        }

        // Strikethrough: ~~text~~
        NodeValue::Strikethrough => {
            output.push_str("<del>");
            for child in node.children() {
                render_node_inline(child, output);
            }
            output.push_str("</del>");
        }

        // Soft/hard breaks
        NodeValue::SoftBreak | NodeValue::LineBreak => {
            output.push(' ');
        }

        // Skip other node types (code blocks, tables, etc.)
        _ => {}
    }
}

impl ContentModel {
    /// Parse file content into structured lines with syntax highlighting.
    pub fn from_file(content: &str, source: ContentSource) -> Self {
        let label = source.label().to_string();
        let path = source.path_hint().unwrap_or("");

        let highlighter = Highlighter::new();
        let html_lines = highlighter.highlight_lines(content, path);

        let lines = content
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let html = html_lines.get(i).cloned();
                Line {
                    number: (i + 1) as u32,
                    content: line.to_string(),
                    html,
                    line_type: LineType::default(),
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
    pub fn from_diff(content: &str, source: ContentSource) -> Result<Self, String> {
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

                Line {
                    number: line_num,
                    content: line_content.to_string(),
                    html,
                    line_type: LineType::default(),
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
    pub fn from_markdown(content: &str, source: ContentSource) -> Self {
        let label = source.label().to_string();
        use comrak::Options;

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

        // Create comrak options for inline rendering
        let mut options = Options::default();
        options.extension.strikethrough = true;
        options.extension.autolink = true;
        options.render.unsafe_ = true; // Allow raw HTML passthrough

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

                // Determine HTML rendering strategy and line type
                if let Some(info) = code_block_lines.get(&line_num) {
                    // Inside a code block
                    let line_type = if info.is_start {
                        LineType::CodeBlockStart {
                            language: info.language.clone(),
                        }
                    } else if info.is_end {
                        LineType::CodeBlockEnd
                    } else {
                        LineType::CodeBlockContent
                    };

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
                        number: line_num,
                        content: display_content,
                        html,
                        line_type,
                    }
                } else if table_lines.contains(&line_num) {
                    // Table row
                    Line {
                        number: line_num,
                        content: display_content.clone(),
                        html: Some(html_escape(&display_content)),
                        line_type: LineType::TableRow,
                    }
                } else {
                    // Regular markdown: render with structural markers preserved
                    let (html, line_type) = render_markdown_line(line_content, &options);
                    Line {
                        number: line_num,
                        content: display_content,
                        html: Some(html),
                        line_type,
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
        self.session.annotations.insert(
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
        self.session.annotations.remove(&key);
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

        assert_eq!(response.lines[0].number, 1);
        assert_eq!(response.lines[1].number, 2);
        assert_eq!(response.lines[2].number, 3);
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

        assert_eq!(state.content.lines[0].number, 1);
        assert_eq!(state.content.lines[1].number, 2);
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
        for line in &state.content.lines {
            println!("Line {}: content={:?}", line.number, line.content);
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
