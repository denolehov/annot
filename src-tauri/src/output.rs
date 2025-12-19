use crate::state::{Annotation, AppState, ContentNode};

/// Format all annotations as structured output for LLM consumption.
pub fn format_output(state: &AppState) -> String {
    if state.annotations.is_empty() {
        return String::new();
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

    let mut output = String::new();

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
}
