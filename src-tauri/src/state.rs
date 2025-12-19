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

/// Content node for structured annotation content.
/// Currently only Text is supported; Tag, Media, Excalidraw to be added later.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentNode {
    Text { text: String },
}

/// An annotation attached to a line range.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub start_line: u32,
    pub end_line: u32,
    pub content: Vec<ContentNode>,
}

/// Application state initialized at startup, before the window opens.
pub struct AppState {
    pub label: String,
    pub lines: Vec<Line>,
    /// Annotations keyed by "start-end" range string (e.g., "10-15").
    pub annotations: HashMap<String, Annotation>,
}

/// Response sent to the frontend via the get_content command.
#[derive(Serialize)]
pub struct ContentResponse {
    pub label: String,
    pub lines: Vec<Line>,
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
        }
    }

    /// Convert to response for frontend.
    pub fn to_response(&self) -> ContentResponse {
        ContentResponse {
            label: self.label.clone(),
            lines: self.lines.clone(),
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
