//! Formatting helpers for specific output sections.
//!
//! Each formatter takes an OutputBuilder and adds its content using
//! the builder's declarative API.

use std::collections::BTreeMap;

use crate::anchor::{Anchor, Annotation, Endpoint};
use crate::mcp::tools::SessionImage;
use crate::source::Side;
use crate::state::{ContentModel, ContentView, DiffDocument, Row};

use super::builder::{BuilderMode, OutputBuilder};
use super::render::render_content;
use super::OutputMode;

/// Format the LEGEND section with tag definitions.
pub fn format_legend(out: &mut OutputBuilder, tags: &BTreeMap<String, String>) {
    for (name, instruction) in tags {
        out.line(&format!("[# {}] {}", name, instruction));
    }
}

/// Format a single annotation block with context lines and content.
/// `doc` is the anchor's diff document (diff mode only).
#[allow(clippy::too_many_arguments)]
pub fn format_annotation(
    out: &mut OutputBuilder,
    content_model: &ContentModel,
    ann: &Annotation,
    file_path: &str,
    doc: Option<&DiffDocument>,
    images: &mut Vec<SessionImage>,
    figure_counter: &mut usize,
    mode: OutputMode,
) {
    if let Some(doc) = doc {
        format_diff_block(out, doc, ann, file_path);
    } else {
        // File header
        if ann.start_line() == ann.end_line() {
            out.raw_line(&format!("{}:{}", file_path, ann.start_line()));
        } else {
            out.raw_line(&format!(
                "{}:{}-{}",
                file_path,
                ann.start_line(),
                ann.end_line()
            ));
        }

        // Context line (1 line before, if exists and non-empty)
        if ann.start_line() > 1 {
            let context_line_num = ann.start_line() - 1;
            if let Some(line) = content_model.find_line(file_path, context_line_num) {
                if !line.content.trim().is_empty() {
                    out.code_line(context_line_num, &line.content);
                }
            }
        }

        // Selected lines
        for line_num in ann.start_line()..=ann.end_line() {
            if let Some(line) = content_model.find_line(file_path, line_num) {
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

/// Format the header, context row, and selected rows for a diff annotation.
///
/// Both anchor endpoints resolve side-aware within the document's rows
/// (hunks flattened in order), and the contiguous row slice between them is
/// what renders — for a mixed-side range that slice covers a deletion and
/// its added replacement.
fn format_diff_block(
    out: &mut OutputBuilder,
    doc: &DiffDocument,
    ann: &Annotation,
    file_path: &str,
) {
    let Anchor::Diff { start, end, .. } = &ann.anchor else {
        // A side-less anchor can't resolve against a diff: header only.
        out.raw_line(&format!("{}:", file_path));
        return;
    };

    // (hunk index, row) in document order — the hunk index bounds context.
    let rows: Vec<(usize, &Row)> = doc
        .hunks
        .iter()
        .enumerate()
        .flat_map(|(hunk, h)| h.rows.iter().map(move |row| (hunk, row)))
        .collect();
    let find = |ep: &Endpoint| {
        rows.iter().position(|(_, row)| match ep.side {
            Side::Old => row.old_line == Some(ep.line),
            Side::New => row.new_line == Some(ep.line),
        })
    };

    let Some((first, last)) = find(start)
        .zip(find(end))
        .map(|(s, e)| (s.min(e), s.max(e)))
    else {
        // Anchor doesn't resolve against this diff: header only.
        out.raw_line(&format!("{}:", file_path));
        return;
    };

    format_diff_header(out, &rows[first..=last], (start, end), file_path);

    // Context: the previous row within the same hunk, if renderable — an
    // anchor at a hunk's first row gets none.
    if first > 0 && rows[first - 1].0 == rows[first].0 {
        let (_, row) = rows[first - 1];
        let content = prefixed(row);
        if !content.trim().is_empty() {
            out.diff_line(row.old_line, row.new_line, &content, false);
        }
    }

    // Selected rows
    for (_, row) in &rows[first..=last] {
        out.diff_line(row.old_line, row.new_line, &prefixed(row), true);
    }
}

/// A row's emit form: the `+`/`-`/` ` sign — derived from the line-number
/// pattern — re-prepended to the raw content. Byte-identical to what the
/// flattened wire used to carry. Shared with the patch-shaped content export.
pub(super) fn prefixed(row: &Row) -> String {
    let prefix = match (row.old_line, row.new_line) {
        (Some(_), Some(_)) => ' ',
        (Some(_), None) => '-',
        (None, Some(_)) => '+',
        (None, None) => unreachable!("a row always belongs to at least one side"),
    };
    format!("{prefix}{}", row.content)
}

/// Format diff header with file info from the resolved row slice.
fn format_diff_header(
    out: &mut OutputBuilder,
    rows: &[(usize, &Row)],
    (start, end): (&Endpoint, &Endpoint),
    file_path: &str,
) {
    // Mixed-side range: name the endpoints with their sides. This shape is
    // additive — no single-side or context annotation can produce it.
    if start.side != end.side {
        out.raw_line(&format!(
            "{} ({}:{} → {}:{}):",
            file_path,
            side_label(start.side),
            start.line,
            side_label(end.side),
            end.line
        ));
        return;
    }

    // Single-side: collect old/new line ranges from the rendered rows.
    let old_lines: Vec<u32> = rows.iter().filter_map(|(_, row)| row.old_line).collect();
    let new_lines: Vec<u32> = rows.iter().filter_map(|(_, row)| row.new_line).collect();

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

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Old => "old",
        Side::New => "new",
    }
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

/// Calculate the BuilderMode from annotations.
pub fn calculate_builder_mode(content: &ContentModel, max_line: u32) -> BuilderMode {
    let is_diff = matches!(content.view, ContentView::Diff { .. });
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
