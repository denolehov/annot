//! Unified diff parsing and detection.
//!
//! Parses raw `diff_content` patches into a per-file model natively via
//! `diffy`'s git-aware parser (`FileOperation`/`FileMode`/`PatchKind`/`Hunk`),
//! then `flatten`s to the legacy wire shape (`Vec<Line>` + `DiffMetadata`) so
//! no downstream consumer changes. Deliberately not emitted, matching what
//! the frontend never renders and what `pipeline.rs`'s git-native path
//! already omits: `index`/`---`/`+++`/mode/rename/similarity plumbing rows.

use diffy::patch_set::{FileMode, FileOperation, ParseOptions, PatchKind, PatchSet};
use serde::Serialize;

use crate::error::AnnotError;
use crate::highlight::Highlighter;
use crate::state::{DiffSemantics, Line, LineHtml, LineOrigin, LineSemantics};
use crate::vcs::FileStatus;

/// Metadata for a hunk within a file.
#[derive(Clone, Debug, Serialize)]
pub struct HunkInfo {
    /// Display line number of the @@ header (1-indexed).
    pub display_line: u32,
    /// Starting line in old file.
    pub old_start: u32,
    /// Number of lines from old file.
    pub old_count: u32,
    /// Starting line in new file.
    pub new_start: u32,
    /// Number of lines in new file.
    pub new_count: u32,
    /// Function/context from hunk header (e.g., "fn process()").
    pub function_context: Option<String>,
    /// Syntax-highlighted HTML of function context.
    pub function_context_html: Option<String>,
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
    /// Hunks within this file, ordered by display line.
    pub hunks: Vec<HunkInfo>,
}

/// Parsed diff metadata for rendering.
#[derive(Clone, Debug, Serialize)]
pub struct DiffMetadata {
    pub files: Vec<DiffFileInfo>,
}

/// One line of hunk content, already `+`/`-`/` `-prefixed — kind is
/// derivable from which of `old_line`/`new_line` is populated (old-only =
/// deleted, new-only = added, both = context).
struct RawDiffRow {
    old_line: Option<u32>,
    new_line: Option<u32>,
    content: String,
    html: Option<String>,
}

/// One hunk in the per-file internal model, pre-flatten (no `display_line`
/// yet — that's a flatten-time concept, assigned as rows land in the
/// flattened stream).
struct RawHunk {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    function_context: Option<String>,
    function_context_html: Option<String>,
    rows: Vec<RawDiffRow>,
}

/// One file's parsed patch, in per-file/per-row form — the internal model
/// `flatten` reproduces the legacy wire shape from.
pub(crate) struct RawDiffFile {
    old_path: Option<String>,
    new_path: Option<String>,
    // Not read by `flatten` — the legacy wire shape has no status field.
    // Kept for C1, which exposes it directly in the new per-file wire model.
    #[allow(dead_code)]
    status: FileStatus,
    unavailable: bool,
    language: String,
    hunks: Vec<RawHunk>,
}

/// Language identifier for a diff file: the extension of the new name (old
/// for deleted files). Feeds syntax highlighting for both the patch parser
/// and the git pipeline.
pub fn language_for(new_name: Option<&str>, old_name: Option<&str>) -> String {
    new_name
        .or(old_name)
        .and_then(|name| std::path::Path::new(name).extension()?.to_str())
        .map(String::from)
        .unwrap_or_default()
}

/// Check if content appears to be a unified diff.
pub fn is_diff(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }
    PatchSet::parse(content, ParseOptions::gitdiff())
        .collect::<Result<Vec<_>, _>>()
        .is_ok_and(|files| !files.is_empty())
}

/// Parse unified diff content into a per-file model.
pub(crate) fn parse_diff(
    content: &str,
    highlighter: &Highlighter,
) -> Result<Vec<RawDiffFile>, AnnotError> {
    let files = PatchSet::parse(content, ParseOptions::gitdiff())
        .map(|result| {
            result
                .map_err(|e| AnnotError::Diff(format!("Failed to parse diff: {e}")))
                .map(|file_patch| build_file(&file_patch, highlighter))
        })
        .collect::<Result<Vec<_>, AnnotError>>()?;

    if files.is_empty() {
        return Err(AnnotError::Diff("Not a valid diff".into()));
    }

    Ok(files)
}

fn build_file(fp: &diffy::patch_set::FilePatch<'_, str>, highlighter: &Highlighter) -> RawDiffFile {
    let (old_path, new_path, status) =
        resolve_operation(fp.operation(), fp.old_mode(), fp.new_mode());
    let language = language_for(new_path.as_deref(), old_path.as_deref());
    let unavailable = fp.patch().is_binary();

    let hunks = match fp.patch() {
        PatchKind::Text(patch) => {
            let fake_path = format!("file.{language}");
            patch
                .hunks()
                .iter()
                .map(|hunk| build_hunk(hunk, &language, highlighter, &fake_path))
                .collect()
        }
        PatchKind::Binary(_) => Vec::new(),
    };

    RawDiffFile {
        old_path,
        new_path,
        status,
        unavailable,
        language,
        hunks,
    }
}

/// Map a diffy `FileOperation` (+ mode headers) to display paths and
/// `FileStatus`. `Delete`/`Create`/`Modify` paths carry an `a/`/`b/` prefix
/// from the `---`/`+++` headers (or the bare `diff --git` line for
/// hunk-less creations) and need `strip_prefix(1)`; `Rename`/`Copy` paths
/// come from `rename from`/`rename to`/`copy from`/`copy to` headers, which
/// git never prefixes.
fn resolve_operation(
    op: &FileOperation<'_, str>,
    old_mode: Option<&FileMode>,
    new_mode: Option<&FileMode>,
) -> (Option<String>, Option<String>, FileStatus) {
    match op {
        FileOperation::Rename { from, to } => (
            Some(from.to_string()),
            Some(to.to_string()),
            // diffy discards the real `similarity index NN%` value (only
            // recognizes the line to skip past it); nothing downstream
            // reads this field today, so a default costs nothing.
            FileStatus::Renamed { similarity: 100 },
        ),
        FileOperation::Copy { from, to } => {
            (Some(from.to_string()), Some(to.to_string()), FileStatus::Copied)
        }
        FileOperation::Delete(_) | FileOperation::Create(_) | FileOperation::Modify { .. } => {
            match op.strip_prefix(1) {
                FileOperation::Delete(path) => (Some(path.into_owned()), None, FileStatus::Deleted),
                FileOperation::Create(path) => (None, Some(path.into_owned()), FileStatus::Added),
                FileOperation::Modify { original, modified } => {
                    let old_path = original.into_owned();
                    let new_path = modified.into_owned();
                    let status = if old_path != new_path {
                        // Paths differ without explicit rename headers —
                        // shouldn't happen from real git output, but if it
                        // does, treat as a rename (diffy's own suggested
                        // fallback), similarity unknown.
                        FileStatus::Renamed { similarity: 100 }
                    } else if old_mode.is_some() && new_mode.is_some() && old_mode != new_mode {
                        FileStatus::TypeChanged
                    } else {
                        FileStatus::Modified
                    };
                    (Some(old_path), Some(new_path), status)
                }
                _ => unreachable!("strip_prefix preserves the operation's variant"),
            }
        }
    }
}

fn build_hunk(
    hunk: &diffy::Hunk<'_, str>,
    language: &str,
    highlighter: &Highlighter,
    fake_path: &str,
) -> RawHunk {
    let old_range = hunk.old_range();
    let new_range = hunk.new_range();
    let function_context = hunk.function_context().map(str::to_owned);
    let function_context_html = function_context
        .as_deref()
        .and_then(|ctx| highlighter.highlight_function_context(ctx, fake_path));

    let mut old_line = old_range.start() as u32;
    let mut new_line = new_range.start() as u32;

    let rows = hunk
        .lines()
        .iter()
        .map(|line| build_row(line, &mut old_line, &mut new_line, language, highlighter, fake_path))
        .collect();

    RawHunk {
        old_start: old_range.start() as u32,
        old_count: old_range.len() as u32,
        new_start: new_range.start() as u32,
        new_count: new_range.len() as u32,
        function_context,
        function_context_html,
        rows,
    }
}

fn build_row(
    line: &diffy::Line<'_, str>,
    old_line: &mut u32,
    new_line: &mut u32,
    language: &str,
    highlighter: &Highlighter,
    fake_path: &str,
) -> RawDiffRow {
    let (raw, old, new, prefix) = match *line {
        diffy::Line::Context(text) => {
            let old = *old_line;
            let new = *new_line;
            *old_line += 1;
            *new_line += 1;
            (text, Some(old), Some(new), " ")
        }
        diffy::Line::Delete(text) => {
            let old = *old_line;
            *old_line += 1;
            (text, Some(old), None, "-")
        }
        diffy::Line::Insert(text) => {
            let new = *new_line;
            *new_line += 1;
            (text, None, Some(new), "+")
        }
    };

    // diffy's `Line` retains the trailing `\n` unless it's the file's final
    // line without one — our internal model, like every other line-based
    // representation in this codebase, is newline-free.
    let code = raw.trim_end_matches('\n');
    let content = format!("{prefix}{code}");

    let html = (!language.is_empty())
        .then(|| highlighter.highlight_diff_row(prefix, code, fake_path))
        .flatten();

    RawDiffRow {
        old_line: old,
        new_line: new,
        content,
        html,
    }
}

/// Flatten the per-file model into the legacy wire shape: a flat `Vec<Line>`
/// plus `DiffMetadata`. Pure — no highlighting or parsing here, only
/// reshaping already-built data. Mirrors `pipeline::render`'s output shape
/// so C1 can expose either producer's rows without writing new parse logic.
pub(crate) fn flatten(files: Vec<RawDiffFile>) -> (Vec<Line>, DiffMetadata) {
    let mut lines = Vec::new();
    let files = files
        .into_iter()
        .map(|file| flatten_file(file, &mut lines))
        .collect();
    (lines, DiffMetadata { files })
}

fn flatten_file(file: RawDiffFile, lines: &mut Vec<Line>) -> DiffFileInfo {
    let display_path = file
        .new_path
        .clone()
        .or_else(|| file.old_path.clone())
        .unwrap_or_default();
    let start_line = lines.len() as u32 + 1;

    let header_origin = |path: &str| LineOrigin::Diff {
        path: path.to_string(),
        old_line: None,
        new_line: None,
    };

    lines.push(Line {
        content: format!(
            "diff --git a/{} b/{}",
            file.old_path.as_deref().unwrap_or("/dev/null"),
            file.new_path.as_deref().unwrap_or("/dev/null"),
        ),
        html: None,
        origin: header_origin(&display_path),
        semantics: LineSemantics::Diff(DiffSemantics::FileHeader),
    });

    if file.unavailable {
        lines.push(Line {
            content: format!(
                "Binary files {} and {} differ",
                file.old_path
                    .as_deref()
                    .map(|p| format!("a/{p}"))
                    .unwrap_or_else(|| "/dev/null".into()),
                file.new_path
                    .as_deref()
                    .map(|p| format!("b/{p}"))
                    .unwrap_or_else(|| "/dev/null".into()),
            ),
            html: None,
            origin: header_origin(&display_path),
            semantics: LineSemantics::Diff(DiffSemantics::Meta),
        });
    }

    let hunks = file
        .hunks
        .into_iter()
        .map(|hunk| flatten_hunk(hunk, &display_path, lines))
        .collect();

    DiffFileInfo {
        old_name: file.old_path,
        new_name: file.new_path,
        language: file.language,
        start_line,
        end_line: lines.len() as u32,
        hunks,
    }
}

fn flatten_hunk(hunk: RawHunk, display_path: &str, lines: &mut Vec<Line>) -> HunkInfo {
    let marker = format!(
        "@@ {} {} @@",
        crate::pipeline::printed_side('-', hunk.old_start, hunk.old_count),
        crate::pipeline::printed_side('+', hunk.new_start, hunk.new_count),
    );
    let header = match hunk.function_context.as_deref() {
        Some(ctx) => format!("{marker} {ctx}"),
        None => marker,
    };

    lines.push(Line {
        content: header,
        html: None,
        origin: LineOrigin::Diff {
            path: display_path.to_string(),
            old_line: None,
            new_line: None,
        },
        semantics: LineSemantics::Diff(DiffSemantics::HunkHeader {
            context: hunk.function_context.clone(),
        }),
    });
    let display_line = lines.len() as u32;

    for row in hunk.rows {
        let semantics = match (row.old_line, row.new_line) {
            (Some(_), Some(_)) => DiffSemantics::Context,
            (Some(_), None) => DiffSemantics::Deleted,
            (None, Some(_)) => DiffSemantics::Added,
            (None, None) => unreachable!("a row always belongs to at least one side"),
        };
        lines.push(Line {
            content: row.content,
            html: row.html.map(LineHtml::Full),
            origin: LineOrigin::Diff {
                path: display_path.to_string(),
                old_line: row.old_line,
                new_line: row.new_line,
            },
            semantics: LineSemantics::Diff(semantics),
        });
    }

    HunkInfo {
        display_line,
        old_start: hunk.old_start,
        old_count: hunk.old_count,
        new_start: hunk.new_start,
        new_count: hunk.new_count,
        function_context: hunk.function_context,
        function_context_html: hunk.function_context_html,
    }
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

    const PURE_RENAME_DIFF: &str = r#"diff --git a/old.rs b/new.rs
similarity index 100%
rename from old.rs
rename to new.rs
"#;

    const NO_NEWLINE_DIFF: &str = r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,2 +1,2 @@
 fn main() {
-}
\ No newline at end of file
+}
"#;

    fn parse(content: &str) -> Vec<RawDiffFile> {
        parse_diff(content, &Highlighter::new()).unwrap()
    }

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
    fn parse_diff_extracts_file_info() {
        let files = parse(SIMPLE_DIFF);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].old_path.as_deref(), Some("file.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("file.rs"));
        assert_eq!(files[0].language, "rs");
        assert_eq!(files[0].status, FileStatus::Modified);
    }

    #[test]
    fn parse_diff_handles_new_file() {
        let files = parse(NEW_FILE_DIFF);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].old_path, None);
        assert_eq!(files[0].new_path.as_deref(), Some("added_file"));
        assert_eq!(files[0].status, FileStatus::Added);
    }

    #[test]
    fn parse_deleted_file_diff() {
        let files = parse(DELETED_FILE_DIFF);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].old_path.as_deref(), Some("old_file.rs"));
        assert_eq!(files[0].new_path, None);
        assert_eq!(files[0].status, FileStatus::Deleted);
    }

    #[test]
    fn parse_multi_file_diff() {
        let files = parse(MULTI_FILE_DIFF);
        assert_eq!(files.len(), 2, "Should have 2 files");
        assert_eq!(files[0].new_path.as_deref(), Some("src/main.rs"));
        assert_eq!(files[1].new_path.as_deref(), Some("src/lib.rs"));
    }

    #[test]
    fn parse_multiple_hunks() {
        let files = parse(MULTIPLE_HUNKS_DIFF);
        assert_eq!(files[0].hunks.len(), 2, "Should have 2 hunks");
        assert_eq!(files[0].hunks[0].old_start, 1);
        assert_eq!(files[0].hunks[1].old_start, 10);
    }

    #[test]
    fn parse_diff_tracks_line_numbers() {
        let files = parse(SIMPLE_DIFF);
        let rows = &files[0].hunks[0].rows;

        let deleted = rows.iter().find(|r| r.new_line.is_none()).unwrap();
        assert_eq!(deleted.old_line, Some(2));

        let added = rows.iter().find(|r| r.old_line.is_none()).unwrap();
        assert!(added.new_line.is_some());
    }

    #[test]
    fn pure_rename_is_not_dropped_and_does_not_misindex_following_files() {
        let combined = format!(
            "{PURE_RENAME_DIFF}diff --git a/foo b/foo\n--- a/foo\n+++ b/foo\n@@ -1 +1 @@\n-old\n+new\n"
        );
        let files = parse(&combined);
        assert_eq!(files.len(), 2, "pure rename must not be dropped");
        assert_eq!(files[0].old_path.as_deref(), Some("old.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new.rs"));
        assert_eq!(files[0].status, FileStatus::Renamed { similarity: 100 });
        assert!(files[0].hunks.is_empty());

        // The second file must resolve correctly — not misindexed.
        assert_eq!(files[1].old_path.as_deref(), Some("foo"));
        assert_eq!(files[1].hunks.len(), 1);
    }

    #[test]
    fn no_newline_marker_does_not_shift_line_numbers() {
        let files = parse(NO_NEWLINE_DIFF);
        let rows = &files[0].hunks[0].rows;

        // Exactly one deleted and one added row — the no-newline marker is
        // not a phantom context row.
        assert_eq!(rows.iter().filter(|r| r.new_line.is_none()).count(), 1);
        assert_eq!(rows.iter().filter(|r| r.old_line.is_none()).count(), 1);

        let added = rows.iter().find(|r| r.old_line.is_none()).unwrap();
        assert_eq!(added.new_line, Some(2));
    }

    /// `pnpm demo:diff` opens this exact file — a native GUI window this
    /// suite can't drive, so this is the closest automatable stand-in:
    /// the same fixture parsed and flattened end-to-end.
    #[test]
    fn parses_the_demo_diff_fixture_end_to_end() {
        let sample = include_str!("../../test-fixtures/sample.diff");
        let files = parse(sample);
        assert_eq!(files.len(), 11, "sample.diff has 11 changed files");
        assert!(files.iter().all(|f| f.status == FileStatus::Modified));

        let (lines, metadata) = flatten(files);
        assert!(!lines.is_empty());
        assert_eq!(metadata.files.len(), 11);
    }

    #[test]
    fn flatten_reproduces_prefixed_content() {
        let files = parse(SIMPLE_DIFF);
        let (lines, metadata) = flatten(files);

        assert!(lines[0].content.starts_with("diff --git"));
        assert!(lines.iter().any(|l| l.content.starts_with('+')));
        assert!(lines.iter().any(|l| l.content.starts_with('-')));
        assert_eq!(metadata.files.len(), 1);
        assert_eq!(metadata.files[0].start_line, 1);
        assert_eq!(metadata.files[0].end_line, lines.len() as u32);
    }
}
