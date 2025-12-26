//! Markdown parsing, rendering, and table formatting.
//!
//! Uses pulldown-cmark's event stream architecture for extensibility.

use std::path::Path;

/// SVG icon for portal references (inline file/page icon).
const PORTAL_REF_ICON: &str = r#"<svg class="portal-ref-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>"#;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;

/// Metadata extracted from a markdown document.
#[derive(Clone, Debug, Serialize)]
pub struct MarkdownMetadata {
    /// Heading sections for breadcrumb navigation.
    pub sections: Vec<SectionInfo>,
    /// Fenced code blocks for syntax highlighting.
    pub code_blocks: Vec<CodeBlockInfo>,
    /// Tables for column auto-alignment.
    pub tables: Vec<TableInfo>,
    /// Portal links (links with line anchors like `#L42-L58`).
    pub portals: Vec<PortalInfo>,
}

/// A heading section in the document.
#[derive(Clone, Debug, Serialize)]
pub struct SectionInfo {
    /// 1-indexed line number in source.
    pub source_line: u32,
    /// Heading level (1-6 for h1-h6).
    pub level: u8,
    /// Plain text title.
    pub title: String,
    /// Index of parent section (for breadcrumb), None if top-level.
    pub parent_index: Option<usize>,
}

/// A fenced code block.
#[derive(Clone, Debug, Serialize)]
pub struct CodeBlockInfo {
    /// 1-indexed start line in source.
    pub start_line: u32,
    /// 1-indexed end line in source.
    pub end_line: u32,
    /// Language hint (e.g., "rust", "python"), None if not specified.
    pub language: Option<String>,
}

/// A table for column auto-alignment.
#[derive(Clone, Debug, Serialize)]
pub struct TableInfo {
    /// 1-indexed start line in source.
    pub start_line: u32,
    /// 1-indexed end line in source.
    pub end_line: u32,
    /// Reformatted lines with aligned columns.
    pub formatted_lines: Vec<String>,
}

/// A portal link detected in markdown.
/// Portal links are markdown links with line anchors: `[label](path#L42-L58)`
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PortalInfo {
    /// 1-indexed line in markdown where the link appears.
    pub source_line: u32,
    /// Link text, or None to use filename as label.
    pub label: Option<String>,
    /// Raw path from the link (before `#`).
    pub path: String,
    /// Start line of the range (1-indexed).
    pub start_line: u32,
    /// End line of the range (1-indexed).
    pub end_line: u32,
}

// =============================================================================
// Markdown semantics and rendering
// =============================================================================

/// Markdown structural semantics for a line.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarkdownSemantics {
    Header { level: u8 },
    CodeBlockStart { language: Option<String> },
    CodeBlockContent,
    CodeBlockEnd,
    TableRow,
    ListItem { ordered: bool },
    BlockQuote,
    HorizontalRule,
}

/// Result of rendering a markdown line.
pub struct RenderedLine {
    /// HTML-rendered content.
    pub html: String,
    /// Semantic classification of the line, if it has markdown structure.
    pub semantics: Option<MarkdownSemantics>,
}

// =============================================================================
// Parser state management
// =============================================================================

/// A parsing context we're currently inside.
#[derive(Debug, Clone)]
enum ParseContext {
    Heading { line: u32, level: u8, text: String },
    CodeBlock { line: u32, lang: Option<String> },
    Table { start_line: u32 },
    PortalLink { line: u32, url: String, text: String },
}

/// Stack-based parser state tracker.
#[derive(Debug, Default)]
struct ParseState {
    stack: Vec<ParseContext>,
}

impl ParseState {
    fn push(&mut self, ctx: ParseContext) {
        self.stack.push(ctx);
    }

    /// Pop heading context if it's on top.
    fn pop_heading(&mut self) -> Option<(u32, u8, String)> {
        match self.stack.last() {
            Some(ParseContext::Heading { .. }) => {
                if let Some(ParseContext::Heading { line, level, text }) = self.stack.pop() {
                    return Some((line, level, text));
                }
            }
            _ => {}
        }
        None
    }

    /// Pop code block context if it's on top.
    fn pop_code_block(&mut self) -> Option<(u32, Option<String>)> {
        match self.stack.last() {
            Some(ParseContext::CodeBlock { .. }) => {
                if let Some(ParseContext::CodeBlock { line, lang }) = self.stack.pop() {
                    return Some((line, lang));
                }
            }
            _ => {}
        }
        None
    }

    /// Pop table context if it's on top.
    fn pop_table(&mut self) -> Option<u32> {
        match self.stack.last() {
            Some(ParseContext::Table { .. }) => {
                if let Some(ParseContext::Table { start_line }) = self.stack.pop() {
                    return Some(start_line);
                }
            }
            _ => {}
        }
        None
    }

    /// Pop portal link context if it's on top.
    fn pop_portal_link(&mut self) -> Option<(u32, String, String)> {
        match self.stack.last() {
            Some(ParseContext::PortalLink { .. }) => {
                if let Some(ParseContext::PortalLink { line, url, text }) = self.stack.pop() {
                    return Some((line, url, text));
                }
            }
            _ => {}
        }
        None
    }

    /// Check if we're inside a code block.
    fn in_code_block(&self) -> bool {
        self.stack.iter().any(|ctx| matches!(ctx, ParseContext::CodeBlock { .. }))
    }

    /// Get mutable reference to current heading's text accumulator.
    fn current_heading_text(&mut self) -> Option<&mut String> {
        self.stack.iter_mut().rev().find_map(|ctx| {
            if let ParseContext::Heading { text, .. } = ctx {
                Some(text)
            } else {
                None
            }
        })
    }

    /// Get mutable reference to current portal link's text accumulator.
    fn current_portal_text(&mut self) -> Option<&mut String> {
        self.stack.iter_mut().rev().find_map(|ctx| {
            if let ParseContext::PortalLink { text, .. } = ctx {
                Some(text)
            } else {
                None
            }
        })
    }
}

/// Check if a file is markdown based on extension.
pub fn is_markdown(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    matches!(ext.as_deref(), Some("md" | "markdown" | "mdown" | "mkd"))
}

/// Parse markdown content and extract metadata.
pub fn parse_markdown(content: &str) -> MarkdownMetadata {
    let mapper = LineMapper::new(content);
    let options = markdown_options();
    let parser = Parser::new_ext(content, options).into_offset_iter();

    let mut sections = Vec::new();
    let mut code_blocks = Vec::new();
    let mut tables = Vec::new();
    let mut portals = Vec::new();

    let mut state = ParseState::default();

    for (event, range) in parser {
        let line = mapper.byte_to_line(range.start);

        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                state.push(ParseContext::Heading {
                    line,
                    level: heading_level_to_u8(level),
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((source_line, level, text)) = state.pop_heading() {
                    sections.push(SectionInfo {
                        source_line,
                        level,
                        title: text.trim().to_string(),
                        parent_index: None,
                    });
                }
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info.split(&[',', ' '][..]).next().unwrap_or("");
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
                state.push(ParseContext::CodeBlock { line, lang: language });
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((start_line, language)) = state.pop_code_block() {
                    code_blocks.push(CodeBlockInfo {
                        start_line,
                        end_line: mapper.byte_to_line(range.end.saturating_sub(1)),
                        language,
                    });
                }
            }

            Event::Start(Tag::Table(_)) => {
                state.push(ParseContext::Table { start_line: line });
            }
            Event::End(TagEnd::Table) => {
                if let Some(start_line) = state.pop_table() {
                    let end_line = mapper.byte_to_line(range.end.saturating_sub(1));
                    let table_content = extract_table_lines(content, start_line, end_line);
                    let formatted = format_table(&table_content);
                    tables.push(TableInfo {
                        start_line,
                        end_line,
                        formatted_lines: formatted,
                    });
                }
            }

            // Portal link detection: [label](path#L42-L58)
            Event::Start(Tag::Link { dest_url, .. }) if !state.in_code_block() => {
                if parse_line_anchor(&dest_url).is_some() {
                    state.push(ParseContext::PortalLink {
                        line,
                        url: dest_url.to_string(),
                        text: String::new(),
                    });
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((source_line, url, text)) = state.pop_portal_link() {
                    if let Some((path, start_line, end_line)) = parse_portal_url(&url) {
                        let label = if text.trim().is_empty() {
                            None
                        } else {
                            Some(text.trim().to_string())
                        };
                        portals.push(PortalInfo {
                            source_line,
                            label,
                            path,
                            start_line,
                            end_line,
                        });
                    }
                }
            }

            // Text accumulation for headings and portal links
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading_text) = state.current_heading_text() {
                    heading_text.push_str(&text);
                }
                if let Some(portal_text) = state.current_portal_text() {
                    portal_text.push_str(&text);
                }
            }

            _ => {}
        }
    }

    // Build parent chain for sections
    build_section_hierarchy(&mut sections);

    MarkdownMetadata {
        sections,
        code_blocks,
        tables,
        portals,
    }
}

/// Helper to convert HeadingLevel to u8.
fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Maps byte offsets to line numbers.
struct LineMapper {
    line_starts: Vec<usize>,
}

impl LineMapper {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Convert byte offset to 1-indexed line number.
    fn byte_to_line(&self, offset: usize) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(idx) => (idx + 1) as u32,
            Err(idx) => idx as u32,
        }
    }
}

/// Create pulldown-cmark options with GFM extensions.
fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
}

/// Parse a line anchor from URL: `#L42` or `#L42-L58`
///
/// Returns `Some((start, end))` if valid, `None` otherwise.
/// Handles:
/// - Single line: `#L42` → (42, 42)
/// - Range: `#L42-L58` → (42, 58)
/// - Inverted: `#L58-L42` → normalized to (42, 58)
/// - Case insensitive: `#l42-L58`
/// - Invalid: `#L0` (1-indexed), no anchor, external URLs
fn parse_line_anchor(url: &str) -> Option<(u32, u32)> {
    // Find the anchor part
    let anchor_start = url.rfind('#')?;
    let anchor = &url[anchor_start + 1..];

    // Must start with L or l
    if !anchor.starts_with('L') && !anchor.starts_with('l') {
        return None;
    }

    let rest = &anchor[1..]; // Skip the 'L'

    // Parse first line number
    let (start_str, remainder) = if let Some(dash_pos) = rest.find('-') {
        (&rest[..dash_pos], Some(&rest[dash_pos + 1..]))
    } else {
        (rest, None)
    };

    let start: u32 = start_str.parse().ok()?;
    if start == 0 {
        return None; // 1-indexed, L0 is invalid
    }

    let end = if let Some(end_part) = remainder {
        // Must start with L or l
        let end_part = if end_part.starts_with('L') || end_part.starts_with('l') {
            &end_part[1..]
        } else {
            end_part // Allow #L42-58 format too
        };

        let e: u32 = end_part.parse().ok()?;
        if e == 0 {
            return None;
        }
        e
    } else {
        start
    };

    // Normalize: ensure start <= end
    Some((start.min(end), start.max(end)))
}

/// Parse a portal URL into (path, start_line, end_line).
///
/// Example: `path/to/file.rs#L42-L58` → ("path/to/file.rs", 42, 58)
fn parse_portal_url(url: &str) -> Option<(String, u32, u32)> {
    let anchor_start = url.rfind('#')?;
    let path = &url[..anchor_start];

    if path.is_empty() {
        return None;
    }

    let (start, end) = parse_line_anchor(url)?;
    Some((path.to_string(), start, end))
}

/// Extract table lines from content.
fn extract_table_lines(content: &str, start_line: u32, end_line: u32) -> Vec<String> {
    content
        .lines()
        .skip((start_line - 1) as usize)
        .take((end_line - start_line + 1) as usize)
        .map(|s| s.to_string())
        .collect()
}

/// Format table with aligned columns.
pub fn format_table(lines: &[String]) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Parse cells from each row
    let rows: Vec<Vec<String>> = lines
        .iter()
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    if rows.is_empty() {
        return lines.to_vec();
    }

    // Find maximum width for each column
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; col_count];

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                // Use Unicode width for proper alignment
                let width = unicode_display_width(cell);
                col_widths[i] = col_widths[i].max(width);
            }
        }
    }

    // Detect separator row (contains only dashes and colons)
    let is_separator = |row: &Vec<String>| {
        row.iter().all(|cell| {
            let trimmed = cell.trim();
            !trimmed.is_empty()
                && trimmed
                    .chars()
                    .all(|c| c == '-' || c == ':' || c == ' ')
        })
    };

    // Format each row
    rows.iter()
        .map(|row| {
            let formatted_cells: Vec<String> = (0..col_count)
                .map(|i| {
                    let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    let width = col_widths.get(i).copied().unwrap_or(3);

                    if is_separator(&row) {
                        // Separator row: preserve alignment markers
                        let has_left = cell.starts_with(':');
                        let has_right = cell.ends_with(':');
                        let dashes = "-".repeat(width.max(3));
                        match (has_left, has_right) {
                            (true, true) => format!(":{:-<width$}:", "", width = width.saturating_sub(2).max(1)),
                            (true, false) => format!(":{:-<width$}", "", width = width.saturating_sub(1).max(1)),
                            (false, true) => format!("{:-<width$}:", "", width = width.saturating_sub(1).max(1)),
                            (false, false) => dashes,
                        }
                    } else {
                        // Regular cell: pad to width
                        pad_to_width(cell, width)
                    }
                })
                .collect();

            format!("| {} |", formatted_cells.join(" | "))
        })
        .collect()
}

/// Calculate display width accounting for Unicode characters.
fn unicode_display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    s.width()
}

/// Pad string to target display width.
fn pad_to_width(s: &str, width: usize) -> String {
    let current_width = unicode_display_width(s);
    if current_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - current_width))
    }
}

/// Build parent chain for sections based on heading levels.
fn build_section_hierarchy(sections: &mut [SectionInfo]) {
    // Stack of (index, level) for finding parents
    let mut stack: Vec<(usize, u8)> = Vec::new();

    for i in 0..sections.len() {
        let level = sections[i].level;

        // Pop sections at same or deeper level
        while let Some(&(_, parent_level)) = stack.last() {
            if parent_level >= level {
                stack.pop();
            } else {
                break;
            }
        }

        // Parent is top of stack (if any)
        sections[i].parent_index = stack.last().map(|&(idx, _)| idx);

        // Push current section
        stack.push((i, level));
    }
}

// =============================================================================
// Markdown line rendering
// =============================================================================

/// HTML-escape a string for safe display.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Render a markdown line with structural markers preserved.
///
/// Structural markers (`#`, `-`, `>`, etc.) are preserved as escaped text.
/// Only inline content (bold, italic, links, code) is rendered as HTML.
pub fn render_line(line: &str) -> RenderedLine {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let indent_str = &line[..indent];

    // Headers: # Title -> styled "# " + inline_render("Title")
    if let Some(level) = detect_header_level(trimmed) {
        let hashes = &trimmed[..level as usize];
        let content = trimmed[level as usize..].trim_start();
        let html = format!(
            "<span class=\"md md-h{}\">{}<span class=\"md-header-level\">{}</span> {}</span>",
            level,
            html_escape(indent_str),
            html_escape(hashes),
            render_inline(content)
        );
        return RenderedLine {
            html,
            semantics: Some(MarkdownSemantics::Header { level }),
        };
    }

    // Blockquotes: > text -> "> " + inline_render("text")
    if trimmed.starts_with('>') {
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
            render_inline(content)
        );
        return RenderedLine {
            html,
            semantics: Some(MarkdownSemantics::BlockQuote),
        };
    }

    // Unordered list: - item or * item
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let marker = &trimmed[..2];
        let content = &trimmed[2..];
        let html = format!(
            "<span class=\"md md-list\">{}{}{}</span>",
            html_escape(indent_str),
            html_escape(marker),
            render_inline(content)
        );
        return RenderedLine {
            html,
            semantics: Some(MarkdownSemantics::ListItem { ordered: false }),
        };
    }

    // Ordered list: 1. item, 2. item, etc.
    if let Some(marker_len) = detect_ordered_list_marker_len(trimmed) {
        let marker = &trimmed[..marker_len];
        let content = &trimmed[marker_len..];
        let html = format!(
            "<span class=\"md md-list\">{}{}{}</span>",
            html_escape(indent_str),
            html_escape(marker),
            render_inline(content)
        );
        return RenderedLine {
            html,
            semantics: Some(MarkdownSemantics::ListItem { ordered: true }),
        };
    }

    // Horizontal rule: ---, ***, ___
    if is_horizontal_rule(trimmed) {
        let html = format!("<span class=\"md md-hr\">{}</span>", html_escape(line));
        return RenderedLine {
            html,
            semantics: Some(MarkdownSemantics::HorizontalRule),
        };
    }

    // Regular text: render inline markdown
    let html = format!("<span class=\"md\">{}</span>", render_inline(line));
    RenderedLine {
        html,
        semantics: None,
    }
}

/// Extract filename from a path string (strips line anchor and directory components).
fn filename_from_path(path: &str) -> &str {
    let without_anchor = path.split('#').next().unwrap_or(path);
    std::path::Path::new(without_anchor)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(without_anchor)
}

/// Render inline markdown (bold, italic, links, code) to HTML.
pub fn render_inline(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let options = markdown_options();
    let parser = Parser::new_ext(text, options);

    let mut output = String::new();
    let mut portal_path: Option<String> = None; // Track if we're inside a portal link

    for event in parser {
        match event {
            // Text: escape and emit
            Event::Text(t) => {
                output.push_str(&html_escape(&t));
            }

            // Strong (bold): **text**
            Event::Start(Tag::Strong) => {
                output.push_str("<strong>");
            }
            Event::End(TagEnd::Strong) => {
                output.push_str("</strong>");
            }

            // Emphasis (italic): *text*
            Event::Start(Tag::Emphasis) => {
                output.push_str("<em>");
            }
            Event::End(TagEnd::Emphasis) => {
                output.push_str("</em>");
            }

            // Inline code: `code`
            Event::Code(code) => {
                output.push_str("<code>");
                output.push_str(&html_escape(&code));
                output.push_str("</code>");
            }

            // Links: [text](url)
            // Portal links (with line anchors) get special styling as spans
            Event::Start(Tag::Link { dest_url, .. }) => {
                if parse_line_anchor(&dest_url).is_some() {
                    portal_path = Some(dest_url.to_string());
                    output.push_str("<span class=\"portal-ref\">");
                    output.push_str(PORTAL_REF_ICON);
                } else {
                    output.push_str("<a href=\"");
                    output.push_str(&html_escape(&dest_url));
                    output.push_str("\">");
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some(path) = portal_path.take() {
                    // If no link text was provided, use filename as label
                    // After the icon, output ends with "</svg>" if no text was added
                    if output.ends_with("</svg>") {
                        output.push_str(filename_from_path(&path));
                    }
                    output.push_str("</span>");
                } else {
                    output.push_str("</a>");
                }
            }

            // Strikethrough: ~~text~~
            Event::Start(Tag::Strikethrough) => {
                output.push_str("<del>");
            }
            Event::End(TagEnd::Strikethrough) => {
                output.push_str("</del>");
            }

            // Soft/hard breaks
            Event::SoftBreak | Event::HardBreak => {
                output.push(' ');
            }

            // Skip block elements (paragraph wrappers, etc.)
            Event::Start(Tag::Paragraph)
            | Event::End(TagEnd::Paragraph)
            | Event::Start(Tag::BlockQuote(_))
            | Event::End(TagEnd::BlockQuote(_))
            | Event::Start(Tag::List(_))
            | Event::End(TagEnd::List(_))
            | Event::Start(Tag::Item)
            | Event::End(TagEnd::Item)
            | Event::Start(Tag::Heading { .. })
            | Event::End(TagEnd::Heading(_)) => {}

            // Skip other events
            _ => {}
        }
    }

    output
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
        Some(len + 2)
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_markdown_detects_md_extension() {
        assert!(is_markdown("README.md"));
        assert!(is_markdown("docs/guide.markdown"));
        assert!(is_markdown("notes.mdown"));
        assert!(is_markdown("file.mkd"));
    }

    #[test]
    fn is_markdown_rejects_non_markdown() {
        assert!(!is_markdown("file.rs"));
        assert!(!is_markdown("file.txt"));
        assert!(!is_markdown("file"));
    }

    #[test]
    fn parse_markdown_extracts_headings() {
        let content = "# Title\n\nSome text\n\n## Section 1\n\nMore text\n\n### Subsection\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.sections.len(), 3);
        assert_eq!(meta.sections[0].title, "Title");
        assert_eq!(meta.sections[0].level, 1);
        assert_eq!(meta.sections[1].title, "Section 1");
        assert_eq!(meta.sections[1].level, 2);
        assert_eq!(meta.sections[2].title, "Subsection");
        assert_eq!(meta.sections[2].level, 3);
    }

    #[test]
    fn parse_markdown_builds_hierarchy() {
        let content = "# Title\n\n## Section 1\n\n### Sub 1.1\n\n## Section 2\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.sections.len(), 4);
        // Title (h1) has no parent
        assert_eq!(meta.sections[0].parent_index, None);
        // Section 1 (h2) parent is Title
        assert_eq!(meta.sections[1].parent_index, Some(0));
        // Sub 1.1 (h3) parent is Section 1
        assert_eq!(meta.sections[2].parent_index, Some(1));
        // Section 2 (h2) parent is Title
        assert_eq!(meta.sections[3].parent_index, Some(0));
    }

    #[test]
    fn parse_markdown_extracts_code_blocks() {
        let content = "# Title\n\n```rust\nfn main() {}\n```\n\nText\n\n```python\nprint('hi')\n```\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.code_blocks.len(), 2);
        assert_eq!(meta.code_blocks[0].language, Some("rust".to_string()));
        assert_eq!(meta.code_blocks[1].language, Some("python".to_string()));
    }

    #[test]
    fn parse_markdown_handles_code_block_without_language() {
        let content = "```\nplain code\n```\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.code_blocks.len(), 1);
        assert_eq!(meta.code_blocks[0].language, None);
    }

    #[test]
    fn parse_markdown_extracts_tables() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.tables.len(), 1);
        assert!(!meta.tables[0].formatted_lines.is_empty());
    }

    #[test]
    fn format_table_aligns_columns() {
        let lines = vec![
            "| Name | Age |".to_string(),
            "|---|---|".to_string(),
            "| Alice | 30 |".to_string(),
            "| Bob | 25 |".to_string(),
        ];

        let formatted = format_table(&lines);

        assert_eq!(formatted.len(), 4);
        // All lines should have same length due to alignment
        let lengths: Vec<usize> = formatted.iter().map(|l| l.len()).collect();
        assert!(
            lengths.windows(2).all(|w| w[0] == w[1]),
            "All lines should have same length: {:?}",
            formatted
        );
    }

    #[test]
    fn format_table_handles_varying_widths() {
        let lines = vec![
            "| Short | A Very Long Column Name |".to_string(),
            "|---|---|".to_string(),
            "| X | Y |".to_string(),
        ];

        let formatted = format_table(&lines);

        // Second column should be padded to match header width
        assert!(formatted[2].contains("Y "));
    }

    #[test]
    fn format_table_preserves_alignment_markers() {
        let lines = vec![
            "| Left | Center | Right |".to_string(),
            "|:---|:---:|---:|".to_string(),
            "| a | b | c |".to_string(),
        ];

        let formatted = format_table(&lines);

        // Separator row should preserve alignment markers
        assert!(formatted[1].contains(":"), "Should preserve colons: {:?}", formatted[1]);
    }

    #[test]
    fn unicode_display_width_handles_ascii() {
        assert_eq!(unicode_display_width("hello"), 5);
    }

    #[test]
    fn unicode_display_width_handles_cjk() {
        // CJK characters are double-width
        assert_eq!(unicode_display_width("日本"), 4);
    }

    #[test]
    fn unicode_display_width_handles_checkmarks() {
        // Checkmarks and X marks are single-width
        assert_eq!(unicode_display_width("✓"), 1);
        assert_eq!(unicode_display_width("✗"), 1);
        assert_eq!(unicode_display_width("✓✗"), 2);
    }

    #[test]
    fn unicode_display_width_handles_emoji() {
        // Common emoji widths
        assert_eq!(unicode_display_width("🚀"), 2);
        assert_eq!(unicode_display_width("✅"), 2);
        assert_eq!(unicode_display_width("⚠️"), 2); // warning sign with variation selector
    }

    #[test]
    fn section_line_numbers_are_correct() {
        let content = "# First\n\nSome paragraph.\n\n## Second\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.sections[0].source_line, 1);
        assert_eq!(meta.sections[1].source_line, 5);
    }

    // =========================================================================
    // Inline rendering tests
    // =========================================================================

    #[test]
    fn render_inline_bold() {
        let html = render_inline("**bold text**");
        assert!(html.contains("<strong>bold text</strong>"));
    }

    #[test]
    fn render_inline_italic() {
        let html = render_inline("*italic text*");
        assert!(html.contains("<em>italic text</em>"));
    }

    #[test]
    fn render_inline_bold_and_italic() {
        let html = render_inline("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn render_inline_code() {
        let html = render_inline("use `inline code` here");
        assert!(html.contains("<code>inline code</code>"));
    }

    #[test]
    fn render_inline_link() {
        let html = render_inline("[link text](https://example.com)");
        assert!(html.contains("<a href=\"https://example.com\">link text</a>"));
    }

    #[test]
    fn render_inline_strikethrough() {
        let html = render_inline("~~deleted~~");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn render_inline_escapes_html() {
        let html = render_inline("<script>alert('xss')</script>");
        // Comrak parses this as HTML block which is skipped in inline rendering
        // Just verify no raw script tags in output
        assert!(!html.contains("<script>") || html.contains("&lt;"));
    }

    // =========================================================================
    // Line rendering tests
    // =========================================================================

    #[test]
    fn render_line_heading() {
        let result = render_line("# Title");
        assert!(matches!(result.semantics, Some(MarkdownSemantics::Header { level: 1 })));
        assert!(result.html.contains("md-h1"));
        assert!(result.html.contains("Title"));
    }

    #[test]
    fn render_line_heading_level_2() {
        let result = render_line("## Subtitle");
        assert!(matches!(result.semantics, Some(MarkdownSemantics::Header { level: 2 })));
        assert!(result.html.contains("md-h2"));
    }

    #[test]
    fn render_line_blockquote() {
        let result = render_line("> quoted text");
        assert!(matches!(result.semantics, Some(MarkdownSemantics::BlockQuote)));
        assert!(result.html.contains("md-blockquote"));
    }

    #[test]
    fn render_line_unordered_list() {
        let result = render_line("- list item");
        assert!(matches!(result.semantics, Some(MarkdownSemantics::ListItem { ordered: false })));
        assert!(result.html.contains("md-list"));
    }

    #[test]
    fn render_line_ordered_list() {
        let result = render_line("1. first item");
        assert!(matches!(result.semantics, Some(MarkdownSemantics::ListItem { ordered: true })));
        assert!(result.html.contains("md-list"));
    }

    #[test]
    fn render_line_horizontal_rule() {
        let result = render_line("---");
        assert!(matches!(result.semantics, Some(MarkdownSemantics::HorizontalRule)));
        assert!(result.html.contains("md-hr"));
    }

    #[test]
    fn render_line_plain_text() {
        let result = render_line("Just regular text");
        assert!(result.semantics.is_none());
        assert!(result.html.contains("Just regular text"));
    }

    #[test]
    fn render_line_with_2_space_indent() {
        // 4 spaces triggers code block in markdown, so use 2 spaces
        let result = render_line("  indented text");
        assert!(result.html.contains("indented text"), "HTML should contain 'indented text': {}", result.html);
    }

    #[test]
    fn html_escape_special_chars() {
        assert_eq!(html_escape("<>&\"'"), "&lt;&gt;&amp;&quot;&#x27;");
    }

    // =========================================================================
    // Portal detection tests
    // =========================================================================

    #[test]
    fn parse_line_anchor_single_line() {
        assert_eq!(parse_line_anchor("file.rs#L42"), Some((42, 42)));
    }

    #[test]
    fn parse_line_anchor_range() {
        assert_eq!(parse_line_anchor("file.rs#L42-L58"), Some((42, 58)));
    }

    #[test]
    fn parse_line_anchor_inverted_range() {
        // Should normalize to start <= end
        assert_eq!(parse_line_anchor("file.rs#L58-L42"), Some((42, 58)));
    }

    #[test]
    fn parse_line_anchor_case_insensitive() {
        assert_eq!(parse_line_anchor("file.rs#l42-L58"), Some((42, 58)));
        assert_eq!(parse_line_anchor("file.rs#L42-l58"), Some((42, 58)));
        assert_eq!(parse_line_anchor("file.rs#l42-l58"), Some((42, 58)));
    }

    #[test]
    fn parse_line_anchor_without_second_l() {
        // Allow #L42-58 format (without L before second number)
        assert_eq!(parse_line_anchor("file.rs#L42-58"), Some((42, 58)));
    }

    #[test]
    fn parse_line_anchor_zero_is_invalid() {
        assert_eq!(parse_line_anchor("file.rs#L0"), None);
        assert_eq!(parse_line_anchor("file.rs#L0-L10"), None);
        assert_eq!(parse_line_anchor("file.rs#L10-L0"), None);
    }

    #[test]
    fn parse_line_anchor_no_anchor() {
        assert_eq!(parse_line_anchor("file.rs"), None);
        assert_eq!(parse_line_anchor("https://example.com"), None);
    }

    #[test]
    fn parse_line_anchor_non_line_anchor() {
        // Anchor without L prefix is not a portal
        assert_eq!(parse_line_anchor("file.rs#section"), None);
        assert_eq!(parse_line_anchor("file.rs#42"), None);
    }

    #[test]
    fn parse_portal_url_extracts_path() {
        let result = parse_portal_url("path/to/file.rs#L42-L58");
        assert_eq!(result, Some(("path/to/file.rs".to_string(), 42, 58)));
    }

    #[test]
    fn parse_portal_url_empty_path_is_invalid() {
        assert_eq!(parse_portal_url("#L42"), None);
    }

    #[test]
    fn parse_markdown_extracts_portals() {
        let content = "Check this [code](src/main.rs#L10-L20)\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.portals.len(), 1);
        assert_eq!(meta.portals[0].path, "src/main.rs");
        assert_eq!(meta.portals[0].label, Some("code".to_string()));
        assert_eq!(meta.portals[0].start_line, 10);
        assert_eq!(meta.portals[0].end_line, 20);
    }

    #[test]
    fn parse_markdown_portal_without_label() {
        let content = "[](src/main.rs#L42)\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.portals.len(), 1);
        assert_eq!(meta.portals[0].label, None);
        assert_eq!(meta.portals[0].path, "src/main.rs");
    }

    #[test]
    fn parse_markdown_portal_single_line() {
        let content = "[func](lib.rs#L42)\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.portals.len(), 1);
        assert_eq!(meta.portals[0].start_line, 42);
        assert_eq!(meta.portals[0].end_line, 42);
    }

    #[test]
    fn parse_markdown_multiple_portals() {
        let content = "See [auth](auth.go#L10-L20) and [db](db.go#L30-L40)\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.portals.len(), 2);
        assert_eq!(meta.portals[0].path, "auth.go");
        assert_eq!(meta.portals[1].path, "db.go");
    }

    #[test]
    fn parse_markdown_ignores_non_portal_links() {
        let content = "See [docs](https://example.com) and [local](file.rs)\n";
        let meta = parse_markdown(content);

        // Links without line anchors are not portals
        assert_eq!(meta.portals.len(), 0);
    }

    #[test]
    fn parse_markdown_portal_in_code_block_is_literal() {
        let content = "```\n[code](file.rs#L1-L10)\n```\n";
        let meta = parse_markdown(content);

        // Portal inside code block should be ignored (literal text)
        assert_eq!(meta.portals.len(), 0);
    }

    #[test]
    fn parse_markdown_portal_line_number_correct() {
        let content = "# Header\n\nSome text\n\n[code](file.rs#L1-L10)\n";
        let meta = parse_markdown(content);

        assert_eq!(meta.portals.len(), 1);
        assert_eq!(meta.portals[0].source_line, 5);
    }

    #[test]
    fn render_portal_ref_with_label() {
        let result = render_line("See [auth](src/auth.rs#L10-L20) for details");
        assert!(result.html.contains("portal-ref"), "Should have portal-ref class");
        assert!(result.html.contains("auth"), "Should contain label text");
    }

    #[test]
    fn render_portal_ref_without_label_uses_filename() {
        let result = render_line("See [](src/auth.rs#L10-L20) for details");
        assert!(result.html.contains("portal-ref"), "Should have portal-ref class");
        assert!(result.html.contains("auth.rs"), "Should contain filename as fallback label");
    }

    #[test]
    fn render_portal_ref_without_label_nested_path() {
        let result = render_line("[](deeply/nested/path/file.go#L1-L5)");
        assert!(result.html.contains("file.go"), "Should extract just the filename from nested path");
    }
}
