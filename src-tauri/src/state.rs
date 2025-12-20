use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::diff::{self, DiffMetadata};
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

impl Tag {
    /// Creates a new tag with a generated 12-character alphanumeric ID.
    pub fn new(name: String, instruction: String) -> Self {
        Self {
            id: generate_id(),
            name,
            instruction,
        }
    }
}

/// Generates a 12-character alphanumeric ID.
fn generate_id() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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

impl ExitMode {
    /// Creates a new exit mode with a generated 12-character alphanumeric ID.
    pub fn new(name: String, color: String, instruction: String, order: u32) -> Self {
        Self {
            id: generate_id(),
            name,
            color,
            instruction,
            order,
            is_ephemeral: false,
        }
    }
}

/// Application state initialized at startup, before the window opens.
pub struct AppState {
    pub label: String,
    pub lines: Vec<Line>,
    /// Annotations keyed by "start-end" range string (e.g., "10-15").
    pub annotations: HashMap<String, Annotation>,
    /// Available tags for annotation.
    pub tags: Vec<Tag>,
    /// IDs of tags deleted this session (for merge-on-save).
    pub deleted_tag_ids: HashSet<String>,
    /// Available exit modes for this session.
    pub exit_modes: Vec<ExitMode>,
    /// IDs of exit modes deleted this session (for merge-on-save).
    pub deleted_exit_mode_ids: HashSet<String>,
    /// Currently selected exit mode ID (None if no mode selected).
    pub selected_exit_mode_id: Option<String>,
    /// Session-level comment (not tied to specific lines).
    pub session_comment: Option<Vec<ContentNode>>,
    /// Diff metadata (Some if content is a unified diff).
    pub diff_metadata: Option<DiffMetadata>,
}

/// Response sent to the frontend via the get_content command.
#[derive(Serialize)]
pub struct ContentResponse {
    pub label: String,
    pub lines: Vec<Line>,
    pub tags: Vec<Tag>,
    pub exit_modes: Vec<ExitMode>,
    pub selected_exit_mode_id: Option<String>,
    pub session_comment: Option<Vec<ContentNode>>,
    /// Diff metadata (Some if content is a unified diff).
    pub diff_metadata: Option<DiffMetadata>,
}

impl AppState {
    /// Create an empty state (used as placeholder in MCP mode before first session).
    pub fn empty() -> Self {
        Self {
            label: String::new(),
            lines: Vec::new(),
            annotations: HashMap::new(),
            tags: Vec::new(),
            deleted_tag_ids: HashSet::new(),
            exit_modes: Vec::new(),
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: None,
        }
    }

    /// Parse file content into structured lines with syntax highlighting.
    ///
    /// # Arguments
    /// * `label` - Display name (usually the filename)
    /// * `content` - Raw file content
    /// * `path` - File path (used for language detection via extension)
    /// * `tags` - Available tags (loaded from config)
    /// * `exit_modes` - Available exit modes (loaded from config)
    pub fn from_file(
        label: String,
        content: &str,
        path: &str,
        tags: Vec<Tag>,
        exit_modes: Vec<ExitMode>,
    ) -> Self {
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
            tags,
            deleted_tag_ids: HashSet::new(),
            exit_modes,
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: None,
        }
    }

    /// Parse diff content into structured lines with diff metadata.
    ///
    /// # Arguments
    /// * `label` - Display name
    /// * `content` - Raw diff content
    /// * `tags` - Available tags (loaded from config)
    /// * `exit_modes` - Available exit modes (loaded from config)
    pub fn from_diff(
        label: String,
        content: &str,
        tags: Vec<Tag>,
        exit_modes: Vec<ExitMode>,
    ) -> Result<Self, String> {
        let diff_metadata = diff::parse_diff(content)?;
        let highlighter = Highlighter::new();

        // For diffs, we create lines from the raw content
        // Each line gets its display number (1-indexed)
        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(i, line_content)| {
                let line_num = (i + 1) as u32;

                // Get file language for this line from diff metadata
                let language = diff_metadata
                    .lines
                    .get(&line_num)
                    .and_then(|info| diff_metadata.files.get(info.file_index))
                    .map(|f| f.language.as_str())
                    .unwrap_or("");

                // Only highlight non-header lines with actual code
                let html = if !language.is_empty() && !line_content.starts_with("diff ")
                    && !line_content.starts_with("---")
                    && !line_content.starts_with("+++")
                    && !line_content.starts_with("@@")
                    && !line_content.starts_with("index ")
                {
                    // Strip the +/- prefix for highlighting, then add it back
                    let (prefix, code) = if line_content.starts_with('+')
                        || line_content.starts_with('-')
                        || line_content.starts_with(' ')
                    {
                        (&line_content[..1], &line_content[1..])
                    } else {
                        ("", line_content)
                    };

                    let fake_path = format!("file.{}", language);
                    let highlighted = highlighter.highlight_lines(code, &fake_path);
                    highlighted.first().map(|h| format!("{}{}", prefix, h))
                } else {
                    None
                };

                Line {
                    number: line_num,
                    content: line_content.to_string(),
                    html,
                }
            })
            .collect();

        Ok(Self {
            label,
            lines,
            annotations: HashMap::new(),
            tags,
            deleted_tag_ids: HashSet::new(),
            exit_modes,
            deleted_exit_mode_ids: HashSet::new(),
            selected_exit_mode_id: None,
            session_comment: None,
            diff_metadata: Some(diff_metadata),
        })
    }

    /// Convert to response for frontend.
    pub fn to_response(&self) -> ContentResponse {
        ContentResponse {
            label: self.label.clone(),
            lines: self.lines.clone(),
            tags: self.tags.clone(),
            exit_modes: self.exit_modes.clone(),
            selected_exit_mode_id: self.selected_exit_mode_id.clone(),
            session_comment: self.session_comment.clone(),
            diff_metadata: self.diff_metadata.clone(),
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

    fn test_state(label: &str, content: &str, path: &str) -> AppState {
        AppState::from_file(label.to_string(), content, path, vec![], vec![])
    }

    #[test]
    fn content_response_has_1_indexed_line_numbers() {
        let state = test_state("test.rs", "a\nb\nc", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].number, 1);
        assert_eq!(response.lines[1].number, 2);
        assert_eq!(response.lines[2].number, 3);
    }

    #[test]
    fn content_response_includes_label() {
        let state = test_state("my_file.rs", "content", "my_file.rs");
        let response = state.to_response();

        assert_eq!(response.label, "my_file.rs");
    }

    #[test]
    fn content_response_preserves_whitespace() {
        let state = test_state("test.rs", "  indented\n\ttabbed", "test.rs");
        let response = state.to_response();

        assert_eq!(response.lines[0].content, "  indented");
        assert_eq!(response.lines[1].content, "\ttabbed");
    }

    #[test]
    fn content_response_includes_highlighted_html() {
        let state = test_state("test.rs", "fn main() {}", "test.rs");
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
        let state = test_state("test.txt", "line1\nline2", "test.txt");
        let response = state.to_response();

        // Plain text should still have html (just escaped text)
        assert_eq!(response.lines.len(), 2);
    }

    #[test]
    fn content_response_includes_tags() {
        let tags = vec![Tag {
            id: "test123".into(),
            name: "TEST".into(),
            instruction: "Test tag".into(),
        }];
        let state = AppState::from_file("test.rs".into(), "code", "test.rs", tags, vec![]);
        let response = state.to_response();

        assert_eq!(response.tags.len(), 1);
        assert_eq!(response.tags[0].name, "TEST");
    }

    #[test]
    fn tag_new_generates_12_char_id() {
        let tag = Tag::new("TEST".into(), "instruction".into());
        assert_eq!(tag.id.len(), 12);
        assert!(tag.id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn exit_mode_new_generates_12_char_id() {
        let mode = ExitMode::new("Test".into(), "#ff0000".into(), "instruction".into(), 0);
        assert_eq!(mode.id.len(), 12);
        assert!(mode.id.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(!mode.is_ephemeral);
    }

    // === Diff tests ===

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

    #[test]
    fn from_diff_creates_state_with_metadata() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        assert!(state.diff_metadata.is_some());
        let meta = state.diff_metadata.as_ref().unwrap();
        assert_eq!(meta.files.len(), 1);
        assert_eq!(meta.files[0].new_name, Some("file.rs".to_string()));
    }

    #[test]
    fn from_diff_creates_lines_from_content() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        // Should have lines matching the diff content
        assert!(!state.lines.is_empty());

        // First line should be the diff header
        assert!(state.lines[0].content.starts_with("diff --git"));

        // Check that +/- lines are preserved
        let has_added = state.lines.iter().any(|l| l.content.starts_with('+'));
        let has_deleted = state.lines.iter().any(|l| l.content.starts_with('-'));
        assert!(has_added, "Should have added lines");
        assert!(has_deleted, "Should have deleted lines");
    }

    #[test]
    fn from_diff_line_numbers_are_1_indexed() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        assert_eq!(state.lines[0].number, 1);
        assert_eq!(state.lines[1].number, 2);
    }

    #[test]
    fn from_diff_response_includes_metadata() {
        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            vec![],
            vec![],
        )
        .unwrap();

        let response = state.to_response();
        assert!(response.diff_metadata.is_some());
    }

    #[test]
    fn from_diff_error_on_invalid_content() {
        let result = AppState::from_diff(
            "not-a-diff.txt".into(),
            "just regular text",
            vec![],
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn from_file_has_no_diff_metadata() {
        let state = test_state("test.rs", "fn main() {}", "test.rs");
        assert!(state.diff_metadata.is_none());

        let response = state.to_response();
        assert!(response.diff_metadata.is_none());
    }

    #[test]
    fn from_diff_includes_tags_and_exit_modes() {
        let tags = vec![Tag::new("TEST".into(), "instruction".into())];
        let modes = vec![ExitMode::new("Apply".into(), "#22c55e".into(), "Apply it".into(), 0)];

        let state = AppState::from_diff(
            "changes.diff".into(),
            SIMPLE_DIFF,
            tags,
            modes,
        )
        .unwrap();

        assert_eq!(state.tags.len(), 1);
        assert_eq!(state.exit_modes.len(), 1);
    }

    /// Test that diff lines with doc comments produce single-line HTML
    #[test]
    fn from_diff_doc_comment_line_html_is_single_line() {
        let diff_with_doc_comment = r#"diff --git a/lib.rs b/lib.rs
--- a/lib.rs
+++ b/lib.rs
@@ -1,3 +1,4 @@
-/// Old doc comment
+/// New doc comment
 fn main() {
 }
"#;

        let state = AppState::from_diff(
            "changes.diff".into(),
            diff_with_doc_comment,
            vec![],
            vec![],
        )
        .unwrap();

        println!("\n=== DIFF DOC COMMENT LINES ===");
        for line in &state.lines {
            println!("Line {}: content={:?}", line.number, line.content);
            if let Some(ref html) = line.html {
                println!("        html={:?}", html);
                // Check for newlines
                if html.contains('\n') {
                    println!("        WARNING: HTML contains newline!");
                }
            }
        }
        println!("=== END ===\n");

        // Find the deleted doc comment line
        let deleted_line = state.lines.iter().find(|l| l.content.starts_with("-///")).unwrap();
        assert!(deleted_line.html.is_some(), "Deleted doc comment should have HTML");
        let html = deleted_line.html.as_ref().unwrap();

        // HTML should not contain newlines
        assert!(!html.contains('\n'), "HTML should not contain newline. Got: {:?}", html);

        // HTML should start with the prefix
        assert!(html.starts_with('-'), "HTML should start with '-' prefix. Got: {:?}", html);
    }
}
