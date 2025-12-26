use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::mcp::tools::SessionImage;
use crate::review::{FileKey, Review};
use crate::state::{Annotation, ContentMetadata, ContentModel, ContentNode};

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

    if !has_exit_mode && !has_annotations && !has_session_comment {
        return FormatResult {
            text: String::new(),
            images: Vec::new(),
        };
    }

    let mut output = String::new();
    let mut images = Vec::new();
    let mut figure_counter = 0usize;

    // LEGEND block (if any tags are used)
    let unique_tags = collect_unique_tags(review);
    if !unique_tags.is_empty() {
        output.push_str("LEGEND:\n");
        for (name, instruction) in &unique_tags {
            output.push_str(&format!("  [# {}] {}\n", name, instruction));
        }
        output.push('\n');
    }

    // SESSION block (if exit mode selected or session comment exists)
    if has_exit_mode || has_session_comment {
        output.push_str("SESSION:\n");

        // If there are portals, show "Reviewing X with embedded files: Y, Z"
        if !content.portals.is_empty() {
            let root_name = &content.label;
            let embedded_files: Vec<_> = content
                .portals
                .iter()
                .map(|p| p.source_path.display().to_string())
                .collect();
            output.push_str(&format!(
                "  Reviewing {} with embedded files: {}\n",
                root_name,
                embedded_files.join(", ")
            ));
        }

        // Session comment (no prefix, directly indented)
        if let Some(ref comment) = review.session_comment {
            if !comment.is_empty() {
                let comment_text = render_content(comment, &mut images, &mut figure_counter, mode);
                for line in comment_text.lines() {
                    output.push_str(&format!("  {}\n", line));
                }
            }
        }

        // Exit mode (original format: "Name (instruction)")
        if let Some(ref mode_id) = review.selected_exit_mode_id {
            if let Some(exit_mode) = review.config.exit_modes().iter().find(|m| &m.id == mode_id) {
                output.push_str(&format!("  {} ({})\n", exit_mode.name, exit_mode.instruction));
            }
        }

        if has_annotations {
            output.push_str("\n---\n\n");
        }
    }

    if !has_annotations {
        return FormatResult {
            text: output,
            images,
        };
    }

    // Build list of (display_path, annotations) for files with annotations
    // We collect display paths (PathBuf) for output formatting
    let files_with_annotations: Vec<(PathBuf, &_)> = if let Some(diff_files) = review.root_view.diff_files() {
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
                        Some((df.path.clone(), target))
                    }
                })
            })
            .collect()
    } else {
        // File mode: extract path from FileKey
        review
            .files
            .iter()
            .filter(|(_, target)| !target.annotations.is_empty())
            .filter_map(|(key, target)| {
                match key {
                    FileKey::Path(p) => Some((p.clone(), target)),
                    FileKey::DiffFile { .. } => None, // Should not happen in file mode
                }
            })
            .collect()
    };

    // Calculate max line number width across all annotations
    let max_line = files_with_annotations
        .iter()
        .flat_map(|(_, target)| target.annotations.values())
        .map(|a| a.end_line)
        .max()
        .unwrap_or(0);
    let line_num_width = max_line.to_string().len();

    let mut first_block = true;
    for (display_path, target) in &files_with_annotations {
        // Sort annotations within this file by start line
        let mut sorted_annotations: Vec<&Annotation> = target.annotations.values().collect();
        sorted_annotations.sort_by_key(|a| a.start_line);

        for ann in sorted_annotations {
            if !first_block {
                output.push_str("\n---\n\n");
            }
            first_block = false;
            format_annotation_block(
                &mut output,
                content,
                ann,
                display_path,
                line_num_width,
                &mut images,
                &mut figure_counter,
                mode,
            );
        }
    }

    FormatResult {
        text: output,
        images,
    }
}

/// Format a single annotation block with context and content.
fn format_annotation_block(
    out: &mut String,
    content: &ContentModel,
    ann: &Annotation,
    file_path: &std::path::PathBuf,
    line_num_width: usize,
    images: &mut Vec<SessionImage>,
    figure_counter: &mut usize,
    mode: OutputMode,
) {
    let is_diff = matches!(content.metadata, ContentMetadata::Diff(_));
    let file_label = file_path.display();

    // File header: "file.rs:10-15" or "file.rs:10"
    // For diffs, include old/new line numbers if available
    if is_diff {
        format_diff_header(out, content, ann, file_path);
    } else if ann.start_line == ann.end_line {
        out.push_str(&format!("{}:{}\n", file_label, ann.start_line));
    } else {
        out.push_str(&format!(
            "{}:{}-{}\n",
            file_label, ann.start_line, ann.end_line
        ));
    }

    // Context line (1 line before, if exists and non-empty)
    if ann.start_line > 1 {
        let context_line_num = ann.start_line - 1;
        if let Some(line) = content.lines.get((context_line_num - 1) as usize) {
            if !line.content.trim().is_empty() {
                // Format: "    N | content" (3 extra spaces for ">" prefix alignment)
                if is_diff {
                    format_diff_line(out, content, context_line_num, &line.content, false, line_num_width);
                } else {
                    out.push_str(&format!(
                        "{:>width$} | {}\n",
                        context_line_num,
                        line.content,
                        width = line_num_width + 3
                    ));
                }
            }
        }
    }

    // Selected lines with ">" prefix
    for line_num in ann.start_line..=ann.end_line {
        if let Some(line) = content.lines.get((line_num - 1) as usize) {
            if is_diff {
                format_diff_line(out, content, line_num, &line.content, true, line_num_width);
            } else {
                // Format: ">  N | content"
                out.push_str(&format!(
                    "> {:>width$} | {}\n",
                    line_num,
                    line.content,
                    width = line_num_width + 1
                ));
            }
        }
    }

    // Annotation content with arrow (aligned with the pipe "|")
    // For non-diff: "> {:>width$} | " - pipe at position 2 + (line_num_width+1) + 1
    // For diff: "{}{:>w$}:{:<w$} | " - pipe at position 2 + w + 1 + w + 1
    let arrow_indent = if is_diff {
        " ".repeat(2 * line_num_width + 4)
    } else {
        " ".repeat(line_num_width + 4)
    };
    let content_text = render_content(&ann.content, images, figure_counter, mode);

    for (i, content_line) in content_text.lines().enumerate() {
        if i == 0 {
            out.push_str(&format!("{}└──> {}\n", arrow_indent, content_line));
        } else {
            // Continuation lines: align with content after arrow
            let continuation_indent = format!("{}     ", arrow_indent); // 5 chars for "└──> "
            out.push_str(&format!("{}{}\n", continuation_indent, content_line));
        }
    }
}

/// Format diff header with file info from annotation range.
fn format_diff_header(
    out: &mut String,
    content: &ContentModel,
    ann: &Annotation,
    file_path: &std::path::PathBuf,
) {
    let diff_meta = match &content.metadata {
        ContentMetadata::Diff(d) => d,
        _ => return,
    };

    // Use the file path we already have (from Review.files key)
    let file_name = file_path.display();

    // Collect old/new line ranges from the annotated lines
    let mut old_lines: Vec<u32> = Vec::new();
    let mut new_lines: Vec<u32> = Vec::new();

    for line_num in ann.start_line..=ann.end_line {
        if let Some(info) = diff_meta.lines.get(&line_num) {
            if let Some(old) = info.old_line_num {
                old_lines.push(old);
            }
            if let Some(new) = info.new_line_num {
                new_lines.push(new);
            }
        }
    }

    // Format header with available line info
    let old_range = format_line_range(&old_lines);
    let new_range = format_line_range(&new_lines);

    match (old_range.as_str(), new_range.as_str()) {
        ("", "") => out.push_str(&format!("{}:\n", file_name)),
        (old, "") => out.push_str(&format!("{} (old:{}):\n", file_name, old)),
        ("", new) => out.push_str(&format!("{} (new:{}):\n", file_name, new)),
        (old, new) => out.push_str(&format!("{} (old:{} new:{}):\n", file_name, old, new)),
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

/// Format a single diff line with old:new line numbers.
fn format_diff_line(
    out: &mut String,
    content_model: &ContentModel,
    line_num: u32,
    content: &str,
    is_selected: bool,
    line_num_width: usize,
) {
    let diff_meta = match &content_model.metadata {
        ContentMetadata::Diff(d) => d,
        _ => return,
    };
    let info = diff_meta.lines.get(&line_num);

    let prefix = if is_selected { "> " } else { "  " };

    let (old_str, new_str) = match info {
        Some(i) => {
            let old = i
                .old_line_num
                .map(|n| n.to_string())
                .unwrap_or_default();
            let new = i
                .new_line_num
                .map(|n| n.to_string())
                .unwrap_or_default();
            (old, new)
        }
        None => (String::new(), String::new()),
    };

    // Format: "> old:new | content" or "  old:new | content"
    out.push_str(&format!(
        "{}{:>w$}:{:<w$} | {}\n",
        prefix,
        old_str,
        new_str,
        content,
        w = line_num_width
    ));
}

/// Render content nodes to plain text, collecting images with figure numbers.
fn render_content(
    nodes: &[ContentNode],
    images: &mut Vec<SessionImage>,
    figure_counter: &mut usize,
    mode: OutputMode,
) -> String {
    nodes
        .iter()
        .map(|node| match node {
            ContentNode::Text { text } => text.clone(),
            ContentNode::Tag { name, .. } => format!("[# {}]", name),
            ContentNode::Media { image, mime_type } => {
                *figure_counter += 1;
                let figure_num = *figure_counter;

                // Extract base64 data from data URL (strip "data:image/png;base64," prefix)
                let data = if let Some(idx) = image.find(",") {
                    image[idx + 1..].to_string()
                } else {
                    image.clone()
                };

                images.push(SessionImage {
                    figure: figure_num,
                    data,
                    mime_type: mime_type.clone(),
                });

                format!("[Figure {}]", figure_num)
            }
            ContentNode::Excalidraw { elements, image } => {
                *figure_counter += 1;
                let figure_num = *figure_counter;

                // If PNG is available, include it for MCP
                if let Some(ref png_data) = image {
                    let data = if let Some(idx) = png_data.find(",") {
                        png_data[idx + 1..].to_string()
                    } else {
                        png_data.clone()
                    };
                    images.push(SessionImage {
                        figure: figure_num,
                        data,
                        mime_type: "image/png".to_string(),
                    });
                }

                match mode {
                    // CLI: include JSON so diagram data is preserved in stdout
                    OutputMode::Cli => format!("[EXCALIDRAW Figure {}]\n{}", figure_num, elements),
                    // MCP/Clipboard: just figure reference, no JSON blob
                    OutputMode::Mcp | OutputMode::Clipboard => {
                        format!("[EXCALIDRAW Figure {}]", figure_num)
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{CliSource, ContentSource};
    use crate::state::{ContentModel, ExitMode, ExitModeOrigin, Line, LineRange, UserConfig};
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
                origin: ExitModeOrigin::Persisted,
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
                origin: ExitModeOrigin::Persisted,
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
                origin: ExitModeOrigin::Persisted,
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
}
