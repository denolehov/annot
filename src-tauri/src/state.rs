use serde::Serialize;

/// A single line of content with its line number.
#[derive(Clone, Serialize)]
pub struct Line {
    pub number: u32,
    pub content: String,
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
    /// Parse file content into structured lines.
    pub fn from_file(label: String, content: &str) -> Self {
        let lines = content
            .lines()
            .enumerate()
            .map(|(i, line)| Line {
                number: (i + 1) as u32,
                content: line.to_string(),
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
        let state = AppState::from_file("test.rs".to_string(), "a\nb\nc");
        let response = state.to_response();

        assert_eq!(response.lines[0].number, 1);
        assert_eq!(response.lines[1].number, 2);
        assert_eq!(response.lines[2].number, 3);
    }

    #[test]
    fn content_response_includes_label() {
        let state = AppState::from_file("my_file.rs".to_string(), "content");
        let response = state.to_response();

        assert_eq!(response.label, "my_file.rs");
    }

    #[test]
    fn content_response_preserves_whitespace() {
        let state = AppState::from_file("test.rs".to_string(), "  indented\n\ttabbed");
        let response = state.to_response();

        assert_eq!(response.lines[0].content, "  indented");
        assert_eq!(response.lines[1].content, "\ttabbed");
    }
}
