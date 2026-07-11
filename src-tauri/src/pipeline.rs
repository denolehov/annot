//! Git-mode render pipeline: enumerated files + full texts + computed hunks
//! → the same patch-shaped `Line` stream the legacy parser produces.
//!
//! The strangler seam for git mode: `diff.rs` keeps parsing raw
//! `diff_content` patches; this module synthesizes the identical wire shape
//! (`LineOrigin::Diff`, `DiffSemantics`, prefixed `content`) from structure,
//! so no consumer changes. Deliberately not emitted, matching what the
//! frontend never renders: `index`/`---`/`+++`/mode plumbing rows and
//! `\ No newline at end of file` markers.

use std::collections::HashMap;
use std::ops::Range;

use crate::diff::{DiffFileInfo, DiffMetadata, HunkInfo};
use crate::engine::{compute_hunks, DiffRow};
use crate::error::AnnotError;
use crate::highlight::Highlighter;
use crate::source::{FileSource, Side};
use crate::state::{DiffSemantics, Line, LineHtml, LineOrigin, LineSemantics};
use crate::vcs::{BlobRef, FileEntry};

/// Lines of context around changes — git's default; unfold is the
/// mechanism for seeing more, not a wider default.
pub const CONTEXT_LINES: u32 = 3;

/// Hunk-header function context is capped like git's (80 bytes).
const FUNCTION_CONTEXT_MAX_BYTES: usize = 80;

/// Build `GixSource`'s oid map from enumerated entries. Each entry yields up
/// to two insertions keyed by the side-appropriate path (renames differ per
/// side); a `None` oid means the side doesn't exist and gets no key.
pub fn build_oid_map(entries: &[FileEntry]) -> HashMap<(String, Side), BlobRef> {
    entries
        .iter()
        .flat_map(|entry| {
            let old = entry
                .old_path
                .clone()
                .zip(entry.old_oid.clone())
                .map(|(path, oid)| ((path, Side::Old), BlobRef::Oid(oid)));
            let new = entry
                .new_path
                .clone()
                .zip(entry.new_oid.clone())
                .map(|(path, blob)| ((path, Side::New), blob));
            old.into_iter().chain(new)
        })
        .collect()
}

/// Render enumerated files into the flat diff line stream + metadata.
pub fn render(
    entries: &[FileEntry],
    source: &dyn FileSource,
    highlighter: &Highlighter,
    context: u32,
) -> Result<(Vec<Line>, DiffMetadata), AnnotError> {
    let mut lines = Vec::new();
    let files = entries
        .iter()
        .map(|entry| render_file(entry, source, highlighter, context, &mut lines))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((lines, DiffMetadata { files }))
}

fn render_file(
    entry: &FileEntry,
    source: &dyn FileSource,
    highlighter: &Highlighter,
    context: u32,
    lines: &mut Vec<Line>,
) -> Result<DiffFileInfo, AnnotError> {
    // Display identity: new name wins (old for deleted files) — mirrors
    // parse_diff. `a/` side is the old name, `b/` side the new.
    let a_path = entry.old_path.as_ref().or(entry.new_path.as_ref());
    let b_path = entry.new_path.as_ref().or(entry.old_path.as_ref());
    let display_path = b_path.cloned().unwrap_or_default();
    let language = crate::diff::language_for(entry.new_path.as_deref(), entry.old_path.as_deref());

    let start_line = lines.len() as u32 + 1;
    let header_origin = || LineOrigin::Diff {
        path: display_path.clone(),
        old_line: None,
        new_line: None,
    };
    lines.push(Line {
        content: format!(
            "diff --git a/{} b/{}",
            a_path.map(String::as_str).unwrap_or("/dev/null"),
            b_path.map(String::as_str).unwrap_or("/dev/null"),
        ),
        html: None,
        origin: header_origin(),
        semantics: LineSemantics::Diff(DiffSemantics::FileHeader),
    });

    // A side the entry says exists but yields no text is binary/oversize/
    // non-UTF-8 (`Ok(None)` capability signal); a nonexistent side diffs
    // against the empty string.
    let old_text = fetch(source, entry, Side::Old)?;
    let new_text = fetch(source, entry, Side::New)?;
    let unavailable = (entry.old_oid.is_some() && old_text.is_none())
        || (entry.new_oid.is_some() && new_text.is_none());

    let mut hunks = Vec::new();
    if unavailable {
        lines.push(Line {
            content: format!(
                "Binary files {} and {} differ",
                a_path
                    .map(|p| format!("a/{p}"))
                    .unwrap_or_else(|| "/dev/null".into()),
                b_path
                    .map(|p| format!("b/{p}"))
                    .unwrap_or_else(|| "/dev/null".into()),
            ),
            html: None,
            origin: header_origin(),
            semantics: LineSemantics::Diff(DiffSemantics::Meta),
        });
    } else {
        let old = old_text.as_deref().unwrap_or("");
        let new = new_text.as_deref().unwrap_or("");
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let fake_path = format!("file.{language}");

        for hunk in compute_hunks(old, new, context).hunks {
            let (old_start, old_count) = printed_range(&hunk.old_range);
            let (new_start, new_count) = printed_range(&hunk.new_range);
            let function_context = function_context(&old_lines, hunk.old_range.start);
            let marker = format!(
                "@@ {} {} @@",
                printed_side('-', old_start, old_count),
                printed_side('+', new_start, new_count)
            );
            let header = match function_context.as_deref() {
                Some(ctx) => format!("{marker} {ctx}"),
                None => marker,
            };

            lines.push(Line {
                content: header,
                html: None,
                origin: header_origin(),
                semantics: LineSemantics::Diff(DiffSemantics::HunkHeader {
                    context: function_context.clone(),
                }),
            });
            hunks.push(HunkInfo {
                display_line: lines.len() as u32,
                old_start,
                old_count,
                new_start,
                new_count,
                function_context_html: function_context
                    .as_deref()
                    .and_then(|ctx| highlighter.highlight_function_context(ctx, &fake_path)),
                function_context,
            });

            for row in &hunk.rows {
                lines.push(render_row(
                    row,
                    &old_lines,
                    &new_lines,
                    &display_path,
                    &language,
                    &fake_path,
                    highlighter,
                ));
            }
        }
    }

    Ok(DiffFileInfo {
        old_name: entry.old_path.clone(),
        new_name: entry.new_path.clone(),
        language,
        start_line,
        end_line: lines.len() as u32,
        hunks,
    })
}

fn fetch(
    source: &dyn FileSource,
    entry: &FileEntry,
    side: Side,
) -> Result<Option<std::sync::Arc<str>>, AnnotError> {
    let (path, exists) = match side {
        Side::Old => (entry.old_path.as_deref(), entry.old_oid.is_some()),
        Side::New => (entry.new_path.as_deref(), entry.new_oid.is_some()),
    };
    match path {
        Some(p) if exists => source.full_text(p, side),
        _ => Ok(None),
    }
}

fn render_row(
    row: &DiffRow,
    old_lines: &[&str],
    new_lines: &[&str],
    display_path: &str,
    language: &str,
    fake_path: &str,
    highlighter: &Highlighter,
) -> Line {
    // `word_ranges` deliberately dropped: no wire field exists yet;
    // word-level highlights recompute from the session's retained FileSource.
    let (prefix, text, old_line, new_line, semantics) = match *row {
        DiffRow::Context { old_line, new_line } => (
            " ",
            line_at(old_lines, old_line),
            Some(old_line),
            Some(new_line),
            DiffSemantics::Context,
        ),
        DiffRow::Deleted { old_line, .. } => (
            "-",
            line_at(old_lines, old_line),
            Some(old_line),
            None,
            DiffSemantics::Deleted,
        ),
        DiffRow::Added { new_line, .. } => (
            "+",
            line_at(new_lines, new_line),
            None,
            Some(new_line),
            DiffSemantics::Added,
        ),
    };

    let html = (!language.is_empty())
        .then(|| highlighter.highlight_diff_row(prefix, text, fake_path))
        .flatten();

    Line {
        content: format!("{prefix}{text}"),
        html: html.map(LineHtml::Full),
        origin: LineOrigin::Diff {
            path: display_path.to_string(),
            old_line,
            new_line,
        },
        semantics: LineSemantics::Diff(semantics),
    }
}

/// 1-indexed line from a pre-split side, empty for out-of-range (defensive:
/// engine row numbers are always in range).
fn line_at<'a>(lines: &[&'a str], number: u32) -> &'a str {
    lines
        .get(number.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("")
}

/// Half-open 1-indexed engine range → the numbers git prints in `@@` headers:
/// count is the length; an empty range prints the line *before* the position
/// (`@@ -0,0 +1,3 @@` for a new file).
fn printed_range(range: &Range<u32>) -> (u32, u32) {
    let count = range.end - range.start;
    let start = if count == 0 {
        range.start.saturating_sub(1)
    } else {
        range.start
    };
    (start, count)
}

/// Git omits the count when it is 1: `-3` not `-3,1`.
/// GNU-diff-style range formatting: omit the count when it's 1. Shared by
/// the git pipeline and the patch parser.
pub(crate) fn printed_side(sign: char, start: u32, count: u32) -> String {
    if count == 1 {
        format!("{sign}{start}")
    } else {
        format!("{sign}{start},{count}")
    }
}

/// Git's default funcname rule (xdiff `def_ff`, used when no userdiff driver
/// matches): nearest line above the hunk — old side, like git — whose first
/// character is alphabetic, `_`, or `$`; capped at 80 bytes.
fn function_context(old_lines: &[&str], hunk_old_start: u32) -> Option<String> {
    let above = (hunk_old_start as usize)
        .saturating_sub(1)
        .min(old_lines.len());
    old_lines[..above]
        .iter()
        .rev()
        .find(|line| {
            line.chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        })
        .map(|line| {
            let end = (0..=FUNCTION_CONTEXT_MAX_BYTES.min(line.len()))
                .rev()
                .find(|&i| line.is_char_boundary(i))
                .unwrap_or(0);
            line[..end].trim_end().to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{CliSource, ContentSource};
    use crate::source::GixSource;
    use crate::state::{ContentMetadata, ContentModel};
    use crate::testutil::git;
    use crate::vcs::{enumerate, DiffTarget};
    use std::path::Path;

    fn render_repo(p: &Path, target: &DiffTarget) -> (Vec<Line>, DiffMetadata) {
        let entries = enumerate(p, target, &[]).unwrap();
        let source = GixSource::new(gix::discover(p).unwrap(), build_oid_map(&entries));
        render(&entries, &source, &Highlighter::new(), CONTEXT_LINES).unwrap()
    }

    /// Compact textual form of the line stream: origin numbers, semantics,
    /// raw content — the full wire-relevant surface except html.
    fn dump(lines: &[Line]) -> String {
        let num = |n: Option<u32>| n.map_or("·".to_string(), |n| n.to_string());
        lines
            .iter()
            .map(|l| {
                let (old, new) = match &l.origin {
                    LineOrigin::Diff {
                        old_line, new_line, ..
                    } => (*old_line, *new_line),
                    _ => (None, None),
                };
                let sem = match &l.semantics {
                    LineSemantics::Diff(d) => match d {
                        DiffSemantics::FileHeader => "file",
                        DiffSemantics::HunkHeader { .. } => "hunk",
                        DiffSemantics::Meta => "meta",
                        DiffSemantics::Added => "add",
                        DiffSemantics::Deleted => "del",
                        DiffSemantics::Context => "ctx",
                    },
                    _ => "?",
                };
                format!("{:>4} {:>4} {:>4} |{}", num(old), num(new), sem, l.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    const MAIN_RS_V1: &str = "fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 4;\n    let e = 5;\n    println!(\"{}\", a + b);\n}\n";
    const MAIN_RS_V2: &str = "fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 40;\n    let e = 5;\n    println!(\"{}\", a + b);\n}\n";

    /// Whether the committed fixture includes a pure rename. `LegacySafe`
    /// exists because the legacy parser cannot survive one: unidiff drops
    /// the hunk-less file, and `parse_diff`'s raw-line walk then misindexes
    /// every file after it — part of why this pipeline exists.
    enum Fixture {
        WithRename,
        LegacySafe,
    }

    /// Committed matrix: modified (with funcname), added, deleted, a
    /// no-trailing-newline change, and (unless `LegacySafe`) a pure rename.
    fn range_fixture(kind: Fixture) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        std::fs::write(p.join("main.rs"), MAIN_RS_V1).unwrap();
        std::fs::write(p.join("deleted.txt"), "doomed\ncontent\n").unwrap();
        std::fs::write(p.join("old_name.txt"), "renamed content\n").unwrap();
        std::fs::write(p.join("noeol.txt"), "alpha\nbeta").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "one"]);
        std::fs::write(p.join("main.rs"), MAIN_RS_V2).unwrap();
        std::fs::write(p.join("added.txt"), "fresh\nfile\n").unwrap();
        std::fs::write(p.join("noeol.txt"), "alpha\nbeta\n").unwrap();
        git(p, &["rm", "-q", "deleted.txt"]);
        if matches!(kind, Fixture::WithRename) {
            git(p, &["mv", "old_name.txt", "new_name.txt"]);
        }
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "two"]);
        dir
    }

    fn head_range() -> DiffTarget {
        DiffTarget::Range {
            from: "HEAD~1".into(),
            to: "HEAD".into(),
            merge_base: false,
        }
    }

    #[test]
    fn range_stream_snapshot() {
        let dir = range_fixture(Fixture::WithRename);
        let (lines, meta) = render_repo(dir.path(), &head_range());
        insta::assert_snapshot!(dump(&lines));

        // File spans and hunk display lines index into the stream correctly.
        for file in &meta.files {
            assert!(matches!(
                lines[file.start_line as usize - 1].semantics,
                LineSemantics::Diff(DiffSemantics::FileHeader)
            ));
            assert!(file.end_line as usize <= lines.len());
            for hunk in &file.hunks {
                assert!(matches!(
                    lines[hunk.display_line as usize - 1].semantics,
                    LineSemantics::Diff(DiffSemantics::HunkHeader { .. })
                ));
            }
        }
    }

    /// The strangler bar: the new pipeline's changed rows must be exactly the
    /// ones the legacy path (git CLI patch → parse_diff) produces.
    #[test]
    fn parity_with_legacy_parser_on_fixture() {
        let dir = range_fixture(Fixture::LegacySafe);
        let p = dir.path();
        let (new_lines, new_meta) = render_repo(p, &head_range());

        let patch = git(p, &["diff", "HEAD~1..HEAD"]);
        let cli_source = ContentSource::Cli(CliSource::Stdin {
            label: "diff".into(),
        });
        let legacy = ContentModel::from_diff(&patch, cli_source).unwrap();
        let ContentMetadata::Diff(legacy_meta) = &legacy.metadata else {
            panic!("legacy model is not a diff");
        };

        let names = |meta: &DiffMetadata| {
            meta.files
                .iter()
                .map(|f| (f.old_name.clone(), f.new_name.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&new_meta), names(legacy_meta));

        // Changed rows: identical content, origins, and semantics. Context
        // rows are excluded — hunk boundaries may differ cosmetically between
        // engines (accepted at design time). noeol.txt is excluded from the
        // side-by-side because the legacy parser miscounts there: it treats
        // the `\ No newline at end of file` marker as a context line, shifting
        // every following new-side number by one (asserted correct below).
        let changed = |lines: &[Line]| {
            lines
                .iter()
                .filter(|l| {
                    matches!(
                        l.semantics,
                        LineSemantics::Diff(DiffSemantics::Added | DiffSemantics::Deleted)
                    )
                })
                .map(|l| {
                    let LineOrigin::Diff {
                        path,
                        old_line,
                        new_line,
                    } = &l.origin
                    else {
                        panic!("diff row without diff origin");
                    };
                    (path.clone(), *old_line, *new_line, l.content.clone())
                })
                .filter(|(path, ..)| path != "noeol.txt")
                .collect::<Vec<_>>()
        };
        assert_eq!(changed(&new_lines), changed(&legacy.lines));

        // The re-added `beta` really is line 2 of the new file — the number
        // the legacy parser gets wrong.
        assert!(new_lines.iter().any(|l| l.content == "+beta"
            && matches!(
                l.origin,
                LineOrigin::Diff {
                    new_line: Some(2),
                    ..
                }
            )));
    }

    #[test]
    fn working_tree_stream_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        std::fs::write(p.join("main.rs"), MAIN_RS_V1).unwrap();
        std::fs::write(p.join("bin.dat"), b"\x00\x01old").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "one"]);
        std::fs::write(p.join("main.rs"), MAIN_RS_V2).unwrap();
        std::fs::write(p.join("bin.dat"), b"\x00\x02new").unwrap();
        std::fs::write(p.join("untracked.txt"), "brand new\n").unwrap();

        let (lines, _) = render_repo(p, &DiffTarget::WorkingTree);
        insta::assert_snapshot!(dump(&lines));
    }

    #[test]
    fn rows_are_highlighted_headers_are_not() {
        let dir = range_fixture(Fixture::WithRename);
        let (lines, _) = render_repo(dir.path(), &head_range());

        let added_rs = lines
            .iter()
            .find(|l| {
                matches!(l.semantics, LineSemantics::Diff(DiffSemantics::Added))
                    && l.content.contains("let d = 40;")
            })
            .unwrap();
        match &added_rs.html {
            Some(LineHtml::Full(html)) => assert!(html.starts_with('+')),
            other => panic!("expected highlighted row, got {other:?}"),
        }

        let header = lines
            .iter()
            .find(|l| matches!(l.semantics, LineSemantics::Diff(DiffSemantics::FileHeader)))
            .unwrap();
        assert!(header.html.is_none());
    }

    #[test]
    fn empty_enumeration_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        std::fs::write(p.join("a.txt"), "x\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "one"]);

        let source = ContentSource::Cli(CliSource::Stdin {
            label: "diff".into(),
        });
        let Err(err) = ContentModel::from_git(p, &DiffTarget::WorkingTree, &[], source) else {
            panic!("expected an error for an empty enumeration");
        };
        assert!(err.to_string().contains("no changes"));
    }

    #[test]
    fn oid_map_keys_renames_by_side_appropriate_path() {
        let entry = FileEntry {
            status: crate::vcs::FileStatus::Renamed { similarity: 100 },
            old_path: Some("old.rs".into()),
            new_path: Some("new.rs".into()),
            old_oid: Some("aaaa".into()),
            new_oid: Some(BlobRef::WorkingTree),
        };
        let map = build_oid_map(&[entry]);
        assert_eq!(
            map.get(&("old.rs".into(), Side::Old)),
            Some(&BlobRef::Oid("aaaa".into()))
        );
        assert_eq!(
            map.get(&("new.rs".into(), Side::New)),
            Some(&BlobRef::WorkingTree)
        );
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn printed_range_matches_git_header_conventions() {
        assert_eq!(printed_range(&(1..4)), (1, 3)); // @@ -1,3
        assert_eq!(printed_range(&(3..4)), (3, 1)); // @@ -3 (count omitted)
        assert_eq!(printed_range(&(1..1)), (0, 0)); // new file: @@ -0,0
        assert_eq!(printed_range(&(6..6)), (5, 0)); // insertion after line 5
        assert_eq!(printed_side('-', 3, 1), "-3");
        assert_eq!(printed_side('+', 1, 3), "+1,3");
        assert_eq!(printed_side('-', 0, 0), "-0,0");
    }

    /// Manual corpus eyeball (the spec's verification bar): render the
    /// enclosing repository's working-tree diff through both pipelines and
    /// report row-level divergences plus the untracked files only the new
    /// pipeline can show.
    /// Run: `cargo test --lib side_by_side_on_this_repo -- --ignored --nocapture`
    #[test]
    #[ignore = "manual eyeball against the enclosing repo's working tree"]
    fn side_by_side_on_this_repo() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let (new_lines, _) = render_repo(repo_root, &DiffTarget::WorkingTree);

        let patch = git(repo_root, &["diff", "HEAD"]);
        if patch.is_empty() {
            println!("working tree clean — nothing to compare");
            return;
        }
        let legacy = ContentModel::from_diff(
            &patch,
            ContentSource::Cli(CliSource::Stdin {
                label: "diff".into(),
            }),
        )
        .unwrap();

        let changed = |lines: &[Line]| {
            lines
                .iter()
                .filter(|l| {
                    matches!(
                        l.semantics,
                        LineSemantics::Diff(DiffSemantics::Added | DiffSemantics::Deleted)
                    )
                })
                .map(|l| format!("{:?} {}", l.origin, l.content))
                .collect::<std::collections::BTreeSet<_>>()
        };
        let (ours, theirs) = (changed(&new_lines), changed(&legacy.lines));
        for row in ours.difference(&theirs) {
            println!("only new pipeline: {row}");
        }
        for row in theirs.difference(&ours) {
            println!("only legacy:       {row}");
        }
        println!(
            "{} changed rows in new pipeline, {} in legacy",
            ours.len(),
            theirs.len()
        );
    }

    #[test]
    fn function_context_follows_default_driver_rule() {
        let lines = ["fn outer() {", "    inner();", "}", "", "    indented"];
        // Nearest line above line 5 starting with [alpha_$]: skips the
        // indented line, the blank, and the closing brace.
        assert_eq!(
            function_context(&lines, 5),
            Some("fn outer() {".to_string())
        );
        // Nothing above the first line.
        assert_eq!(function_context(&lines, 1), None);
        // 80-byte cap lands on a char boundary.
        let long = format!("f{}", "é".repeat(60));
        let capped = function_context(&[&long], 2).unwrap();
        assert!(capped.len() <= FUNCTION_CONTEXT_MAX_BYTES);
        assert!(long.starts_with(&capped));
    }
}
