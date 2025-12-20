//! Unified diff parsing and detection.

use std::collections::HashMap;

use serde::Serialize;
use unidiff::PatchSet;

/// Line type in a diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineKind {
    Context,
    Added,
    Deleted,
    Header,
}

/// Metadata for a single line in the flattened diff view.
#[derive(Clone, Debug, Serialize)]
pub struct DiffLineInfo {
    pub kind: DiffLineKind,
    /// Original file line number (None for added lines and headers).
    pub old_line_num: Option<u32>,
    /// New file line number (None for deleted lines and headers).
    pub new_line_num: Option<u32>,
    /// Index into the files array.
    pub file_index: usize,
}

/// Metadata for a single file in the diff.
#[derive(Clone, Debug, Serialize)]
pub struct DiffFileInfo {
    pub old_name: Option<String>,
    pub new_name: Option<String>,
    /// Detected language (from extension).
    pub language: String,
    /// 1-indexed start line in flattened view.
    pub start_line: u32,
    /// 1-indexed end line in flattened view.
    pub end_line: u32,
}

/// Parsed diff metadata for rendering.
#[derive(Clone, Debug, Serialize)]
pub struct DiffMetadata {
    pub files: Vec<DiffFileInfo>,
    /// Map from display line number (1-indexed) to line info.
    pub lines: HashMap<u32, DiffLineInfo>,
}

/// Check if content appears to be a unified diff.
pub fn is_diff(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }
    let mut patch = PatchSet::new();
    patch.parse(content).is_ok() && !patch.files().is_empty()
}

/// Parse unified diff content into metadata by iterating raw lines.
/// This ensures line numbers match the actual content display.
pub fn parse_diff(content: &str) -> Result<DiffMetadata, String> {
    // Validate it's a diff first
    let mut patch = PatchSet::new();
    patch
        .parse(content)
        .map_err(|e| format!("Failed to parse diff: {:?}", e))?;

    if patch.files().is_empty() {
        return Err("Not a valid diff".to_string());
    }

    let mut metadata = DiffMetadata {
        files: Vec::new(),
        lines: HashMap::new(),
    };

    // Track state while iterating raw lines
    let mut current_file_idx: usize = 0;
    let mut current_old_line: u32 = 0;
    let mut current_new_line: u32 = 0;
    let mut in_hunk = false;

    // Build file info from parsed data
    for file in patch.files() {
        let new_name = if file.target_file == "/dev/null" {
            None
        } else {
            Some(file.target_file.trim_start_matches("b/").to_string())
        };
        let old_name = if file.source_file == "/dev/null" {
            None
        } else {
            Some(file.source_file.trim_start_matches("a/").to_string())
        };

        let language = new_name
            .as_ref()
            .or(old_name.as_ref())
            .and_then(|name| {
                std::path::Path::new(name)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        metadata.files.push(DiffFileInfo {
            old_name,
            new_name,
            language,
            start_line: 0, // Will be updated
            end_line: 0,   // Will be updated
        });
    }

    // Iterate raw content lines to build line metadata
    for (idx, line_content) in content.lines().enumerate() {
        let line_num = (idx + 1) as u32;

        // Detect line type by prefix
        let (kind, old_num, new_num) = if line_content.starts_with("diff --git ") {
            // New file header - update file tracking
            if current_file_idx > 0 || in_hunk {
                // Update previous file's end line
                if let Some(f) = metadata.files.get_mut(current_file_idx) {
                    f.end_line = line_num - 1;
                }
                current_file_idx += 1;
            }
            if let Some(f) = metadata.files.get_mut(current_file_idx) {
                f.start_line = line_num;
            }
            in_hunk = false;
            (DiffLineKind::Header, None, None)
        } else if line_content.starts_with("index ")
            || line_content.starts_with("--- ")
            || line_content.starts_with("+++ ")
            || line_content.starts_with("new file mode")
            || line_content.starts_with("deleted file mode")
            || line_content.starts_with("old mode")
            || line_content.starts_with("new mode")
            || line_content.starts_with("similarity index")
            || line_content.starts_with("rename from")
            || line_content.starts_with("rename to")
            || line_content.starts_with("Binary files")
        {
            (DiffLineKind::Header, None, None)
        } else if line_content.starts_with("@@ ") {
            // Hunk header - parse line numbers
            in_hunk = true;
            if let Some((old_start, new_start)) = parse_hunk_header(line_content) {
                current_old_line = old_start;
                current_new_line = new_start;
            }
            (DiffLineKind::Header, None, None)
        } else if in_hunk {
            // Inside a hunk - determine line type
            if line_content.starts_with('+') {
                let new_num = current_new_line;
                current_new_line += 1;
                (DiffLineKind::Added, None, Some(new_num))
            } else if line_content.starts_with('-') {
                let old_num = current_old_line;
                current_old_line += 1;
                (DiffLineKind::Deleted, Some(old_num), None)
            } else if line_content.starts_with(' ') || line_content.is_empty() {
                let old_num = current_old_line;
                let new_num = current_new_line;
                current_old_line += 1;
                current_new_line += 1;
                (DiffLineKind::Context, Some(old_num), Some(new_num))
            } else {
                // Unknown line in hunk - treat as context
                let old_num = current_old_line;
                let new_num = current_new_line;
                current_old_line += 1;
                current_new_line += 1;
                (DiffLineKind::Context, Some(old_num), Some(new_num))
            }
        } else {
            // Outside hunk - treat as header
            (DiffLineKind::Header, None, None)
        };

        metadata.lines.insert(
            line_num,
            DiffLineInfo {
                kind,
                old_line_num: old_num,
                new_line_num: new_num,
                file_index: current_file_idx,
            },
        );
    }

    // Update last file's end line
    let total_lines = content.lines().count() as u32;
    if let Some(f) = metadata.files.get_mut(current_file_idx) {
        f.end_line = total_lines;
    }

    Ok(metadata)
}

/// Parse hunk header like "@@ -1,10 +1,12 @@" to extract start line numbers.
fn parse_hunk_header(header: &str) -> Option<(u32, u32)> {
    // Format: @@ -old_start,old_count +new_start,new_count @@
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let old_part = parts.get(1)?; // "-1,10"
    let new_part = parts.get(2)?; // "+1,12"

    let old_start = old_part
        .trim_start_matches('-')
        .split(',')
        .next()?
        .parse::<u32>()
        .ok()?;

    let new_start = new_part
        .trim_start_matches('+')
        .split(',')
        .next()?
        .parse::<u32>()
        .ok()?;

    Some((old_start, new_start))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_DIFF: &str = r#"diff --git a/file.rs b/file.rs
index 1234567..abcdefg 100644
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,4 @@
 fn main() {
-    old_code();
+    new_code();
+    more_code();
 }
"#;

    const NEW_FILE_DIFF: &str = r#"diff --git a/added_file b/added_file
new file mode 100644
index 0000000..9b710f3
--- /dev/null
+++ b/added_file
@@ -0,0 +1,4 @@
+This was missing!
+Adding it now.
+
+Only for testing purposes."#;

    #[test]
    fn is_diff_returns_true_for_valid_diff() {
        assert!(is_diff(SIMPLE_DIFF));
    }

    #[test]
    fn is_diff_returns_true_for_new_file_diff() {
        assert!(is_diff(NEW_FILE_DIFF));
    }

    #[test]
    fn is_diff_returns_false_for_empty() {
        assert!(!is_diff(""));
    }

    #[test]
    fn is_diff_returns_false_for_regular_content() {
        assert!(!is_diff("fn main() {\n    println!(\"hello\");\n}"));
    }

    #[test]
    fn is_diff_returns_false_for_partial_diff_like_content() {
        // Content that looks like diff but isn't valid
        // Note: unidiff is permissive, so we test truly invalid content
        assert!(!is_diff("just some random text\nwith multiple lines"));
    }

    #[test]
    fn parse_diff_extracts_file_info() {
        let meta = parse_diff(SIMPLE_DIFF).unwrap();

        assert_eq!(meta.files.len(), 1);
        assert_eq!(meta.files[0].old_name, Some("file.rs".to_string()));
        assert_eq!(meta.files[0].new_name, Some("file.rs".to_string()));
        assert_eq!(meta.files[0].language, "rs");
    }

    #[test]
    fn parse_diff_handles_new_file() {
        let meta = parse_diff(NEW_FILE_DIFF).unwrap();

        assert_eq!(meta.files.len(), 1);
        assert_eq!(meta.files[0].old_name, None);
        assert_eq!(meta.files[0].new_name, Some("added_file".to_string()));
    }

    #[test]
    fn parse_diff_tracks_line_types() {
        let meta = parse_diff(SIMPLE_DIFF).unwrap();

        // Find the lines by type
        let added_lines: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.kind == DiffLineKind::Added)
            .collect();
        let deleted_lines: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.kind == DiffLineKind::Deleted)
            .collect();
        let context_lines: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.kind == DiffLineKind::Context)
            .collect();

        assert_eq!(deleted_lines.len(), 1, "Should have 1 deleted line");
        assert_eq!(added_lines.len(), 2, "Should have 2 added lines");
        assert_eq!(context_lines.len(), 2, "Should have 2 context lines");
    }

    #[test]
    fn parse_diff_tracks_line_numbers() {
        let meta = parse_diff(SIMPLE_DIFF).unwrap();

        // Find the deleted line
        let deleted = meta
            .lines
            .iter()
            .find(|(_, info)| info.kind == DiffLineKind::Deleted)
            .map(|(_, info)| info)
            .unwrap();

        assert_eq!(deleted.old_line_num, Some(2), "Deleted line should have old line num");
        assert_eq!(deleted.new_line_num, None, "Deleted line should not have new line num");

        // Find an added line
        let added = meta
            .lines
            .iter()
            .find(|(_, info)| info.kind == DiffLineKind::Added)
            .map(|(_, info)| info)
            .unwrap();

        assert_eq!(added.old_line_num, None, "Added line should not have old line num");
        assert!(added.new_line_num.is_some(), "Added line should have new line num");
    }

    #[test]
    fn parse_diff_context_has_both_line_nums() {
        let meta = parse_diff(SIMPLE_DIFF).unwrap();

        let context = meta
            .lines
            .iter()
            .find(|(_, info)| info.kind == DiffLineKind::Context)
            .map(|(_, info)| info)
            .unwrap();

        assert!(context.old_line_num.is_some(), "Context should have old line num");
        assert!(context.new_line_num.is_some(), "Context should have new line num");
    }

    const MULTI_FILE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
 fn main() {
-    println!("old");
+    println!("new");
 }
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,2 +1,3 @@
 pub fn hello() {
+    // added comment
 }
"#;

    const DELETED_FILE_DIFF: &str = r#"diff --git a/old_file.rs b/old_file.rs
deleted file mode 100644
index abcdef..0000000
--- a/old_file.rs
+++ /dev/null
@@ -1,3 +0,0 @@
-fn deprecated() {
-    // removed
-}
"#;

    const MULTIPLE_HUNKS_DIFF: &str = r#"diff --git a/big_file.rs b/big_file.rs
--- a/big_file.rs
+++ b/big_file.rs
@@ -1,3 +1,3 @@
 fn first() {
-    old1();
+    new1();
 }
@@ -10,3 +10,3 @@
 fn second() {
-    old2();
+    new2();
 }
"#;

    #[test]
    fn parse_multi_file_diff() {
        let meta = parse_diff(MULTI_FILE_DIFF).unwrap();

        assert_eq!(meta.files.len(), 2, "Should have 2 files");
        assert_eq!(meta.files[0].new_name, Some("src/main.rs".to_string()));
        assert_eq!(meta.files[0].language, "rs");
        assert_eq!(meta.files[1].new_name, Some("src/lib.rs".to_string()));
        assert_eq!(meta.files[1].language, "rs");
    }

    #[test]
    fn parse_multi_file_diff_tracks_file_index() {
        let meta = parse_diff(MULTI_FILE_DIFF).unwrap();

        // Lines from file 0 should have file_index 0
        let file0_lines: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.file_index == 0)
            .collect();
        assert!(!file0_lines.is_empty(), "Should have lines for file 0");

        // Lines from file 1 should have file_index 1
        let file1_lines: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.file_index == 1)
            .collect();
        assert!(!file1_lines.is_empty(), "Should have lines for file 1");
    }

    #[test]
    fn parse_deleted_file_diff() {
        let meta = parse_diff(DELETED_FILE_DIFF).unwrap();

        assert_eq!(meta.files.len(), 1);
        assert_eq!(meta.files[0].old_name, Some("old_file.rs".to_string()));
        assert_eq!(meta.files[0].new_name, None, "Deleted file has no new name");

        // All content lines should be deleted
        let deleted: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.kind == DiffLineKind::Deleted)
            .collect();
        assert_eq!(deleted.len(), 3, "Should have 3 deleted lines");
    }

    #[test]
    fn parse_multiple_hunks() {
        let meta = parse_diff(MULTIPLE_HUNKS_DIFF).unwrap();

        assert_eq!(meta.files.len(), 1);

        // Should have 5 header lines: diff --git, ---, +++, @@ (hunk1), @@ (hunk2)
        let headers: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.kind == DiffLineKind::Header)
            .collect();
        assert_eq!(headers.len(), 5, "Should have 5 header lines (3 file headers + 2 hunk headers)");

        // First hunk changes line 2, second hunk changes line 11
        let deleted: Vec<_> = meta
            .lines
            .iter()
            .filter(|(_, info)| info.kind == DiffLineKind::Deleted)
            .collect();
        assert_eq!(deleted.len(), 2, "Should have 2 deleted lines");

        // Check line numbers span different ranges
        let old_line_nums: Vec<u32> = deleted
            .iter()
            .filter_map(|(_, info)| info.old_line_num)
            .collect();
        assert!(old_line_nums.contains(&2), "Should have deletion at line 2");
        assert!(old_line_nums.contains(&11), "Should have deletion at line 11");
    }

    #[test]
    fn parse_diff_file_line_ranges() {
        let meta = parse_diff(MULTI_FILE_DIFF).unwrap();

        // File ranges should be contiguous and non-overlapping
        assert!(meta.files[0].start_line < meta.files[0].end_line);
        assert!(meta.files[0].end_line < meta.files[1].start_line);
        assert!(meta.files[1].start_line < meta.files[1].end_line);
    }

    #[test]
    fn parse_diff_display_line_numbers_are_sequential() {
        let meta = parse_diff(SIMPLE_DIFF).unwrap();

        // All display line numbers should be present from 1 to max
        let max_line = *meta.lines.keys().max().unwrap();
        for i in 1..=max_line {
            assert!(
                meta.lines.contains_key(&i),
                "Missing display line {}",
                i
            );
        }
    }

    #[test]
    fn parse_diff_error_on_invalid_content() {
        let result = parse_diff("not a diff at all");
        assert!(result.is_err());
    }

    #[test]
    fn diff_line_kind_serializes_lowercase() {
        // Verify serde serialization for frontend
        let json = serde_json::to_string(&DiffLineKind::Added).unwrap();
        assert_eq!(json, "\"added\"");

        let json = serde_json::to_string(&DiffLineKind::Deleted).unwrap();
        assert_eq!(json, "\"deleted\"");

        let json = serde_json::to_string(&DiffLineKind::Context).unwrap();
        assert_eq!(json, "\"context\"");

        let json = serde_json::to_string(&DiffLineKind::Header).unwrap();
        assert_eq!(json, "\"header\"");
    }
}
