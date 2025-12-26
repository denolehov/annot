//! Portal validation and loading.
//!
//! Portals are live code embeds: markdown links with line anchors
//! that expand inline with syntax-highlighted source code.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::highlight::Highlighter;
use crate::markdown::PortalInfo;
use crate::state::{Line, LineOrigin, LineSemantics, PortalSemantics};

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of portals per document.
pub const MAX_PORTALS: usize = 50;

/// Maximum number of lines per portal.
pub const MAX_LINES_PER_PORTAL: usize = 500;

/// Maximum characters per line before truncation.
pub const MAX_LINE_LENGTH: usize = 2000;

/// Sensitive path patterns that should be blocked.
const SENSITIVE_PATTERNS: &[&str] = &[
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    ".env",
    "credentials",
    "secrets",
    ".ssh/",
    ".aws/",
    ".gcp/",
    ".pem",
    ".key",
];

// =============================================================================
// Error types
// =============================================================================

/// Errors that can occur during portal validation or loading.
#[derive(Debug, Clone)]
pub enum PortalError {
    /// File not found at the resolved path.
    NotFound(PathBuf),
    /// Recursive markdown files are not allowed.
    RecursiveMarkdown,
    /// Path matches a sensitive pattern.
    Sensitive(String),
    /// File appears to be binary (contains null bytes).
    Binary,
    /// File content is not valid UTF-8.
    InvalidUtf8,
    /// Too many portals in document.
    TooManyPortals,
    /// Portal line range exceeds limit.
    TooManyLines,
    /// I/O error reading file.
    IoError(String),
}

impl std::fmt::Display for PortalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalError::NotFound(p) => write!(f, "File not found: {}", p.display()),
            PortalError::RecursiveMarkdown => write!(f, "Markdown files cannot be portals"),
            PortalError::Sensitive(p) => write!(f, "Sensitive path blocked: {}", p),
            PortalError::Binary => write!(f, "Binary file cannot be a portal"),
            PortalError::InvalidUtf8 => write!(f, "File is not valid UTF-8"),
            PortalError::TooManyPortals => write!(f, "Too many portals (max {})", MAX_PORTALS),
            PortalError::TooManyLines => {
                write!(f, "Portal exceeds line limit (max {})", MAX_LINES_PER_PORTAL)
            }
            PortalError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for PortalError {}

// =============================================================================
// Loaded portal
// =============================================================================

/// A fully loaded portal ready for interleaving.
#[derive(Clone, Debug)]
pub struct LoadedPortal {
    /// Canonical path to the source file.
    pub source_path: PathBuf,
    /// Display label (from link text or filename).
    pub label: String,
    /// Start line in the source file (1-indexed, clamped to file length).
    pub start_line: u32,
    /// End line in the source file (1-indexed, clamped to file length).
    pub end_line: u32,
    /// Line number in the markdown where this portal should be inserted.
    pub insert_at: u32,
    /// Generated lines: header + content + footer.
    pub lines: Vec<Line>,
}

// =============================================================================
// Validation
// =============================================================================

/// Validate a portal path and return the resolved path if valid.
///
/// Checks:
/// 1. Path resolves relative to `base_dir`
/// 2. File exists
/// 3. Not a markdown file (no recursion)
/// 4. Not a sensitive path
/// 5. Not a binary file
pub fn validate_portal(raw_path: &str, base_dir: &Path) -> Result<PathBuf, PortalError> {
    let resolved = base_dir.join(raw_path);

    // Check if file exists
    if !resolved.exists() {
        return Err(PortalError::NotFound(resolved));
    }

    // Check for markdown files (no recursion)
    if let Some(ext) = resolved.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if matches!(ext_lower.as_str(), "md" | "markdown" | "mdown" | "mkd") {
            return Err(PortalError::RecursiveMarkdown);
        }
    }

    // Check for sensitive paths
    let path_str = resolved.to_string_lossy().to_lowercase();
    for pattern in SENSITIVE_PATTERNS {
        if path_str.contains(pattern) {
            return Err(PortalError::Sensitive(pattern.to_string()));
        }
    }

    // Check for binary content (read first 8KB)
    let mut file = fs::File::open(&resolved).map_err(|e| PortalError::IoError(e.to_string()))?;
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer).map_err(|e| PortalError::IoError(e.to_string()))?;

    if buffer[..bytes_read].contains(&0) {
        return Err(PortalError::Binary);
    }

    Ok(resolved)
}

// =============================================================================
// Loading
// =============================================================================

/// Load a portal and generate its lines.
///
/// Returns `LoadedPortal` with header, content, and footer lines ready for interleaving.
pub fn load_portal(info: &PortalInfo, base_dir: &Path) -> Result<LoadedPortal, PortalError> {
    // Validate the path
    let source_path = validate_portal(&info.path, base_dir)?;

    // Read the file content
    let content = fs::read_to_string(&source_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            PortalError::InvalidUtf8
        } else {
            PortalError::IoError(e.to_string())
        }
    })?;

    // Count lines and clamp range
    let total_lines = content.lines().count() as u32;
    let start_line = info.start_line.min(total_lines).max(1);
    let end_line = info.end_line.min(total_lines).max(start_line);

    // Check line count limit
    let line_count = end_line - start_line + 1;
    if line_count > MAX_LINES_PER_PORTAL as u32 {
        return Err(PortalError::TooManyLines);
    }

    // Resolve label (use filename if not provided)
    let label = info
        .label
        .clone()
        .unwrap_or_else(|| {
            source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string()
        });

    // Build lines: header + content + footer
    let mut lines = Vec::new();

    // Header line
    let range_str = if start_line == end_line {
        format!("L{}", start_line)
    } else {
        format!("L{}-L{}", start_line, end_line)
    };
    let path_display = truncate_path(&info.path, 50);

    lines.push(Line {
        content: format!("{} — {}#{}", label, path_display, range_str),
        html: None,
        origin: LineOrigin::Virtual,
        semantics: LineSemantics::Portal(PortalSemantics::Header {
            label: label.clone(),
            path: info.path.clone(),
            range: range_str.clone(),
        }),
    });

    // Highlight the content
    let highlighter = Highlighter::new();
    let path_for_highlighting = source_path.to_string_lossy();
    let highlighted_lines = highlighter.highlight_lines(&content, &path_for_highlighting);

    // Extract the relevant lines (1-indexed to 0-indexed)
    let start_idx = (start_line - 1) as usize;
    let end_idx = end_line as usize;

    for (offset, html) in highlighted_lines
        .into_iter()
        .enumerate()
        .skip(start_idx)
        .take(end_idx - start_idx)
    {
        let source_line = (offset + 1) as u32;
        let raw_content: String = content
            .lines()
            .nth(offset)
            .map(|s| truncate_line(s, MAX_LINE_LENGTH))
            .unwrap_or_default();

        lines.push(Line {
            content: raw_content,
            html: Some(html),
            origin: LineOrigin::Source {
                path: source_path.to_string_lossy().to_string(),
                line: source_line,
            },
            semantics: LineSemantics::Portal(PortalSemantics::Content),
        });
    }

    // Footer line
    lines.push(Line {
        content: String::new(),
        html: None,
        origin: LineOrigin::Virtual,
        semantics: LineSemantics::Portal(PortalSemantics::Footer),
    });

    Ok(LoadedPortal {
        source_path,
        label,
        start_line,
        end_line,
        insert_at: info.source_line,
        lines,
    })
}

// =============================================================================
// Helpers
// =============================================================================

/// Truncate a path in the middle if it exceeds max length.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    // Keep first and last parts, elide middle
    let half = (max_len - 3) / 2;
    format!("{}...{}", &path[..half], &path[path.len() - half..])
}

/// Truncate a line if it exceeds max length.
fn truncate_line(line: &str, max_len: usize) -> String {
    if line.len() <= max_len {
        line.to_string()
    } else {
        format!("{}...", &line[..max_len - 3])
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn validate_portal_accepts_valid_file() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "test.rs", "fn main() {}");

        let result = validate_portal("test.rs", temp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_portal_rejects_missing_file() {
        let temp = TempDir::new().unwrap();

        let result = validate_portal("missing.rs", temp.path());
        assert!(matches!(result, Err(PortalError::NotFound(_))));
    }

    #[test]
    fn validate_portal_rejects_markdown() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "test.md", "# Title");

        let result = validate_portal("test.md", temp.path());
        assert!(matches!(result, Err(PortalError::RecursiveMarkdown)));
    }

    #[test]
    fn validate_portal_rejects_sensitive_paths() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".ssh")).unwrap();
        create_test_file(temp.path(), ".ssh/id_rsa", "secret key");

        let result = validate_portal(".ssh/id_rsa", temp.path());
        assert!(matches!(result, Err(PortalError::Sensitive(_))));
    }

    #[test]
    fn validate_portal_rejects_env_file() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), ".env", "SECRET=value");

        let result = validate_portal(".env", temp.path());
        assert!(matches!(result, Err(PortalError::Sensitive(_))));
    }

    #[test]
    fn validate_portal_rejects_binary() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("binary.bin");
        let mut file = File::create(&path).unwrap();
        file.write_all(&[0x00, 0x01, 0x02, 0x00]).unwrap();

        let result = validate_portal("binary.bin", temp.path());
        assert!(matches!(result, Err(PortalError::Binary)));
    }

    #[test]
    fn load_portal_generates_lines() {
        let temp = TempDir::new().unwrap();
        let content = "fn main() {\n    println!(\"Hello\");\n}\n";
        create_test_file(temp.path(), "test.rs", content);

        let info = PortalInfo {
            source_line: 1,
            label: Some("test".to_string()),
            path: "test.rs".to_string(),
            start_line: 1,
            end_line: 3,
        };

        let result = load_portal(&info, temp.path());
        assert!(result.is_ok());

        let portal = result.unwrap();
        assert_eq!(portal.label, "test");
        assert_eq!(portal.start_line, 1);
        assert_eq!(portal.end_line, 3);
        // Header + 3 content lines + footer = 5 lines
        assert_eq!(portal.lines.len(), 5);

        // Check header
        assert!(matches!(
            portal.lines[0].semantics,
            LineSemantics::Portal(PortalSemantics::Header { .. })
        ));

        // Check footer
        assert!(matches!(
            portal.lines[4].semantics,
            LineSemantics::Portal(PortalSemantics::Footer)
        ));
    }

    #[test]
    fn load_portal_clamps_out_of_range() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "short.rs", "line1\nline2\n");

        let info = PortalInfo {
            source_line: 1,
            label: None,
            path: "short.rs".to_string(),
            start_line: 1,
            end_line: 100, // Beyond file length
        };

        let result = load_portal(&info, temp.path());
        assert!(result.is_ok());

        let portal = result.unwrap();
        assert_eq!(portal.end_line, 2); // Clamped to file length
    }

    #[test]
    fn load_portal_uses_filename_when_no_label() {
        let temp = TempDir::new().unwrap();
        create_test_file(temp.path(), "myfile.rs", "content");

        let info = PortalInfo {
            source_line: 1,
            label: None,
            path: "myfile.rs".to_string(),
            start_line: 1,
            end_line: 1,
        };

        let result = load_portal(&info, temp.path());
        assert!(result.is_ok());

        let portal = result.unwrap();
        assert_eq!(portal.label, "myfile.rs");
    }

    #[test]
    fn truncate_path_handles_short_paths() {
        assert_eq!(truncate_path("short.rs", 50), "short.rs");
    }

    #[test]
    fn truncate_path_elides_long_paths() {
        let long_path = "very/long/path/to/some/deeply/nested/file.rs";
        let truncated = truncate_path(long_path, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.contains("..."));
    }

    #[test]
    fn truncate_line_handles_short_lines() {
        assert_eq!(truncate_line("short", 100), "short");
    }

    #[test]
    fn truncate_line_truncates_long_lines() {
        let long_line = "a".repeat(100);
        let truncated = truncate_line(&long_line, 50);
        assert_eq!(truncated.len(), 50);
        assert!(truncated.ends_with("..."));
    }
}
