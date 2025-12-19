use serde::Serialize;

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

/// Application state initialized at startup, before the window opens.
pub struct AppState {
    pub label: String,
    pub lines: Vec<Line>,
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

        Self { label, lines }
    }

    /// Convert to response for frontend.
    pub fn to_response(&self) -> ContentResponse {
        ContentResponse {
            label: self.label.clone(),
            lines: self.lines.clone(),
        }
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
