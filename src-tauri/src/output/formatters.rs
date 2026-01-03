//! Formatting helpers for specific output sections.
//!
//! Each formatter takes an OutputBuilder and adds its content using
//! the builder's declarative API.

use std::collections::BTreeMap;

use crate::mcp::tools::SessionImage;
use crate::state::{
    Annotation, Bookmark, ContentMetadata, ContentModel, LineOrigin,
};

use super::builder::{BuilderMode, OutputBuilder};
use super::render::render_content;
use super::OutputMode;

/// Format the LEGEND section with tag definitions.
pub fn format_legend(out: &mut OutputBuilder, tags: &BTreeMap<String, String>) {
    for (name, instruction) in tags {
        out.line(&format!("[# {}] {}", name, instruction));
    }
}

/// Format a single bookmark entry.
pub fn format_bookmark(out: &mut OutputBuilder, bookmark: &Bookmark, created_this_session: bool) {
    let short_id = &bookmark.id[..bookmark.id.len().min(3)];
    let display_label = bookmark.display_label();

    if created_this_session {
        // Condensed: created this session, agent already has context
        out.line(&format!(
            "[BOOKMARK {}] {} (this session)",
            short_id, display_label
        ));
    } else {
        // Full: pre-existing bookmark, emit full context
        out.line(&format!("[BOOKMARK {}] {}", short_id, display_label));
        out.indented(|b| {
            b.field("Source", &bookmark.snapshot.source_title());
            if let Some(ref project) = bookmark.project_path {
                b.field("Project", &project.display().to_string());
            }
            b.field("Created", &bookmark.created_at.format("%Y-%m-%d").to_string());
            b.separator();
            for line in bookmark.snapshot.content().lines() {
                b.line(line);
            }
            b.separator();
        });
        out.blank_line();
    }
}

/// Format a single annotation block with context lines and content.
pub fn format_annotation(
    out: &mut OutputBuilder,
    content_model: &ContentModel,
    ann: &Annotation,
    file_path: &str,
    images: &mut Vec<SessionImage>,
    figure_counter: &mut usize,
    mode: OutputMode,
) {
    let is_diff = matches!(content_model.metadata, ContentMetadata::Diff(_));

    // File header
    if is_diff {
        format_diff_header(out, content_model, ann, file_path);
    } else if ann.start_line == ann.end_line {
        out.raw_line(&format!("{}:{}", file_path, ann.start_line));
    } else {
        out.raw_line(&format!(
            "{}:{}-{}",
            file_path, ann.start_line, ann.end_line
        ));
    }

    // Context line (1 line before, if exists and non-empty)
    if ann.start_line > 1 {
        let context_line_num = ann.start_line - 1;
        if let Some(line) = content_model.find_line(file_path, context_line_num) {
            if !line.content.trim().is_empty() {
                if is_diff {
                    format_diff_context_line(out, content_model, file_path, context_line_num, &line.content);
                } else {
                    out.code_line(context_line_num, &line.content);
                }
            }
        }
    }

    // Selected lines
    for line_num in ann.start_line..=ann.end_line {
        if let Some(line) = content_model.find_line(file_path, line_num) {
            if is_diff {
                format_diff_selected_line(out, content_model, file_path, line_num, &line.content);
            } else {
                out.selected_code_line(line_num, &line.content);
            }
        }
    }

    // Annotation content with arrow
    let content_text = render_content(&ann.content, images, figure_counter, mode);
    let mut lines = content_text.lines();

    if let Some(first) = lines.next() {
        out.arrow(first);
        for continuation in lines {
            out.arrow_continuation(continuation);
        }
    }
}

/// Format diff header with file info from annotation range.
fn format_diff_header(
    out: &mut OutputBuilder,
    content: &ContentModel,
    ann: &Annotation,
    file_path: &str,
) {
    // Collect old/new line ranges from the annotated lines
    let mut old_lines: Vec<u32> = Vec::new();
    let mut new_lines: Vec<u32> = Vec::new();

    for line_num in ann.start_line..=ann.end_line {
        if let Some(line) = content.find_line(file_path, line_num) {
            if let LineOrigin::Diff { old_line, new_line, .. } = &line.origin {
                if let Some(old) = old_line {
                    old_lines.push(*old);
                }
                if let Some(new) = new_line {
                    new_lines.push(*new);
                }
            }
        }
    }

    // Format header with available line info
    let old_range = format_line_range(&old_lines);
    let new_range = format_line_range(&new_lines);

    let header = match (old_range.as_str(), new_range.as_str()) {
        ("", "") => format!("{}:", file_path),
        (old, "") => format!("{} (old:{}):", file_path, old),
        ("", new) => format!("{} (new:{}):", file_path, new),
        (old, new) => format!("{} (old:{} new:{}):", file_path, old, new),
    };
    out.raw_line(&header);
}

/// Format a line range like "10" or "10-15".
fn format_line_range(lines: &[u32]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let min = *lines.iter().min().unwrap();
    let max = *lines.iter().max().unwrap();
    if min == max {
        min.to_string()
    } else {
        format!("{}-{}", min, max)
    }
}

/// Format a diff context line (not selected).
fn format_diff_context_line(
    out: &mut OutputBuilder,
    content_model: &ContentModel,
    file_path: &str,
    line_num: u32,
    content: &str,
) {
    let (old, new) = extract_diff_line_nums(content_model, file_path, line_num);
    out.diff_line(old, new, content, false);
}

/// Format a diff selected line.
fn format_diff_selected_line(
    out: &mut OutputBuilder,
    content_model: &ContentModel,
    file_path: &str,
    line_num: u32,
    content: &str,
) {
    let (old, new) = extract_diff_line_nums(content_model, file_path, line_num);
    out.diff_line(old, new, content, true);
}

/// Extract old/new line numbers from a diff line.
fn extract_diff_line_nums(
    content_model: &ContentModel,
    file_path: &str,
    line_num: u32,
) -> (Option<u32>, Option<u32>) {
    content_model
        .find_line(file_path, line_num)
        .and_then(|line| {
            if let LineOrigin::Diff { old_line, new_line, .. } = &line.origin {
                Some((*old_line, *new_line))
            } else {
                None
            }
        })
        .unwrap_or((None, None))
}

/// Calculate the BuilderMode from annotations.
pub fn calculate_builder_mode(
    content: &ContentModel,
    max_line: u32,
) -> BuilderMode {
    let is_diff = matches!(content.metadata, ContentMetadata::Diff(_));
    let line_num_width = max_line.to_string().len();

    if is_diff {
        BuilderMode::Diff {
            left_width: line_num_width,
            right_width: line_num_width,
        }
    } else {
        BuilderMode::File { line_num_width }
    }
}
