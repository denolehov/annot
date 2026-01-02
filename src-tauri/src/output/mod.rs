//! Output formatting for annotation sessions.
//!
//! This module formats review sessions into structured text for LLM consumption.
//! It uses a declarative OutputBuilder to construct output with automatic
//! indentation and alignment.

mod builder;
mod formatters;
mod render;

use std::collections::{BTreeMap, HashMap};

use crate::lang;
use crate::mcp::tools::SessionImage;
use crate::portal::LoadedPortal;
use crate::review::{FileKey, Review};
use crate::state::{
    Annotation, Bookmark, ContentModel, ContentNode, LineSemantics,
    PortalSemantics,
};

pub use builder::{BuilderMode, OutputBuilder, SECTION_DIVIDER, SEPARATOR};
pub use render::render_content;

use formatters::{calculate_builder_mode, format_annotation, format_bookmark, format_legend, format_session};

/// Output mode determines how content is formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// CLI mode - all data inline in text (images as base64, diagrams as JSON)
    #[default]
    Cli,
    /// MCP mode - images returned separately, omit inline data
    Mcp,
    /// Clipboard mode - [Figure N] placeholders only, no base64 or JSON
    Clipboard,
}

/// Result of formatting output, including text and collected images.
pub struct FormatResult {
    pub text: String,
    pub images: Vec<SessionImage>,
}

/// Reconstructs content for export, with portals embedded as code blocks.
///
/// When content contains portal links (e.g., `[label](file.rs#L10-L20)`),
/// the exported text includes the portal content as fenced code blocks
/// immediately after the source line containing the link.
pub fn export_content(content: &ContentModel) -> String {
    // If no portals, just join all lines
    if content.portals.is_empty() {
        return content
            .lines
            .iter()
            .filter(|line| !matches!(line.semantics, LineSemantics::Portal(_)))
            .map(|l| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Build a map: insert_at (1-indexed line in original markdown) -> portals to insert after
    let mut portal_inserts: HashMap<u32, Vec<&LoadedPortal>> = HashMap::new();
    for portal in &content.portals {
        portal_inserts
            .entry(portal.insert_at)
            .or_default()
            .push(portal);
    }

    let mut result = String::new();
    let mut original_line_num: u32 = 0;

    for line in &content.lines {
        // Skip portal lines (they're interleaved; we'll re-emit them as code blocks)
        if matches!(line.semantics, LineSemantics::Portal(_)) {
            continue;
        }

        // This is an original markdown line
        original_line_num += 1;

        // Emit the line
        result.push_str(&line.content);
        result.push('\n');

        // If there are portals to insert after this line, emit them as code blocks
        if let Some(portals) = portal_inserts.get(&original_line_num) {
            for portal in portals {
                let code_block = format_portal_code_block(portal);
                if !code_block.is_empty() {
                    result.push_str(&code_block);
                }
            }
        }
    }

    // Remove trailing newline if present (to match original behavior)
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Format a portal as a fenced code block with language hint.
fn format_portal_code_block(portal: &LoadedPortal) -> String {
    // Collect only content lines (skip header/footer)
    let content_lines: Vec<&str> = portal
        .lines
        .iter()
        .filter_map(|line| {
            if matches!(line.semantics, LineSemantics::Portal(PortalSemantics::Content)) {
                Some(line.content.as_str())
            } else {
                None
            }
        })
        .collect();

    // Skip empty portals (per user feedback)
    if content_lines.is_empty() {
        return String::new();
    }

    // Detect language from file extension
    let fence_lang = portal
        .source_path
        .extension()
        .and_then(|e| e.to_str())
        .map(lang::extension_to_fence_language)
        .unwrap_or("");

    // Build the code block
    let path_display = portal.source_path.display();
    let range = format!("L{}-L{}", portal.start_line, portal.end_line);

    let mut block = String::new();
    block.push_str(&format!("\n<!-- portal: {}#{} -->\n", path_display, range));
    block.push_str(&format!("```{}\n", fence_lang));
    for line in content_lines {
        block.push_str(line);
        block.push('\n');
    }
    block.push_str("```\n");

    block
}

/// Export a section (line range) from markdown content.
///
/// Like `export_content`, but only includes lines in [start_line, end_line].
/// Portal content is included if the portal link appears within the range.
pub fn export_section(content: &ContentModel, start_line: u32, end_line: u32) -> String {
    // Filter lines by source line number (excluding portal-interleaved lines)
    // Then handle portals whose source link is in range

    // Build a map of portal insert positions within our range
    let mut portal_inserts: HashMap<u32, Vec<&LoadedPortal>> = HashMap::new();
    for portal in &content.portals {
        if portal.insert_at >= start_line && portal.insert_at <= end_line {
            portal_inserts
                .entry(portal.insert_at)
                .or_default()
                .push(portal);
        }
    }

    let mut result = String::new();
    let mut current_line: u32 = 0;

    for line in &content.lines {
        // Skip portal-interleaved lines (we'll re-emit them as code blocks)
        if matches!(line.semantics, LineSemantics::Portal(_)) {
            continue;
        }

        // Track original line number
        current_line += 1;

        // Skip lines outside our range
        if current_line < start_line || current_line > end_line {
            continue;
        }

        // Emit the line
        result.push_str(&line.content);
        result.push('\n');

        // If there are portals to insert after this line, emit them as code blocks
        if let Some(portals) = portal_inserts.get(&current_line) {
            for portal in portals {
                let code_block = format_portal_code_block(portal);
                if !code_block.is_empty() {
                    result.push_str(&code_block);
                }
            }
        }
    }

    // Trim trailing whitespace, blank lines, and separators (---, ___, ***)
    let result = result.trim_end();
    let result = trim_trailing_separators(result);

    result.to_string()
}

/// Trim trailing horizontal rule separators (---, ___, ***) and blank lines.
fn trim_trailing_separators(s: &str) -> &str {
    let mut result = s;
    loop {
        let trimmed = result.trim_end();
        // Check for horizontal rules: 3+ of same char (-, _, *)
        let is_separator = trimmed
            .rsplit('\n')
            .next()
            .map(|last_line| {
                let line = last_line.trim();
                if line.len() < 3 {
                    return false;
                }
                let chars: Vec<char> = line.chars().collect();
                let first = chars[0];
                (first == '-' || first == '_' || first == '*')
                    && chars.iter().all(|&c| c == first || c.is_whitespace())
            })
            .unwrap_or(false);

        if is_separator {
            // Remove the separator line
            if let Some(newline_pos) = trimmed.rfind('\n') {
                result = &trimmed[..newline_pos];
            } else {
                // Entire string is just a separator
                return "";
            }
        } else {
            return trimmed;
        }
    }
}

/// Collect unique tags from all content nodes (session comment + annotations).
/// Returns a BTreeMap for alphabetical ordering by tag name.
fn collect_unique_tags(review: &Review) -> BTreeMap<String, String> {
    let mut tags: BTreeMap<String, String> = BTreeMap::new();

    // Collect from session comment
    if let Some(ref comment) = review.session_comment {
        for node in comment {
            if let ContentNode::Tag {
                name, instruction, ..
            } = node
            {
                tags.insert(name.clone(), instruction.clone());
            }
        }
    }

    // Collect from all file annotations
    for file in review.files.values() {
        for annotation in file.annotations.values() {
            for node in &annotation.content {
                if let ContentNode::Tag {
                    name, instruction, ..
                } = node
                {
                    tags.insert(name.clone(), instruction.clone());
                }
            }
        }
    }

    tags
}

/// Collect unique bookmarks referenced from all content nodes (session comment + annotations).
/// Returns embedded Bookmark data in order of first occurrence (keyed by ID).
fn collect_unique_bookmarks(review: &Review) -> Vec<Bookmark> {
    use indexmap::IndexMap;
    let mut bookmarks: IndexMap<String, Bookmark> = IndexMap::new();

    let mut process_nodes = |nodes: &[ContentNode]| {
        for node in nodes {
            match node {
                ContentNode::BookmarkRef { id, bookmark, .. } => {
                    bookmarks
                        .entry(id.clone())
                        .or_insert_with(|| bookmark.clone());
                }
                ContentNode::Ref { snapshot, .. } => {
                    if let crate::state::RefSnapshot::Bookmark { bookmark } = snapshot {
                        bookmarks
                            .entry(bookmark.id.clone())
                            .or_insert_with(|| bookmark.clone());
                    }
                }
                // Other node types don't contain bookmark references
                ContentNode::Text { .. }
                | ContentNode::Tag { .. }
                | ContentNode::Media { .. }
                | ContentNode::Excalidraw { .. }
                | ContentNode::Replace { .. }
                | ContentNode::Error { .. }
                | ContentNode::Paste { .. }
                | ContentNode::File { .. } => {}
            }
        }
    };

    // Collect from session comment
    if let Some(ref comment) = review.session_comment {
        process_nodes(comment);
    }

    // Collect from all file annotations
    for file in review.files.values() {
        for annotation in file.annotations.values() {
            process_nodes(&annotation.content);
        }
    }

    bookmarks.into_values().collect()
}

/// Format all annotations as structured output for LLM consumption.
pub fn format_output(review: &Review, mode: OutputMode) -> FormatResult {
    // Get content from root_view
    let content = review.root_view.content();

    // Check if ANY file has annotations
    let has_annotations = review
        .files
        .values()
        .any(|target| !target.annotations.is_empty());

    let has_exit_mode = review.selected_exit_mode_id.is_some();
    let has_session_comment = review
        .session_comment
        .as_ref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    let has_saved_to = review.saved_to.is_some();

    if !has_exit_mode && !has_annotations && !has_session_comment && !has_saved_to {
        return FormatResult {
            text: String::new(),
            images: Vec::new(),
        };
    }

    let mut images = Vec::new();
    let mut figure_counter = 0usize;

    // Calculate max line number for builder mode
    let max_line = calculate_max_line(review);
    let builder_mode = calculate_builder_mode(content, max_line);
    let mut out = OutputBuilder::new(builder_mode);

    // LEGEND block (if any tags are used)
    let unique_tags = collect_unique_tags(review);
    if !unique_tags.is_empty() {
        out.section("LEGEND", |b| {
            format_legend(b, &unique_tags);
        });
    }

    // BOOKMARKS REFERENCED block (if any bookmarks are referenced)
    let unique_bookmarks = collect_unique_bookmarks(review);
    if !unique_bookmarks.is_empty() {
        out.section("BOOKMARKS REFERENCED", |b| {
            for bookmark in &unique_bookmarks {
                let created_this_session = review.session_created_bookmarks.contains(&bookmark.id);
                format_bookmark(b, bookmark, created_this_session);
            }
        });
    }

    // SESSION block (if exit mode selected or session comment exists)
    if has_exit_mode || has_session_comment {
        // Prepare session data
        let portals_header = if !content.portals.is_empty() {
            let root_name = &content.label;
            let embedded_files: Vec<_> = content
                .portals
                .iter()
                .map(|p| p.source_path.display().to_string())
                .collect();
            Some(format!(
                "Reviewing {} with embedded files: {}",
                root_name,
                embedded_files.join(", ")
            ))
        } else {
            None
        };

        let session_comment_text = review.session_comment.as_ref().and_then(|comment| {
            if comment.is_empty() {
                None
            } else {
                Some(render_content(comment, &mut images, &mut figure_counter, mode))
            }
        });

        let exit_mode_info = review.selected_exit_mode_id.as_ref().and_then(|mode_id| {
            review
                .config
                .exit_modes()
                .iter()
                .find(|m| &m.id == mode_id)
                .map(|em| (em.name.as_str(), em.instruction.as_str(), em.command_path()))
        });

        let command_content = exit_mode_info.as_ref().and_then(|(_, _, cmd_path)| {
            cmd_path.and_then(|path| {
                std::fs::read_to_string(path)
                    .ok()
                    .map(|content| (path.display().to_string(), content))
            })
        });

        out.section("SESSION", |b| {
            format_session(
                b,
                portals_header.as_deref(),
                session_comment_text.as_deref(),
                exit_mode_info.map(|(name, instr, _)| (name, instr)),
                command_content.as_ref().map(|(p, c)| (p.as_str(), c.as_str())),
            );
        });

        // Remove the trailing blank line from section, add divider if annotations follow
        // Actually section adds blank line, we need divider before annotations
        if has_annotations {
            // Section already added blank line, we need "---" separator
            out.raw("---\n\n");
        }
    }

    // Build annotation blocks (if any)
    if has_annotations {
        let files_with_annotations = collect_files_with_annotations(review);

        let mut first_block = true;
        for (display_path, target) in &files_with_annotations {
            // Sort annotations within this file by start line
            let mut sorted_annotations: Vec<&Annotation> = target.annotations.values().collect();
            sorted_annotations.sort_by_key(|a| a.start_line);

            for ann in sorted_annotations {
                if !first_block {
                    out.divider();
                }
                first_block = false;
                format_annotation(
                    &mut out,
                    content,
                    ann,
                    display_path,
                    &mut images,
                    &mut figure_counter,
                    mode,
                );
            }
        }
    }

    // Saved path (single location, always runs)
    if let Some(ref saved_path) = review.saved_to {
        if !out.is_empty() {
            out.blank_line();
        }
        out.raw_line(&format!("Saved to {}", saved_path.display()));
    }

    FormatResult {
        text: out.build(),
        images,
    }
}

/// Calculate max line number across all annotations.
fn calculate_max_line(review: &Review) -> u32 {
    review
        .files
        .values()
        .flat_map(|target| target.annotations.values())
        .map(|a| a.end_line)
        .max()
        .unwrap_or(0)
}

/// Collect files with annotations in display order.
fn collect_files_with_annotations(review: &Review) -> Vec<(String, &crate::review::AnnotationTarget)> {
    if let Some(diff_files) = review.root_view.diff_files() {
        // Diff mode: use DiffFileView for display paths, enumerate for index
        diff_files
            .iter()
            .enumerate()
            .filter_map(|(index, df)| {
                let key = FileKey::diff_file(index);
                review.files.get(&key).and_then(|target| {
                    if target.annotations.is_empty() {
                        None
                    } else {
                        Some((df.path.display().to_string(), target))
                    }
                })
            })
            .collect()
    } else {
        // File mode: extract display string from FileKey
        review
            .files
            .iter()
            .filter(|(_, target)| !target.annotations.is_empty())
            .filter_map(|(key, target)| match key {
                FileKey::Path(p) => Some((p.display().to_string(), target)),
                FileKey::Ephemeral { label } => Some((label.clone(), target)),
                FileKey::DiffFile { .. } => None, // Should not happen in file mode
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{CliSource, ContentSource};
    use crate::state::{ContentMetadata, ContentModel, ExitMode, ExitModeSource, Line, LineRange, UserConfig};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_line(number: u32, content: &str) -> Line {
        Line {
            content: content.to_string(),
            html: None,
            origin: crate::state::LineOrigin::Source {
                path: "test.rs".to_string(),
                line: number,
            },
            semantics: crate::state::LineSemantics::Plain,
        }
    }

    fn make_review(label: &str, lines: Vec<Line>, annotations: HashMap<LineRange, Annotation>) -> Review {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from(label),
        });
        let content = ContentModel {
            label: label.to_string(),
            lines,
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let config = UserConfig::empty();
        let mut review = Review::cli(content, config, "main".to_string());
        // Insert annotations into the first file
        if let Some(file) = review.files.values_mut().next() {
            file.annotations = annotations;
        }
        review
    }

    #[test]
    fn empty_annotations_returns_empty_string() {
        let review = make_review("test.rs", vec![], HashMap::new());
        assert_eq!(format_output(&review, OutputMode::Cli).text, "");
    }

    #[test]
    fn single_line_annotation() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![ContentNode::Text {
                    text: "Fix this".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=10)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        assert!(output.contains("test.rs:5\n"));
        assert!(output.contains("> "));
        assert!(output.contains("line 5"));
        assert!(output.contains("└──> Fix this"));
    }

    #[test]
    fn multi_line_annotation() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(10, 15),
            Annotation {
                start_line: 10,
                end_line: 15,
                content: vec![ContentNode::Text {
                    text: "Review these lines".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=20)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        assert!(output.contains("test.rs:10-15\n"));
        // Check context line
        assert!(output.contains("line 9"));
        // Check selected lines have > prefix
        assert!(output.contains("> "));
        assert!(output.contains("line 10"));
        assert!(output.contains("line 15"));
    }

    #[test]
    fn multiple_annotations_sorted_by_line() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(20, 20),
            Annotation {
                start_line: 20,
                end_line: 20,
                content: vec![ContentNode::Text {
                    text: "Second".to_string(),
                }],
            },
        );
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![ContentNode::Text {
                    text: "First".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=25)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        // First annotation should come before second
        let first_pos = output.find("First").unwrap();
        let second_pos = output.find("Second").unwrap();
        assert!(first_pos < second_pos);

        // Should have separator between annotations
        assert!(output.contains("---"));
    }

    #[test]
    fn context_line_excluded_when_empty() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(3, 3),
            Annotation {
                start_line: 3,
                end_line: 3,
                content: vec![ContentNode::Text {
                    text: "Note".to_string(),
                }],
            },
        );

        let lines = vec![
            make_line(1, "first"),
            make_line(2, "   "), // whitespace only
            make_line(3, "third"),
        ];

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        // Line 2 is whitespace, shouldn't appear as context
        assert!(!output.contains("   \n"));
    }

    #[test]
    fn multiline_content_properly_indented() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![ContentNode::Text {
                    text: "Line one\nLine two\nLine three".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=10)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        // First line has arrow
        assert!(output.contains("└──> Line one"));
        // Continuation lines should be indented
        assert!(output.contains("Line two"));
        assert!(output.contains("Line three"));
    }

    #[test]
    fn session_block_with_exit_mode() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let config = UserConfig::with_data(
            vec![],
            vec![ExitMode {
                id: "apply".to_string(),
                name: "Apply".to_string(),
                color: "#22c55e".to_string(),
                instruction: "Apply the suggested changes".to_string(),
                order: 0,
                source: ExitModeSource::Persisted,
            }],
        );
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, config, "main".to_string());
        review.selected_exit_mode_id = Some("apply".to_string());

        let output = format_output(&review, OutputMode::Cli).text;

        assert!(output.contains("SESSION:"));
        assert!(output.contains("Apply (Apply the suggested changes)"));
    }

    #[test]
    fn session_block_with_annotations() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let config = UserConfig::with_data(
            vec![],
            vec![ExitMode {
                id: "reject".to_string(),
                name: "Reject".to_string(),
                color: "#ef4444".to_string(),
                instruction: "Do not apply".to_string(),
                order: 0,
                source: ExitModeSource::Persisted,
            }],
        );

        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![ContentNode::Text {
                    text: "Note".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=10)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let content = ContentModel {
            label: "test.rs".to_string(),
            lines,
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, config, "main".to_string());
        review.selected_exit_mode_id = Some("reject".to_string());
        if let Some(file) = review.files.values_mut().next() {
            file.annotations = annotations;
        }

        let output = format_output(&review, OutputMode::Cli).text;

        // SESSION block comes first
        let session_pos = output.find("SESSION:").unwrap();
        let annotation_pos = output.find("test.rs:5").unwrap();
        assert!(session_pos < annotation_pos);

        // Separator between SESSION and annotations
        assert!(output.contains("---"));
    }

    #[test]
    fn session_comment_in_output() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, UserConfig::empty(), "main".to_string());
        review.session_comment = Some(vec![ContentNode::Text {
            text: "This is a session comment".to_string(),
        }]);

        let output = format_output(&review, OutputMode::Cli).text;

        assert!(output.contains("SESSION:"));
        assert!(output.contains("  This is a session comment"));
    }

    #[test]
    fn session_comment_with_exit_mode() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let config = UserConfig::with_data(
            vec![],
            vec![ExitMode {
                id: "apply".to_string(),
                name: "Apply".to_string(),
                color: "#22c55e".to_string(),
                instruction: "Apply changes".to_string(),
                order: 0,
                source: ExitModeSource::Persisted,
            }],
        );
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, config, "main".to_string());
        review.session_comment = Some(vec![ContentNode::Text {
            text: "Overall looks good!".to_string(),
        }]);
        review.selected_exit_mode_id = Some("apply".to_string());

        let output = format_output(&review, OutputMode::Cli).text;

        // Session comment comes before exit mode
        let comment_pos = output.find("Overall looks good!").unwrap();
        let exit_pos = output.find("Apply (Apply changes)").unwrap();
        assert!(comment_pos < exit_pos);
    }

    #[test]
    fn empty_session_comment_not_rendered() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, UserConfig::empty(), "main".to_string());
        review.session_comment = Some(vec![]);

        let output = format_output(&review, OutputMode::Cli).text;

        // Empty session comment should result in no output
        assert!(output.is_empty());
    }

    #[test]
    fn legend_block_with_tags() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![
                    ContentNode::Tag {
                        id: "sec001".to_string(),
                        name: "SECURITY".to_string(),
                        instruction: "Review for vulnerabilities".to_string(),
                    },
                    ContentNode::Text {
                        text: " Use constant-time comparison".to_string(),
                    },
                ],
            },
        );

        let lines: Vec<Line> = (1..=10)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        // LEGEND block should appear at the top
        assert!(output.starts_with("LEGEND:\n"));
        assert!(output.contains("[# SECURITY] Review for vulnerabilities"));

        // Tag should render in annotation content
        assert!(output.contains("[# SECURITY] Use constant-time comparison"));
    }

    #[test]
    fn legend_alphabetically_sorted() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![
                    ContentNode::Tag {
                        id: "sec001".to_string(),
                        name: "SECURITY".to_string(),
                        instruction: "Security check".to_string(),
                    },
                    ContentNode::Tag {
                        id: "bug001".to_string(),
                        name: "BUG".to_string(),
                        instruction: "Bug fix".to_string(),
                    },
                ],
            },
        );

        let lines: Vec<Line> = (1..=10)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        // BUG should come before SECURITY (alphabetical)
        let bug_pos = output.find("[# BUG]").unwrap();
        let sec_pos = output.find("[# SECURITY]").unwrap();
        assert!(bug_pos < sec_pos);
    }

    #[test]
    fn tag_deduplication_in_legend() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![ContentNode::Tag {
                    id: "sec001".to_string(),
                    name: "SECURITY".to_string(),
                    instruction: "Security check".to_string(),
                }],
            },
        );
        annotations.insert(
            LineRange::new(10, 10),
            Annotation {
                start_line: 10,
                end_line: 10,
                content: vec![ContentNode::Tag {
                    id: "sec001".to_string(),
                    name: "SECURITY".to_string(),
                    instruction: "Security check".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=15)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let state = make_review("test.rs", lines, annotations);
        let output = format_output(&state, OutputMode::Cli).text;

        // SECURITY should only appear once in LEGEND
        let legend_end = output.find("\n\n").unwrap();
        let legend = &output[..legend_end];
        assert_eq!(legend.matches("[# SECURITY]").count(), 1);
    }

    #[test]
    fn portal_annotation_includes_source_lines() {
        // Create lines: main doc lines 1-5, then portal lines (from "portal.rs" lines 100-102),
        // then main doc lines 6-10. Portal lines are interleaved at index 5-7.
        let mut lines: Vec<Line> = Vec::new();

        // Main doc lines 1-5
        for n in 1..=5 {
            lines.push(Line {
                content: format!("main line {}", n),
                html: None,
                origin: crate::state::LineOrigin::Source {
                    path: "test.rs".to_string(),
                    line: n,
                },
                semantics: crate::state::LineSemantics::Plain,
            });
        }

        // Portal lines from "portal.rs" lines 100-102 (inserted at indices 5-7)
        for n in 100..=102 {
            lines.push(Line {
                content: format!("portal code line {}", n),
                html: None,
                origin: crate::state::LineOrigin::Source {
                    path: "/path/to/portal.rs".to_string(),
                    line: n,
                },
                semantics: crate::state::LineSemantics::Plain,
            });
        }

        // Main doc lines 6-10
        for n in 6..=10 {
            lines.push(Line {
                content: format!("main line {}", n),
                html: None,
                origin: crate::state::LineOrigin::Source {
                    path: "test.rs".to_string(),
                    line: n,
                },
                semantics: crate::state::LineSemantics::Plain,
            });
        }

        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines,
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let config = UserConfig::empty();
        let mut review = Review::cli(content, config, "main".to_string());

        // Register the portal file as an annotation target
        let portal_key = FileKey::path("/path/to/portal.rs");
        review.files.insert(portal_key.clone(), crate::review::AnnotationTarget::new());

        // Add annotation on portal line 101 (which is at array index 6, not 100)
        let portal_target = review.files.get_mut(&portal_key).unwrap();
        portal_target.annotations.insert(
            LineRange::new(101, 101),
            Annotation {
                start_line: 101,
                end_line: 101,
                content: vec![ContentNode::Text {
                    text: "Check this portal line".to_string(),
                }],
            },
        );

        let output = format_output(&review, OutputMode::Cli).text;

        // The output should contain the portal file path and line number
        assert!(output.contains("/path/to/portal.rs:101"), "Should have portal file header");
        // The output should contain the actual portal line content (found via find_line)
        assert!(output.contains("portal code line 101"), "Should have portal line content");
        // The annotation should be present
        assert!(output.contains("Check this portal line"), "Should have annotation text");
    }

    // ========== export_content tests ==========

    fn make_portal_line(content: &str, semantics: PortalSemantics) -> Line {
        Line {
            content: content.to_string(),
            html: None,
            origin: crate::state::LineOrigin::Source {
                path: "portal.rs".to_string(),
                line: 1,
            },
            semantics: LineSemantics::Portal(semantics),
        }
    }

    #[test]
    fn export_content_without_portals() {
        let content = ContentModel {
            label: "test.md".to_string(),
            lines: vec![
                make_line(1, "# Title"),
                make_line(2, "Some text"),
                make_line(3, "More text"),
            ],
            source: ContentSource::Cli(CliSource::File {
                path: PathBuf::from("test.md"),
            }),
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };

        let output = export_content(&content);
        assert_eq!(output, "# Title\nSome text\nMore text");
    }

    #[test]
    fn export_content_with_single_portal() {
        // Simulate markdown with a portal link on line 2
        // Line 1: "# Title"
        // Line 2: "Check [code](src/lib.rs#L10-L12)"
        // Then portal lines (header, content, footer) are interleaved
        // Line 3: "More text"

        let mut lines = vec![
            make_line(1, "# Title"),
            make_line(2, "Check [code](src/lib.rs#L10-L12)"),
        ];

        // Portal lines (interleaved after line 2)
        lines.push(make_portal_line(
            "src/lib.rs#L10-L12",
            PortalSemantics::Header {
                label: "code".to_string(),
                path: "src/lib.rs".to_string(),
                range: "L10-L12".to_string(),
            },
        ));
        lines.push(make_portal_line("fn hello() {", PortalSemantics::Content));
        lines.push(make_portal_line("    println!(\"hi\");", PortalSemantics::Content));
        lines.push(make_portal_line("}", PortalSemantics::Content));
        lines.push(make_portal_line("", PortalSemantics::Footer));

        lines.push(make_line(3, "More text"));

        let portal = crate::portal::LoadedPortal {
            source_path: PathBuf::from("src/lib.rs"),
            label: "code".to_string(),
            start_line: 10,
            end_line: 12,
            insert_at: 2, // Insert after line 2 (the portal link line)
            lines: vec![
                make_portal_line(
                    "src/lib.rs#L10-L12",
                    PortalSemantics::Header {
                        label: "code".to_string(),
                        path: "src/lib.rs".to_string(),
                        range: "L10-L12".to_string(),
                    },
                ),
                make_portal_line("fn hello() {", PortalSemantics::Content),
                make_portal_line("    println!(\"hi\");", PortalSemantics::Content),
                make_portal_line("}", PortalSemantics::Content),
                make_portal_line("", PortalSemantics::Footer),
            ],
        };

        let content = ContentModel {
            label: "test.md".to_string(),
            lines,
            source: ContentSource::Cli(CliSource::File {
                path: PathBuf::from("test.md"),
            }),
            metadata: ContentMetadata::Plain,
            portals: vec![portal],
        };

        let output = export_content(&content);

        // Should contain original markdown lines
        assert!(output.contains("# Title"), "Should have title");
        assert!(output.contains("Check [code](src/lib.rs#L10-L12)"), "Should have portal link");
        assert!(output.contains("More text"), "Should have text after portal");

        // Should contain portal comment and code fence
        assert!(
            output.contains("<!-- portal: src/lib.rs#L10-L12 -->"),
            "Should have portal comment"
        );
        assert!(output.contains("```rust"), "Should have rust code fence");
        assert!(output.contains("fn hello() {"), "Should have portal code content");
        assert!(output.contains("```\n"), "Should close code fence");
    }

    #[test]
    fn export_content_skips_empty_portal() {
        let mut lines = vec![
            make_line(1, "# Title"),
            make_line(2, "Check [code](empty.rs#L1-L1)"),
        ];

        // Portal with only header/footer, no content lines
        lines.push(make_portal_line(
            "empty.rs#L1-L1",
            PortalSemantics::Header {
                label: "code".to_string(),
                path: "empty.rs".to_string(),
                range: "L1-L1".to_string(),
            },
        ));
        lines.push(make_portal_line("", PortalSemantics::Footer));

        let portal = crate::portal::LoadedPortal {
            source_path: PathBuf::from("empty.rs"),
            label: "code".to_string(),
            start_line: 1,
            end_line: 1,
            insert_at: 2,
            lines: vec![
                make_portal_line(
                    "empty.rs#L1-L1",
                    PortalSemantics::Header {
                        label: "code".to_string(),
                        path: "empty.rs".to_string(),
                        range: "L1-L1".to_string(),
                    },
                ),
                // No content lines - empty portal
                make_portal_line("", PortalSemantics::Footer),
            ],
        };

        let content = ContentModel {
            label: "test.md".to_string(),
            lines,
            source: ContentSource::Cli(CliSource::File {
                path: PathBuf::from("test.md"),
            }),
            metadata: ContentMetadata::Plain,
            portals: vec![portal],
        };

        let output = export_content(&content);

        // Should NOT contain portal code block for empty portal
        assert!(!output.contains("<!-- portal:"), "Should not have portal comment for empty portal");
        assert!(!output.contains("```"), "Should not have code fence for empty portal");
    }

    #[test]
    fn export_content_with_multiple_portals() {
        let mut lines = vec![
            make_line(1, "# Title"),
            make_line(2, "[first](a.rs#L1-L2)"),
        ];

        // First portal lines
        lines.push(make_portal_line("a.rs#L1-L2", PortalSemantics::Header {
            label: "first".to_string(),
            path: "a.rs".to_string(),
            range: "L1-L2".to_string(),
        }));
        lines.push(make_portal_line("line1", PortalSemantics::Content));
        lines.push(make_portal_line("line2", PortalSemantics::Content));
        lines.push(make_portal_line("", PortalSemantics::Footer));

        lines.push(make_line(3, "[second](b.go#L5-L6)"));

        // Second portal lines
        lines.push(make_portal_line("b.go#L5-L6", PortalSemantics::Header {
            label: "second".to_string(),
            path: "b.go".to_string(),
            range: "L5-L6".to_string(),
        }));
        lines.push(make_portal_line("func main() {", PortalSemantics::Content));
        lines.push(make_portal_line("}", PortalSemantics::Content));
        lines.push(make_portal_line("", PortalSemantics::Footer));

        let portal1 = crate::portal::LoadedPortal {
            source_path: PathBuf::from("a.rs"),
            label: "first".to_string(),
            start_line: 1,
            end_line: 2,
            insert_at: 2,
            lines: vec![
                make_portal_line("a.rs#L1-L2", PortalSemantics::Header {
                    label: "first".to_string(),
                    path: "a.rs".to_string(),
                    range: "L1-L2".to_string(),
                }),
                make_portal_line("line1", PortalSemantics::Content),
                make_portal_line("line2", PortalSemantics::Content),
                make_portal_line("", PortalSemantics::Footer),
            ],
        };

        let portal2 = crate::portal::LoadedPortal {
            source_path: PathBuf::from("b.go"),
            label: "second".to_string(),
            start_line: 5,
            end_line: 6,
            insert_at: 3,
            lines: vec![
                make_portal_line("b.go#L5-L6", PortalSemantics::Header {
                    label: "second".to_string(),
                    path: "b.go".to_string(),
                    range: "L5-L6".to_string(),
                }),
                make_portal_line("func main() {", PortalSemantics::Content),
                make_portal_line("}", PortalSemantics::Content),
                make_portal_line("", PortalSemantics::Footer),
            ],
        };

        let content = ContentModel {
            label: "test.md".to_string(),
            lines,
            source: ContentSource::Cli(CliSource::File {
                path: PathBuf::from("test.md"),
            }),
            metadata: ContentMetadata::Plain,
            portals: vec![portal1, portal2],
        };

        let output = export_content(&content);

        // Both portals should be present with correct language hints
        assert!(output.contains("```rust"), "Should have rust code fence");
        assert!(output.contains("```go"), "Should have go code fence");
        assert!(output.contains("<!-- portal: a.rs#L1-L2 -->"), "Should have first portal comment");
        assert!(output.contains("<!-- portal: b.go#L5-L6 -->"), "Should have second portal comment");
    }

    #[test]
    fn export_content_language_detection() {
        // Test various file extensions produce correct fence languages
        assert_eq!(lang::extension_to_fence_language("rs"), "rust");
        assert_eq!(lang::extension_to_fence_language("go"), "go");
        assert_eq!(lang::extension_to_fence_language("ts"), "typescript");
        assert_eq!(lang::extension_to_fence_language("tsx"), "tsx"); // TSX is its own language
        assert_eq!(lang::extension_to_fence_language("py"), "python");
        assert_eq!(lang::extension_to_fence_language("js"), "javascript");
        assert_eq!(lang::extension_to_fence_language("unknown"), "");
    }

    // ========== Diff annotation output tests ==========

    /// Regression test: diff annotations must include line numbers in output.
    #[test]
    fn diff_annotation_includes_line_numbers() {
        use crate::input::{DiffSource, McpSource};

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

        // Create diff content model
        let source = ContentSource::Mcp(McpSource::Diff {
            label: Some("test.diff".to_string()),
            source: DiffSource::Raw,
        });
        let content = ContentModel::from_diff(SIMPLE_DIFF, source).unwrap();
        let config = UserConfig::empty();
        let mut review = Review::cli(content, config, "main".to_string());

        // The added line "+    more_code();" is at new_line=3 in file.rs
        let diff_file_key = FileKey::diff_file(0);
        let target = review.files.get_mut(&diff_file_key).unwrap();

        // Add annotation at line 3
        target.upsert_annotation(
            3,
            3,
            vec![ContentNode::Text {
                text: "Review this change".to_string(),
            }],
        );

        let output = format_output(&review, OutputMode::Cli).text;

        // The output should include the file name with line number info
        assert!(
            output.contains("file.rs (new:3):"),
            "Diff annotation header should include new line number. Got:\n{}",
            output
        );

        // The output should include the line content
        assert!(
            output.contains("more_code"),
            "Diff annotation should include the line content. Got:\n{}",
            output
        );

        // The annotation text should be present
        assert!(
            output.contains("Review this change"),
            "Diff annotation should include annotation text. Got:\n{}",
            output
        );
    }

    /// Test that deleted lines in diff annotations show old line numbers.
    #[test]
    fn diff_annotation_deleted_line_shows_old_number() {
        use crate::input::{DiffSource, McpSource};

        const SIMPLE_DIFF: &str = r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,3 @@
 fn main() {
-    old_code();
+    new_code();
 }
"#;

        let source = ContentSource::Mcp(McpSource::Diff {
            label: Some("test.diff".to_string()),
            source: DiffSource::Raw,
        });
        let content = ContentModel::from_diff(SIMPLE_DIFF, source).unwrap();
        let config = UserConfig::empty();
        let mut review = Review::cli(content, config, "main".to_string());

        // The deleted line "-    old_code();" is at old_line=2
        let diff_file_key = FileKey::diff_file(0);
        let target = review.files.get_mut(&diff_file_key).unwrap();

        // Add annotation at line 2
        target.upsert_annotation(
            2,
            2,
            vec![ContentNode::Text {
                text: "This was removed".to_string(),
            }],
        );

        let output = format_output(&review, OutputMode::Cli).text;

        // For a deleted line, should show old line number
        assert!(
            output.contains("file.rs (old:2):"),
            "Diff annotation header should include old line number for deleted line. Got:\n{}",
            output
        );
    }

    /// Test that context lines in diff annotations show both old and new line numbers.
    #[test]
    fn diff_annotation_context_line_shows_both_numbers() {
        use crate::input::{DiffSource, McpSource};

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

        let source = ContentSource::Mcp(McpSource::Diff {
            label: Some("test.diff".to_string()),
            source: DiffSource::Raw,
        });
        let content = ContentModel::from_diff(SIMPLE_DIFF, source).unwrap();
        let config = UserConfig::empty();
        let mut review = Review::cli(content, config, "main".to_string());

        // The context line " fn main() {" is at old_line=1, new_line=1
        let diff_file_key = FileKey::diff_file(0);
        let target = review.files.get_mut(&diff_file_key).unwrap();

        // Add annotation at line 1
        target.upsert_annotation(
            1,
            1,
            vec![ContentNode::Text {
                text: "Check function signature".to_string(),
            }],
        );

        let output = format_output(&review, OutputMode::Cli).text;

        // For a context line, should show both old and new line numbers
        assert!(
            output.contains("file.rs (old:1 new:1):"),
            "Diff annotation header should include both line numbers for context line. Got:\n{}",
            output
        );
    }

    // ========== saved_to output tests ==========

    #[test]
    fn saved_to_only_produces_output() {
        let mut review = make_review("test.rs", vec![], HashMap::new());
        review.saved_to = Some(PathBuf::from("/tmp/saved-file.md"));

        let output = format_output(&review, OutputMode::Cli).text;

        assert_eq!(output, "Saved to /tmp/saved-file.md\n");
    }

    #[test]
    fn saved_to_with_annotations() {
        let mut annotations = HashMap::new();
        annotations.insert(
            LineRange::new(5, 5),
            Annotation {
                start_line: 5,
                end_line: 5,
                content: vec![ContentNode::Text {
                    text: "Fix this".to_string(),
                }],
            },
        );

        let lines: Vec<Line> = (1..=10)
            .map(|n| make_line(n, &format!("line {}", n)))
            .collect();

        let mut review = make_review("test.rs", lines, annotations);
        review.saved_to = Some(PathBuf::from("/tmp/output.md"));

        let output = format_output(&review, OutputMode::Cli).text;

        // Should have annotation content
        assert!(output.contains("test.rs:5"), "Should have annotation header");
        assert!(output.contains("Fix this"), "Should have annotation text");
        // Should end with saved_to line
        assert!(
            output.ends_with("Saved to /tmp/output.md\n"),
            "Should end with saved_to. Got:\n{}",
            output
        );
    }

    #[test]
    fn saved_to_with_session_comment() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, UserConfig::empty(), "main".to_string());
        review.session_comment = Some(vec![ContentNode::Text {
            text: "Overall looks good".to_string(),
        }]);
        review.saved_to = Some(PathBuf::from("/tmp/review.md"));

        let output = format_output(&review, OutputMode::Cli).text;

        assert!(output.contains("SESSION:"), "Should have session block");
        assert!(output.contains("Overall looks good"), "Should have session comment");
        assert!(
            output.ends_with("Saved to /tmp/review.md\n"),
            "Should end with saved_to. Got:\n{}",
            output
        );
    }

    #[test]
    fn no_saved_to_no_extra_line() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, UserConfig::empty(), "main".to_string());
        review.session_comment = Some(vec![ContentNode::Text {
            text: "Comment only".to_string(),
        }]);
        // No saved_to

        let output = format_output(&review, OutputMode::Cli).text;

        assert!(!output.contains("Saved to"), "Should not have saved_to line");
    }

    #[test]
    fn trim_trailing_separators_removes_dashes() {
        let input = "# Heading\n\nContent\n\n---";
        let result = super::trim_trailing_separators(input);
        assert_eq!(result, "# Heading\n\nContent");
    }

    #[test]
    fn trim_trailing_separators_removes_underscores() {
        let input = "# Heading\n\n___";
        let result = super::trim_trailing_separators(input);
        assert_eq!(result, "# Heading");
    }

    #[test]
    fn trim_trailing_separators_removes_asterisks() {
        let input = "Content\n***";
        let result = super::trim_trailing_separators(input);
        assert_eq!(result, "Content");
    }

    #[test]
    fn trim_trailing_separators_removes_multiple() {
        let input = "Content\n\n---\n\n***";
        let result = super::trim_trailing_separators(input);
        assert_eq!(result, "Content");
    }

    #[test]
    fn trim_trailing_separators_preserves_content() {
        let input = "# Heading\n\nSome content here";
        let result = super::trim_trailing_separators(input);
        assert_eq!(result, "# Heading\n\nSome content here");
    }

    #[test]
    fn trim_trailing_separators_handles_spaced_separator() {
        let input = "Content\n- - -";
        let result = super::trim_trailing_separators(input);
        assert_eq!(result, "Content");
    }

    // ====== Command Exit Mode Tests ======

    #[test]
    fn command_exit_mode_includes_path_and_content() {
        use tempfile::TempDir;
        use std::fs;

        // Create a temp command file
        let temp = TempDir::new().unwrap();
        let cmd_path = temp.path().join("test-cmd.md");
        fs::write(&cmd_path, r#"---
description: "Test command"
---
## Instructions

Do something useful.
"#).unwrap();

        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let config = UserConfig::with_data(
            vec![],
            vec![ExitMode {
                id: "cmd-test".to_string(),
                name: "/test-cmd".to_string(),
                color: "#8b5cf6".to_string(),
                instruction: "Test command".to_string(),
                order: 0,
                source: crate::state::ExitModeSource::Command { path: cmd_path.clone() },
            }],
        );
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, config, "main".to_string());
        review.selected_exit_mode_id = Some("cmd-test".to_string());

        let output = format_output(&review, OutputMode::Cli).text;

        // Should include the exit mode name and instruction
        assert!(output.contains("/test-cmd (Test command)"), "Should have exit mode header");
        // Should include the command path
        assert!(output.contains("Command:"), "Should have Command: line");
        assert!(output.contains("test-cmd.md"), "Should include command file name");
        // Should include the command content
        assert!(output.contains("## Instructions"), "Should include command content heading");
        assert!(output.contains("Do something useful"), "Should include command content body");
        // Should have separator lines
        assert!(output.contains(SEPARATOR), "Should have content separators");
    }

    #[test]
    fn regular_exit_mode_does_not_include_command_content() {
        let source = ContentSource::Cli(CliSource::File {
            path: PathBuf::from("test.rs"),
        });
        let config = UserConfig::with_data(
            vec![],
            vec![ExitMode {
                id: "apply".to_string(),
                name: "Apply".to_string(),
                color: "#22c55e".to_string(),
                instruction: "Apply changes".to_string(),
                order: 0,
                source: ExitModeSource::Persisted,
            }],
        );
        let content = ContentModel {
            label: "test.rs".to_string(),
            lines: vec![],
            source,
            metadata: ContentMetadata::Plain,
            portals: Vec::new(),
        };
        let mut review = Review::cli(content, config, "main".to_string());
        review.selected_exit_mode_id = Some("apply".to_string());

        let output = format_output(&review, OutputMode::Cli).text;

        // Should have exit mode but NOT command-specific content
        assert!(output.contains("Apply (Apply changes)"), "Should have exit mode");
        assert!(!output.contains("Command:"), "Should NOT have Command: line for regular exit mode");
        assert!(!output.contains(SEPARATOR), "Should NOT have content separators");
    }
}
