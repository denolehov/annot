use std::collections::BTreeMap;

use crate::state::{Annotation, AppState, ContentNode};

/// Collect unique tags from all content nodes (session comment + annotations).
/// Returns a BTreeMap for alphabetical ordering by tag name.
fn collect_unique_tags(state: &AppState) -> BTreeMap<String, String> {
    let mut tags: BTreeMap<String, String> = BTreeMap::new();

    // Collect from session comment
    if let Some(ref comment) = state.session_comment {
        for node in comment {
            if let ContentNode::Tag {
                name, instruction, ..
            } = node
            {
                tags.insert(name.clone(), instruction.clone());
            }
        }
    }

    // Collect from all annotations
    for annotation in state.annotations.values() {
        for node in &annotation.content {
            if let ContentNode::Tag {
                name, instruction, ..
            } = node
            {
                tags.insert(name.clone(), instruction.clone());
            }
        }
    }

    tags
}

/// Format all annotations as structured output for LLM consumption.
pub fn format_output(state: &AppState) -> String {
    let has_exit_mode = state.selected_exit_mode_id.is_some();
    let has_annotations = !state.annotations.is_empty();
    let has_session_comment = state
        .session_comment
        .as_ref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);

    if !has_exit_mode && !has_annotations && !has_session_comment {
        return String::new();
    }

    let mut output = String::new();

    // LEGEND block (if any tags are used)
    let unique_tags = collect_unique_tags(state);
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

        // Session comment (no prefix, directly indented)
        if let Some(ref comment) = state.session_comment {
            if !comment.is_empty() {
                let comment_text = render_content(comment);
                for line in comment_text.lines() {
                    output.push_str(&format!("  {}\n", line));
                }
            }
        }

        // Exit mode (original format: "Name (instruction)")
        if let Some(ref mode_id) = state.selected_exit_mode_id {
            if let Some(mode) = state.exit_modes.iter().find(|m| &m.id == mode_id) {
                output.push_str(&format!("  {} ({})\n", mode.name, mode.instruction));
            }
        }

        if has_annotations {
            output.push_str("\n---\n\n");
        }
    }

    if !has_annotations {
        return output;
    }

    // Collect and sort annotations by start line
    let mut annotations: Vec<&Annotation> = state.annotations.values().collect();
    annotations.sort_by_key(|a| a.start_line);

    // Calculate max line number width for alignment
    let max_line = annotations
        .iter()
        .map(|a| a.end_line)
        .max()
        .unwrap_or(0);
    let line_num_width = max_line.to_string().len();

    for (i, ann) in annotations.iter().enumerate() {
        if i > 0 {
            output.push_str("\n---\n\n");
        }
        format_annotation_block(&mut output, state, ann, line_num_width);
    }

    output
}

/// Format a single annotation block with context and content.
fn format_annotation_block(
    out: &mut String,
    state: &AppState,
    ann: &Annotation,
    line_num_width: usize,
) {
    // File header: "file.rs:10-15" or "file.rs:10"
    if ann.start_line == ann.end_line {
        out.push_str(&format!("{}:{}\n", state.label, ann.start_line));
    } else {
        out.push_str(&format!(
            "{}:{}-{}\n",
            state.label, ann.start_line, ann.end_line
        ));
    }

    // Context line (1 line before, if exists and non-empty)
    if ann.start_line > 1 {
        let context_line_num = ann.start_line - 1;
        if let Some(line) = state.lines.get((context_line_num - 1) as usize) {
            if !line.content.trim().is_empty() {
                // Format: "    N | content" (3 extra spaces for ">" prefix alignment)
                out.push_str(&format!(
                    "{:>width$} | {}\n",
                    context_line_num,
                    line.content,
                    width = line_num_width + 3
                ));
            }
        }
    }

    // Selected lines with ">" prefix
    for line_num in ann.start_line..=ann.end_line {
        if let Some(line) = state.lines.get((line_num - 1) as usize) {
            // Format: ">  N | content"
            out.push_str(&format!(
                "> {:>width$} | {}\n",
                line_num,
                line.content,
                width = line_num_width + 1
            ));
        }
    }

    // Annotation content with arrow
    let arrow_indent = " ".repeat(line_num_width + 4); // +4 for "> " and " | "
    let content_text = render_content(&ann.content);

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

/// Render content nodes to plain text.
fn render_content(nodes: &[ContentNode]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            ContentNode::Text { text } => text.clone(),
            ContentNode::Tag { name, .. } => format!("[# {}]", name),
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Line;
    use std::collections::HashMap;

    fn make_line(number: u32, content: &str) -> Line {
        Line {
            number,
            content: content.to_string(),
            html: None,
        }
    }

    fn make_state(label: &str, lines: Vec<Line>, annotations: HashMap<String, Annotation>) -> AppState {
        AppState {
            label: label.to_string(),
            lines,
            annotations,
            exit_modes: vec![],
            selected_exit_mode_id: None,
            session_comment: None,
        }
    }

    #[test]
    fn empty_annotations_returns_empty_string() {
        let state = make_state("test.rs", vec![], HashMap::new());
        assert_eq!(format_output(&state), "");
    }

    #[test]
    fn single_line_annotation() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "5-5".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

        assert!(output.contains("test.rs:5\n"));
        assert!(output.contains("> "));
        assert!(output.contains("line 5"));
        assert!(output.contains("└──> Fix this"));
    }

    #[test]
    fn multi_line_annotation() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "10-15".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

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
            "20-20".to_string(),
            Annotation {
                start_line: 20,
                end_line: 20,
                content: vec![ContentNode::Text {
                    text: "Second".to_string(),
                }],
            },
        );
        annotations.insert(
            "5-5".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

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
            "3-3".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

        // Line 2 is whitespace, shouldn't appear as context
        assert!(!output.contains("   \n"));
    }

    #[test]
    fn multiline_content_properly_indented() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "5-5".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

        // First line has arrow
        assert!(output.contains("└──> Line one"));
        // Continuation lines should be indented
        assert!(output.contains("Line two"));
        assert!(output.contains("Line three"));
    }

    #[test]
    fn session_block_with_exit_mode() {
        use crate::state::ExitMode;

        let exit_modes = vec![ExitMode {
            id: "apply".to_string(),
            name: "Apply".to_string(),
            color: "#22c55e".to_string(),
            instruction: "Apply the suggested changes".to_string(),
            order: 0,
            is_ephemeral: false,
        }];

        let state = AppState {
            label: "test.rs".to_string(),
            lines: vec![],
            annotations: HashMap::new(),
            exit_modes,
            selected_exit_mode_id: Some("apply".to_string()),
            session_comment: None,
        };

        let output = format_output(&state);

        assert!(output.contains("SESSION:"));
        assert!(output.contains("Apply (Apply the suggested changes)"));
    }

    #[test]
    fn session_block_with_annotations() {
        use crate::state::ExitMode;

        let exit_modes = vec![ExitMode {
            id: "reject".to_string(),
            name: "Reject".to_string(),
            color: "#ef4444".to_string(),
            instruction: "Do not apply".to_string(),
            order: 0,
            is_ephemeral: false,
        }];

        let mut annotations = HashMap::new();
        annotations.insert(
            "5-5".to_string(),
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

        let state = AppState {
            label: "test.rs".to_string(),
            lines,
            annotations,
            exit_modes,
            selected_exit_mode_id: Some("reject".to_string()),
            session_comment: None,
        };

        let output = format_output(&state);

        // SESSION block comes first
        let session_pos = output.find("SESSION:").unwrap();
        let annotation_pos = output.find("test.rs:5").unwrap();
        assert!(session_pos < annotation_pos);

        // Separator between SESSION and annotations
        assert!(output.contains("---"));
    }

    #[test]
    fn session_comment_in_output() {
        let state = AppState {
            label: "test.rs".to_string(),
            lines: vec![],
            annotations: HashMap::new(),
            exit_modes: vec![],
            selected_exit_mode_id: None,
            session_comment: Some(vec![ContentNode::Text {
                text: "This is a session comment".to_string(),
            }]),
        };

        let output = format_output(&state);

        assert!(output.contains("SESSION:"));
        assert!(output.contains("  This is a session comment"));
    }

    #[test]
    fn session_comment_with_exit_mode() {
        use crate::state::ExitMode;

        let exit_modes = vec![ExitMode {
            id: "apply".to_string(),
            name: "Apply".to_string(),
            color: "#22c55e".to_string(),
            instruction: "Apply changes".to_string(),
            order: 0,
            is_ephemeral: false,
        }];

        let state = AppState {
            label: "test.rs".to_string(),
            lines: vec![],
            annotations: HashMap::new(),
            exit_modes,
            selected_exit_mode_id: Some("apply".to_string()),
            session_comment: Some(vec![ContentNode::Text {
                text: "Overall looks good!".to_string(),
            }]),
        };

        let output = format_output(&state);

        // Session comment comes before exit mode
        let comment_pos = output.find("Overall looks good!").unwrap();
        let exit_pos = output.find("Apply (Apply changes)").unwrap();
        assert!(comment_pos < exit_pos);
    }

    #[test]
    fn empty_session_comment_not_rendered() {
        let state = AppState {
            label: "test.rs".to_string(),
            lines: vec![],
            annotations: HashMap::new(),
            exit_modes: vec![],
            selected_exit_mode_id: None,
            session_comment: Some(vec![]),
        };

        let output = format_output(&state);

        // Empty session comment should result in no output
        assert!(output.is_empty());
    }

    #[test]
    fn legend_block_with_tags() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "5-5".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

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
            "5-5".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

        // BUG should come before SECURITY (alphabetical)
        let bug_pos = output.find("[# BUG]").unwrap();
        let sec_pos = output.find("[# SECURITY]").unwrap();
        assert!(bug_pos < sec_pos);
    }

    #[test]
    fn tag_deduplication_in_legend() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "5-5".to_string(),
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
            "10-10".to_string(),
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

        let state = make_state("test.rs", lines, annotations);
        let output = format_output(&state);

        // SECURITY should only appear once in LEGEND
        let legend_end = output.find("\n\n").unwrap();
        let legend = &output[..legend_end];
        assert_eq!(legend.matches("[# SECURITY]").count(), 1);
    }
}
