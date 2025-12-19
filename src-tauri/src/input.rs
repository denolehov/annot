//! Input mode handling for different content sources.
//!
//! Supports reading from files or stdin, with future extensibility for MCP.

use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

/// The source of input content.
pub enum InputMode {
    /// Read content from a file path.
    File { path: PathBuf },
    /// Read content from stdin with an optional label for display/highlighting.
    Stdin { label: String },
    // Future: Mcp { session_id: String, content: String, label: String }
}

/// Resolved input ready for use by AppState.
#[derive(Debug)]
pub struct ResolvedInput {
    /// Display name (filename or custom label).
    pub label: String,
    /// Raw content string.
    pub content: String,
    /// Path hint for language detection (uses extension).
    pub path_hint: String,
}

impl InputMode {
    /// Resolve the input mode to content and metadata.
    ///
    /// # Errors
    /// Returns an error string if reading fails or content is empty.
    pub fn resolve(self) -> Result<ResolvedInput, String> {
        match self {
            InputMode::File { path } => {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Error reading file '{}': {}", path.display(), e))?;

                let label = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());

                let path_hint = path.to_string_lossy().to_string();

                Ok(ResolvedInput {
                    label,
                    content,
                    path_hint,
                })
            }
            InputMode::Stdin { label } => {
                let mut content = String::new();
                io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|e| format!("Error reading stdin: {}", e))?;

                if content.is_empty() {
                    return Err("Error: stdin is empty".to_string());
                }

                // Use label for both display and language detection
                let path_hint = label.clone();

                Ok(ResolvedInput {
                    label,
                    content,
                    path_hint,
                })
            }
        }
    }

    /// Detect the appropriate input mode from CLI arguments and stdin state.
    ///
    /// Returns the input mode and optionally a warning message.
    /// File argument takes priority over stdin when both are present.
    pub fn detect(file: Option<PathBuf>, label: String) -> Result<(InputMode, Option<String>), String> {
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
            Err("Error: no input provided\nUsage: annot <file> or <command> | annot\nTry: annot --help".to_string())
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

        assert_eq!(resolved.label, "test.rs");
        assert_eq!(resolved.content, "fn main() {}");
        assert!(resolved.path_hint.ends_with("test.rs"));
    }

    #[test]
    fn file_mode_error_on_missing_file() {
        let mode = InputMode::File {
            path: PathBuf::from("/nonexistent/file.rs"),
        };
        let result = mode.resolve();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Error reading file"));
    }

    #[test]
    fn file_mode_extracts_filename_as_label() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("deeply").join("nested").join("file.go");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "package main").unwrap();

        let mode = InputMode::File { path: file_path };
        let resolved = mode.resolve().unwrap();

        assert_eq!(resolved.label, "file.go");
    }

    // Note: Stdin mode tests require subprocess spawning or mock injection,
    // which is complex. Manual testing covers stdin scenarios.
}
