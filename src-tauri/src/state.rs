use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::diff::{self, DiffMetadata};
use crate::highlight::Highlighter;
use crate::markdown::{self, MarkdownMetadata};
use crate::perf::timed;

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
    pub is_ephemeral: bool,
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
            is_ephemeral: false,
        }
    }
}

/// Application state initialized at startup, before the window opens.
pub struct AppState {
    pub label: String,
    pub lines: Vec<Line>,
    /// Annotations keyed by "start-end" range string (e.g., "10-15").
    pub annotations: HashMap<String, Annotation>,
    /// Available tags for annotation.
    pub tags: Vec<Tag>,
    /// IDs of tags deleted this session (for merge-on-save).
    pub deleted_tag_ids: HashSet<String>,
    /// Available exit modes for this session.
    pub exit_modes: Vec<ExitMode>,
    /// IDs of exit modes deleted this session (for merge-on-save).
    pub deleted_exit_mode_ids: HashSet<String>,
    /// Currently selected exit mode ID (None if no mode selected).
    pub selected_exit_mode_id: Option<String>,
    /// Session-level comment (not tied to specific lines).
    pub session_comment: Option<Vec<ContentNode>>,
    /// Diff metadata (Some if content is a unified diff).
    pub diff_metadata: Option<DiffMetadata>,
    /// Markdown metadata (Some if content is markdown).
    pub markdown_metadata: Option<MarkdownMetadata>,
    /// Whether this is an ephemeral session (MCP/review_content mode).
    /// When true, image paste is enabled.
    pub ephemeral: bool,
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
    /// Diff metadata (Some if content is a unified diff).
    pub diff_metadata: Option<DiffMetadata>,
    /// Markdown metadata (Some if content is markdown).
    pub markdown_metadata: Option<MarkdownMetadata>,
    /// Whether this is an ephemeral session (enables image paste).
    pub ephemeral: bool,
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

impl AppState {
    /// Create an empty state (used as placeholder in MCP mode before first session).
    pub fn empty() -> Self {
        Self {
            label: String::new(),
            lines: Vec::new(),
            annotations: HashMap::new(),
            tags: Vec::new(),
            deleted_tag_ids: HashSet::new(),
            exit_modes: Vec::new(),
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: None,
            markdown_metadata: None,
            ephemeral: false,
        }
    }

    /// Parse file content into structured lines with syntax highlighting.
    ///
    /// # Arguments
    /// * `label` - Display name (usually the filename)
    /// * `content` - Raw file content
    /// * `path` - File path (used for language detection via extension)
    /// * `tags` - Available tags (loaded from config)
    /// * `exit_modes` - Available exit modes (loaded from config)
    pub fn from_file(
        label: String,
        content: &str,
        path: &str,
        tags: Vec<Tag>,
        exit_modes: Vec<ExitMode>,
    ) -> Self {
        let highlighter = timed!("Highlighter::new", Highlighter::new());
        let html_lines = timed!(
            &format!("highlight_lines({} lines)", content.lines().count()),
            highlighter.highlight_lines(content, path)
        );

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
            annotations: HashMap::new(),
            tags,
            deleted_tag_ids: HashSet::new(),
            exit_modes,
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: None,
            markdown_metadata: None,
            ephemeral: false,
        }
    }

    /// Parse diff content into structured lines with diff metadata.
    ///
    /// # Arguments
    /// * `label` - Display name
    /// * `content` - Raw diff content
    /// * `tags` - Available tags (loaded from config)
    /// * `exit_modes` - Available exit modes (loaded from config)
    pub fn from_diff(
        label: String,
        content: &str,
        tags: Vec<Tag>,
        exit_modes: Vec<ExitMode>,
    ) -> Result<Self, String> {
        let mut diff_metadata = timed!("parse_diff", diff::parse_diff(content)?);
        let highlighter = timed!("Highlighter::new", Highlighter::new());

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
                let html = if !language.is_empty() && !line_content.starts_with("diff ")
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
            annotations: HashMap::new(),
            tags,
            deleted_tag_ids: HashSet::new(),
            exit_modes,
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: Some(diff_metadata),
            markdown_metadata: None,
            ephemeral: false,
        })
    }

    /// Parse markdown content with inline rendering and code block highlighting.
    ///
    /// # Arguments
    /// * `label` - Display name
    /// * `content` - Raw markdown content
    /// * `path` - File path (used for markdown detection)
    /// * `tags` - Available tags (loaded from config)
    /// * `exit_modes` - Available exit modes (loaded from config)
    /// * `ephemeral` - Whether this is ephemeral content (MCP/review_content mode)
    pub fn from_markdown(
        label: String,
        content: &str,
        _path: &str,
        tags: Vec<Tag>,
        exit_modes: Vec<ExitMode>,
        ephemeral: bool,
    ) -> Self {
        use comrak::Options;

        let md_metadata = timed!("parse_markdown", markdown::parse_markdown(content));
        let highlighter = timed!("Highlighter::new", Highlighter::new());

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
            annotations: HashMap::new(),
            tags,
            deleted_tag_ids: HashSet::new(),
            exit_modes,
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: None,
            markdown_metadata: Some(md_metadata),
            ephemeral,
        }
    }

    /// Convert to response for frontend.
    pub fn to_response(&self) -> ContentResponse {
        ContentResponse {
            label: self.label.clone(),
            lines: self.lines.clone(),
            tags: self.tags.clone(),
            exit_modes: self.exit_modes.clone(),
            selected_exit_mode_id: self.selected_exit_mode_id.clone(),
            session_comment: self.session_comment.clone(),
            diff_metadata: self.diff_metadata.clone(),
            markdown_metadata: self.markdown_metadata.clone(),
            ephemeral: self.ephemeral,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(label: &str, content: &str, path: &str) -> AppState {
        AppState::from_file(label.to_string(), content, path, vec![], vec![])
    }

    #[test]
    fn content_response_has_1_indexed_line_numbers() {
        let state = test_state("test.rs", "a\nb\nc", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].number, 1);
        assert_eq!(response.lines[1].number, 2);
        assert_eq!(response.lines[2].number, 3);
    }

    #[test]
    fn content_response_includes_label() {
        let state = test_state("my_file.rs", "content", "my_file.rs");
        let response = state.to_response();

        assert_eq!(response.label, "my_file.rs");
    }

    #[test]
    fn content_response_preserves_whitespace() {
        let state = test_state("test.rs", "  indented\n\ttabbed", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].content, "  indented");
        assert_eq!(response.lines[1].content, "\ttabbed");
    }

    #[test]
    fn content_response_includes_highlighted_html() {
        let state = test_state("test.rs", "fn main() {}", "test.rs");
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
        let state = test_state("test.txt", "line1\nline2", "test.txt");
        let response = state.to_response();

        // Plain text should still have html (just escaped text)
        assert_eq!(response.lines.len(), 2);
    }

    #[test]
    fn content_response_includes_tags() {
        let tags = vec![Tag {
            id: "test123".into(),
            name: "TEST".into(),
            instruction: "Test tag".into(),
        }];
        let state = AppState::from_file("test.rs".into(), "code", "test.rs", tags, vec![]);
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
        assert!(!mode.is_ephemeral);
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
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        assert!(state.diff_metadata.is_some());
        let meta = state.diff_metadata.as_ref().unwrap();
        assert_eq!(meta.files.len(), 1);
        assert_eq!(meta.files[0].new_name, Some("file.rs".to_string()));
    }

    #[test]
    fn from_diff_creates_lines_from_content() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        // Should have lines matching the diff content
        assert!(!state.lines.is_empty());

        // First line should be the diff header
        assert!(state.lines[0].content.starts_with("diff --git"));

        // Check that +/- lines are preserved
        let has_added = state.lines.iter().any(|l| l.content.starts_with('+'));
        let has_deleted = state.lines.iter().any(|l| l.content.starts_with('-'));
        assert!(has_added, "Should have added lines");
        assert!(has_deleted, "Should have deleted lines");
    }

    #[test]
    fn from_diff_line_numbers_are_1_indexed() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        assert_eq!(state.lines[0].number, 1);
        assert_eq!(state.lines[1].number, 2);
    }

    #[test]
    fn from_diff_response_includes_metadata() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        let response = state.to_response();
        assert!(response.diff_metadata.is_some());
    }

    #[test]
    fn from_diff_error_on_invalid_content() {
        let result = AppState::from_diff(
            "not-a-diff.txt".into(),
            "just regular text",
            vec![],
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn from_file_has_no_diff_metadata() {
        let state = test_state("test.rs", "fn main() {}", "test.rs");
        assert!(state.diff_metadata.is_none());

        let response = state.to_response();
        assert!(response.diff_metadata.is_none());
    }

    #[test]
    fn from_diff_includes_tags_and_exit_modes() {
        let tags = vec![Tag::new("TEST".into(), "instruction".into())];
        let modes = vec![ExitMode::new("Apply".into(), "#22c55e".into(), "Apply it".into(), 0)];

        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            tags,
            modes,
        )
        .unwrap();

        assert_eq!(state.tags.len(), 1);
        assert_eq!(state.exit_modes.len(), 1);
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

        let state = AppState::from_diff(
            "changes.diff".into(),
            diff_with_doc_comment,
            vec![],
            vec![],
        )
        .unwrap();

        println!("\n=== DIFF DOC COMMENT LINES ===");
        for line in &state.lines {
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
        let deleted_line = state.lines.iter().find(|l| l.content.starts_with("-///")).unwrap();
        assert!(deleted_line.html.is_some(), "Deleted doc comment should have HTML");
        let html = deleted_line.html.as_ref().unwrap();

        // HTML should not contain newlines
        assert!(!html.contains('\n'), "HTML should not contain newline. Got: {:?}", html);

        // HTML should start with the prefix
        assert!(html.starts_with('-'), "HTML should start with '-' prefix. Got: {:?}", html);
    }
}
