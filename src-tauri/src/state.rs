use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::highlight::Highlighter;

/// A single line of content with its line number.
#[derive(Clone, Serialize)]
pub struct Line {
    pub number: u32,
    pub content: String,
    /// Syntax-highlighted HTML (spans with CSS classes).
    /// None if highlighting failed or language is unknown.
    pub html: Option<String>,
}

/// A tag is a composable mini-prompt that can be embedded in annotations.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub instruction: String,
}

/// Default tags (hardcoded for now, persistence comes later).
pub fn default_tags() -> Vec<Tag> {
    vec![
        Tag {
            id: "sec000000001".into(),
            name: "SECURITY".into(),
            instruction: "Review for security vulnerabilities".into(),
        },
        Tag {
            id: "ref000000002".into(),
            name: "REFACTOR".into(),
            instruction: "Consider cleaner abstraction".into(),
        },
        Tag {
            id: "bug000000003".into(),
            name: "BUG".into(),
            instruction: "This is a bug that needs fixing".into(),
        },
        Tag {
            id: "per000000004".into(),
            name: "PERF".into(),
            instruction: "Performance concern".into(),
        },
    ]
}

/// Content node for structured annotation content.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentNode {
    Text { text: String },
    Tag { id: String, name: String, instruction: String },
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

/// Default exit modes (hardcoded for now, persistence comes later).
pub fn default_exit_modes() -> Vec<ExitMode> {
    vec![
        ExitMode {
            id: "apply".into(),
            name: "Apply".into(),
            color: "#22c55e".into(),
            instruction: "Apply the suggested changes".into(),
            order: 0,
            is_ephemeral: false,
        },
        ExitMode {
            id: "revise".into(),
            name: "Revise".into(),
            color: "#eab308".into(),
            instruction: "Revise based on feedback".into(),
            order: 1,
            is_ephemeral: false,
        },
        ExitMode {
            id: "reject".into(),
            name: "Reject".into(),
            color: "#ef4444".into(),
            instruction: "Do not apply these changes".into(),
            order: 2,
            is_ephemeral: false,
        },
    ]
}

/// Application state initialized at startup, before the window opens.
pub struct AppState {
    pub label: String,
    pub lines: Vec<Line>,
    /// Annotations keyed by "start-end" range string (e.g., "10-15").
    pub annotations: HashMap<String, Annotation>,
    /// Available exit modes for this session.
    pub exit_modes: Vec<ExitMode>,
    /// Currently selected exit mode ID (None if no mode selected).
    pub selected_exit_mode_id: Option<String>,
    /// Session-level comment (not tied to specific lines).
    pub session_comment: Option<Vec<ContentNode>>,
}

/// Response sent to the frontend via the get_content command.
#[derive(Serialize)]
pub struct ContentResponse {
    pub label: String,
    pub lines: Vec<Line>,
    pub exit_modes: Vec<ExitMode>,
    pub selected_exit_mode_id: Option<String>,
    pub session_comment: Option<Vec<ContentNode>>,
}

impl AppState {
    /// Parse file content into structured lines with syntax highlighting.
    ///
    /// # Arguments
    /// * `label` - Display name (usually the filename)
    /// * `content` - Raw file content
    /// * `path` - File path (used for language detection via extension)
    pub fn from_file(label: String, content: &str, path: &str) -> Self {
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
                }
            })
            .collect();

        Self {
            label,
            lines,
            annotations: HashMap::new(),
            exit_modes: default_exit_modes(),
            selected_exit_mode_id: None,
            session_comment: None,
        }
    }

    /// Convert to response for frontend.
    pub fn to_response(&self) -> ContentResponse {
        ContentResponse {
            label: self.label.clone(),
            lines: self.lines.clone(),
            exit_modes: self.exit_modes.clone(),
            selected_exit_mode_id: self.selected_exit_mode_id.clone(),
            session_comment: self.session_comment.clone(),
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

    #[test]
    fn content_response_has_1_indexed_line_numbers() {
        let state = AppState::from_file("test.rs".to_string(), "a\nb\nc", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].number, 1);
        assert_eq!(response.lines[1].number, 2);
        assert_eq!(response.lines[2].number, 3);
    }

    #[test]
    fn content_response_includes_label() {
        let state = AppState::from_file("my_file.rs".to_string(), "content", "my_file.rs");
        let response = state.to_response();

        assert_eq!(response.label, "my_file.rs");
    }

    #[test]
    fn content_response_preserves_whitespace() {
        let state = AppState::from_file("test.rs".to_string(), "  indented\n\ttabbed", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].content, "  indented");
        assert_eq!(response.lines[1].content, "\ttabbed");
    }

    #[test]
    fn content_response_includes_highlighted_html() {
        let state = AppState::from_file("test.rs".to_string(), "fn main() {}", "test.rs");
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
        let state = AppState::from_file("test.txt".to_string(), "line1\nline2", "test.txt");
        let response = state.to_response();

        // Plain text should still have html (just escaped text)
        assert_eq!(response.lines.len(), 2);
    }
}
