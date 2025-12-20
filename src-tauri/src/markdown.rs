//! Markdown parsing and table formatting.

use std::path::Path;

use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};
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
    let arena = Arena::new();
    let options = markdown_options();
    let root = parse_document(&arena, content, &options);

    let mut sections = Vec::new();
    let mut code_blocks = Vec::new();
    let mut tables = Vec::new();

    // Build line start positions for accurate line number lookup
    let line_starts = build_line_starts(content);

    extract_nodes(
        root,
        &line_starts,
        &mut sections,
        &mut code_blocks,
        &mut tables,
        content,
    );

    // Build parent chain for sections
    build_section_hierarchy(&mut sections);

    MarkdownMetadata {
        sections,
        code_blocks,
        tables,
    }
}

/// Build line start byte offsets (reserved for future use).
fn build_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, c) in content.char_indices() {
        if c == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Create comrak options with GFM extensions.
fn markdown_options() -> Options {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.parse.smart = true;
    options
}

/// Recursively extract nodes from AST.
fn extract_nodes<'a>(
    node: &'a AstNode<'a>,
    line_starts: &[usize],
    sections: &mut Vec<SectionInfo>,
    code_blocks: &mut Vec<CodeBlockInfo>,
    tables: &mut Vec<TableInfo>,
    content: &str,
) {
    let data = node.data.borrow();

    match &data.value {
        NodeValue::Heading(heading) => {
            // Extract title from heading children
            let title = extract_text_content(node);
            sections.push(SectionInfo {
                source_line: data.sourcepos.start.line as u32,
                level: heading.level,
                title,
                parent_index: None, // Will be filled in build_section_hierarchy
            });
        }
        NodeValue::CodeBlock(code_block) => {
            let lang = if code_block.info.is_empty() {
                None
            } else {
                // Extract language from info string (e.g., "rust" from "rust,linenos")
                Some(code_block.info.split(&[',', ' '][..]).next().unwrap_or("").to_string())
                    .filter(|s| !s.is_empty())
            };
            code_blocks.push(CodeBlockInfo {
                start_line: data.sourcepos.start.line as u32,
                end_line: data.sourcepos.end.line as u32,
                language: lang,
            });
        }
        NodeValue::Table(_) => {
            let start_line = data.sourcepos.start.line as u32;
            let end_line = data.sourcepos.end.line as u32;

            // Extract and format table lines
            let table_content = extract_table_lines(content, start_line, end_line);
            let formatted = format_table(&table_content);

            tables.push(TableInfo {
                start_line,
                end_line,
                formatted_lines: formatted,
            });
        }
        _ => {}
    }

    // Recurse into children
    for child in node.children() {
        extract_nodes(child, line_starts, sections, code_blocks, tables, content);
    }
}

/// Extract plain text content from a node and its children.
fn extract_text_content<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    collect_text(node, &mut text);
    text
}

fn collect_text<'a>(node: &'a AstNode<'a>, text: &mut String) {
    let data = node.data.borrow();
    if let NodeValue::Text(ref t) = data.value {
        text.push_str(t);
    } else if let NodeValue::Code(ref c) = data.value {
        text.push_str(&c.literal);
    }
    for child in node.children() {
        collect_text(child, text);
    }
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
}
