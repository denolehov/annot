//! Input mode handling for different content sources.
//!
//! Two orthogonal axes:
//!
//! ```text
//! ContentSource (where from?)  x  RenderingMode (how to display?)
//!      Cli::File                      Source
//!      Cli::Stdin                     Markdown
//!      Mcp::File                      Diff
//!      Mcp::Content
//!      Mcp::Diff
//! ```
//!
//! Any combination is valid — a CLI file can be a diff, an MCP file can be markdown.

use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use crate::diff;
use crate::error::AnnotError;
use crate::review::FileKey;
use crate::markdown;

/// How the content should be rendered/processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderingMode {
    /// Parse as unified diff with line-level change tracking.
    Diff,
    /// Render with markdown formatting (inline styles, code blocks).
    Markdown,
    /// Syntax highlight as source code.
    Source,
}

// ============================================================================
// ContentSource — where content originated
// ============================================================================

/// Where content originated. Determines available features.
///
/// - CLI mode outputs to stdout (no image support)
/// - MCP mode outputs via MCP response (image paste enabled)
#[derive(Debug, Clone)]
pub enum ContentSource {
    /// CLI mode: content from file or stdin, output to stdout.
    Cli(CliSource),
    /// MCP mode: content from MCP tool, output via MCP response.
    Mcp(McpSource),
}

/// CLI content source.
#[derive(Debug, Clone)]
pub enum CliSource {
    /// File path provided as CLI argument.
    File { path: PathBuf },
    /// Piped from stdin with user-provided label.
    Stdin { label: String },
}

/// MCP content source (which tool was called).
#[derive(Debug, Clone)]
pub enum McpSource {
    /// `review_file` tool — opens a real file.
    File { path: PathBuf },
    /// `review_content` tool — agent-generated content.
    Content { label: String },
    /// `review_diff` tool — git or raw diff.
    Diff {
        label: Option<String>,
        source: DiffSource,
    },
}

/// How a diff was obtained.
#[derive(Debug, Clone)]
pub enum DiffSource {
    /// Generated from git with these args (e.g., `["--staged"]`).
    Git { args: Vec<String> },
    /// Raw diff content provided directly.
    Raw,
}

impl ContentSource {
    /// Whether this is an MCP session.
    pub fn is_mcp(&self) -> bool {
        matches!(self, ContentSource::Mcp(_))
    }

    /// Whether image paste is allowed.
    ///
    /// MCP mode can return images in responses; CLI stdout cannot.
    pub fn allows_image_paste(&self) -> bool {
        self.is_mcp()
    }

    /// Label for content identification.
    /// Returns full path for files (matches LineOrigin.path for consistency).
    /// Frontend truncates for display.
    pub fn label(&self) -> &str {
        match self {
            ContentSource::Cli(CliSource::File { path })
            | ContentSource::Mcp(McpSource::File { path }) => path
                .to_str()
                .unwrap_or("file"),
            ContentSource::Cli(CliSource::Stdin { label })
            | ContentSource::Mcp(McpSource::Content { label }) => label,
            ContentSource::Mcp(McpSource::Diff { label, .. }) => {
                label.as_deref().unwrap_or("diff")
            }
        }
    }

    /// Path hint for language detection.
    pub fn path_hint(&self) -> Option<&str> {
        match self {
            ContentSource::Cli(CliSource::File { path })
            | ContentSource::Mcp(McpSource::File { path }) => path.to_str(),
            ContentSource::Cli(CliSource::Stdin { label })
            | ContentSource::Mcp(McpSource::Content { label }) => Some(label),
            ContentSource::Mcp(McpSource::Diff { label, .. }) => label.as_deref(),
        }
    }

    /// Base directory for resolving relative paths (e.g., portal links).
    ///
    /// Returns the parent directory of the source file, or current working directory
    /// if the source doesn't have a file path.
    pub fn base_dir(&self) -> PathBuf {
        match self {
            ContentSource::Cli(CliSource::File { path })
            | ContentSource::Mcp(McpSource::File { path }) => {
                path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                })
            }
            _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Generate FileKey for this content source.
    ///
    /// This key is used for annotation routing in Review.files.
    pub fn file_key(&self) -> FileKey {
        match self {
            ContentSource::Cli(CliSource::File { path })
            | ContentSource::Mcp(McpSource::File { path }) => FileKey::path(path.clone()),
            ContentSource::Cli(CliSource::Stdin { label }) => FileKey::ephemeral(label.clone()),
            ContentSource::Mcp(McpSource::Content { label }) => FileKey::ephemeral(label.clone()),
            ContentSource::Mcp(McpSource::Diff { .. }) => {
                // Diff mode uses DiffFile keys per file, not a single key for the whole diff
                unreachable!("Diff mode uses DiffFile keys per file")
            }
        }
    }
}

// ============================================================================
// InputMode — CLI-only input handling (will resolve to ContentSource::Cli)
// ============================================================================

/// The source of input content (CLI mode only).
pub enum InputMode {
    /// Read content from a file path.
    File { path: PathBuf },
    /// Read content from stdin with an optional label for display/highlighting.
    Stdin { label: String },
}

/// Resolved input ready for use by AppState.
#[derive(Debug)]
pub struct ResolvedInput {
    /// Raw content string.
    pub content: String,
    /// How the content should be rendered.
    pub rendering_mode: RenderingMode,
    /// Where the content came from.
    pub content_source: ContentSource,
}

impl InputMode {
    /// Resolve the input mode to content and metadata.
    ///
    /// # Errors
    /// Returns an error if reading fails or content is empty.
    pub fn resolve(self) -> Result<ResolvedInput, AnnotError> {
        match self {
            InputMode::File { path } => {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| AnnotError::io(e, format!("reading file '{}'", path.display())))?;

                let path_str = path.to_string_lossy();
                let rendering_mode = if diff::is_diff(&content) {
                    RenderingMode::Diff
                } else if markdown::is_markdown(&path_str) {
                    RenderingMode::Markdown
                } else {
                    RenderingMode::Source
                };

                Ok(ResolvedInput {
                    content,
                    rendering_mode,
                    content_source: ContentSource::Cli(CliSource::File { path }),
                })
            }
            InputMode::Stdin { label } => {
                let mut content = String::new();
                io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|e| AnnotError::io(e, "reading stdin"))?;

                if content.is_empty() {
                    return Err(AnnotError::Validation("stdin is empty".into()));
                }

                let rendering_mode = if diff::is_diff(&content) {
                    RenderingMode::Diff
                } else if markdown::is_markdown(&label) {
                    RenderingMode::Markdown
                } else {
                    RenderingMode::Source
                };

                Ok(ResolvedInput {
                    content,
                    rendering_mode,
                    content_source: ContentSource::Cli(CliSource::Stdin { label }),
                })
            }
        }
    }

    /// Detect the appropriate input mode from CLI arguments and stdin state.
    ///
    /// Returns the input mode and optionally a warning message.
    /// File argument takes priority over stdin when both are present.
    pub fn detect(file: Option<PathBuf>, label: String) -> Result<(InputMode, Option<String>), AnnotError> {
        let has_stdin = !io::stdin().is_terminal();

        if let Some(path) = file {
            let warning = if has_stdin {
                Some("Warning: both stdin and file argument provided, using file".to_string())
            } else {
                None
            };
            Ok((InputMode::File { path }, warning))
        } else if has_stdin {
            Ok((InputMode::Stdin { label }, None))
        } else {
            Err(AnnotError::Validation(
                "no input provided\nUsage: annot <file> or <command> | annot\nTry: annot --help".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_mode_reads_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let mode = InputMode::File { path: file_path.clone() };
        let resolved = mode.resolve().unwrap();

        // Label is full path (matches LineOrigin.path for consistency)
        assert_eq!(resolved.content_source.label(), file_path.to_str().unwrap());
        assert_eq!(resolved.content, "fn main() {}");
        assert!(resolved.content_source.path_hint().unwrap().ends_with("test.rs"));
    }

    #[test]
    fn file_mode_error_on_missing_file() {
        let mode = InputMode::File {
            path: PathBuf::from("/nonexistent/file.rs"),
        };
        let result = mode.resolve();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AnnotError::Io { .. }));
        assert!(err.to_string().contains("reading file"));
    }

    #[test]
    fn file_mode_uses_full_path_as_label() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("deeply").join("nested").join("file.go");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "package main").unwrap();

        let mode = InputMode::File { path: file_path.clone() };
        let resolved = mode.resolve().unwrap();

        // Label is full path (matches LineOrigin.path for consistency)
        assert_eq!(resolved.content_source.label(), file_path.to_str().unwrap());
    }

    // Note: Stdin mode tests require subprocess spawning or mock injection,
    // which is complex. Manual testing covers stdin scenarios.

    #[test]
    fn file_mode_detects_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let mode = InputMode::File { path: file_path };
        let resolved = mode.resolve().unwrap();

        assert_eq!(resolved.rendering_mode, RenderingMode::Source);
        assert!(!resolved.content_source.allows_image_paste()); // CLI mode
    }

    #[test]
    fn file_mode_detects_diff_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("changes.diff");
        std::fs::write(
            &file_path,
            r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,3 @@
 fn main() {
-    old();
+    new();
 }
"#,
        )
        .unwrap();

        let mode = InputMode::File { path: file_path };
        let resolved = mode.resolve().unwrap();

        assert_eq!(resolved.rendering_mode, RenderingMode::Diff);
    }

    #[test]
    fn content_source_cli_disallows_image_paste() {
        let source = ContentSource::Cli(CliSource::Stdin {
            label: "test.md".to_string(),
        });
        assert!(!source.allows_image_paste());
    }

    #[test]
    fn content_source_mcp_allows_image_paste() {
        let source = ContentSource::Mcp(McpSource::Content {
            label: "plan.md".to_string(),
        });
        assert!(source.allows_image_paste());
    }
}
