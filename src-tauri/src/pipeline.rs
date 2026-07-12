//! Git-mode render pipeline: enumerated files + full texts + computed hunks
//! → per-file `DiffDocument`s.
//!
//! Headers and `+`/`-` signs are presentation synthesized at the edges
//! (frontend walk, output emit) — this module produces only structure.
//! Plumbing (`index`/`---`/`+++`/mode lines, `\ No newline at end of file`
//! markers) has no representation at all.

use std::collections::HashMap;
use std::ops::Range;

use serde::Deserialize;

use crate::engine::{compute_hunks, DiffRow};
use crate::error::AnnotError;
use crate::highlight::Highlighter;
use crate::source::{FileSource, Side};
use crate::state::{DiffDocument, HunkV2, LineHtml, Row};
use crate::vcs::{BlobRef, FileEntry};

/// Lines of context around changes — git's default; unfold is the
/// mechanism for seeing more, not a wider default.
pub const CONTEXT_LINES: u32 = 3;

/// Context lines revealed per directional unfold click (GitHub's step).
pub const EXPAND_STEP: u32 = 20;

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

/// Render enumerated files into per-file diff documents.
pub fn render(
    entries: &[FileEntry],
    source: &dyn FileSource,
    highlighter: &Highlighter,
    context: u32,
) -> Result<Vec<DiffDocument>, AnnotError> {
    entries
        .iter()
        .map(|entry| render_file(entry, source, highlighter, context))
        .collect()
}

fn render_file(
    entry: &FileEntry,
    source: &dyn FileSource,
    highlighter: &Highlighter,
    context: u32,
) -> Result<DiffDocument, AnnotError> {
    let (display_path, old_path) =
        crate::diff::display_identity(entry.old_path.as_deref(), entry.new_path.as_deref());
    let language = crate::diff::language_for(entry.new_path.as_deref(), entry.old_path.as_deref());

    // A side the entry says exists but yields no text is binary/oversize/
    // non-UTF-8 (`Ok(None)` capability signal); a nonexistent side diffs
    // against the empty string.
    let old_text = fetch(source, entry, Side::Old)?;
    let new_text = fetch(source, entry, Side::New)?;
    let unavailable = (entry.old_oid.is_some() && old_text.is_none())
        || (entry.new_oid.is_some() && new_text.is_none());

    let mut hunks = Vec::new();
    if !unavailable {
        let old = old_text.as_deref().unwrap_or("");
        let new = new_text.as_deref().unwrap_or("");
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let fake_path = format!("file.{language}");

        for hunk in compute_hunks(old, new, context).hunks {
            let (old_start, old_count) = printed_range(&hunk.old_range);
            let (new_start, new_count) = printed_range(&hunk.new_range);
            let function_context = function_context(&old_lines, hunk.old_range.start);
            hunks.push(HunkV2 {
                old_range: old_start..old_start + old_count,
                new_range: new_start..new_start + new_count,
                function_context_html: function_context
                    .as_deref()
                    .and_then(|ctx| highlighter.highlight_function_context(ctx, &fake_path)),
                function_context,
                rows: hunk
                    .rows
                    .iter()
                    .map(|row| {
                        render_row(
                            row,
                            &old_lines,
                            &new_lines,
                            &language,
                            &fake_path,
                            highlighter,
                        )
                    })
                    .collect(),
            });
        }
    }

    // Free byproduct of the diff having read the full text: the unfold
    // capability signal and the trailing-gap bound.
    let new_len = (!unavailable)
        .then(|| new_text.as_deref().map(|t| t.lines().count() as u32))
        .flatten();

    Ok(DiffDocument {
        path: display_path,
        old_path,
        status: entry.status.clone(),
        unavailable,
        language,
        new_len,
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
    language: &str,
    fake_path: &str,
    highlighter: &Highlighter,
) -> Row {
    let (text, old_line, new_line, word_ranges) = match row {
        DiffRow::Context { old_line, new_line } => (
            line_at(old_lines, *old_line),
            Some(*old_line),
            Some(*new_line),
            &[][..],
        ),
        DiffRow::Deleted {
            old_line,
            word_ranges,
        } => (
            line_at(old_lines, *old_line),
            Some(*old_line),
            None,
            word_ranges.as_slice(),
        ),
        DiffRow::Added {
            new_line,
            word_ranges,
        } => (
            line_at(new_lines, *new_line),
            None,
            Some(*new_line),
            word_ranges.as_slice(),
        ),
    };

    let html = (!language.is_empty())
        .then(|| highlighter.highlight_diff_row(text, fake_path))
        .flatten();

    Row {
        old_line,
        new_line,
        content: text.to_string(),
        html: html.map(LineHtml::Full),
        word_ranges: utf16_ranges(text, word_ranges),
    }
}

/// Engine byte offsets → UTF-16 code-unit offsets (what the webview's DOM
/// APIs address). Engine ranges are always on char boundaries.
fn utf16_ranges(text: &str, byte_ranges: &[Range<usize>]) -> Vec<Range<u32>> {
    let utf16_at = |byte: usize| text[..byte].encode_utf16().count() as u32;
    byte_ranges
        .iter()
        .map(|r| utf16_at(r.start)..utf16_at(r.end))
        .collect()
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

// ════════════════════════════════════════════════════════════════════════════
// UNFOLD (S3) — grow a hunk's context, then restore the no-touching invariant
// ════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpandDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpandAmount {
    Step,
    All,
}

/// Stored ranges use git-printed convention: an empty side starts at the
/// line *before* the position. Expansion arithmetic needs the true half-open
/// range, where an empty side sits at its insertion point.
fn true_range(range: &Range<u32>) -> Range<u32> {
    if range.start == range.end {
        range.start + 1..range.start + 1
    } else {
        range.clone()
    }
}

/// Unfold context around `hunks[hunk_index]`: grow its ranges toward the
/// neighboring hunk (or the file edge), splice in context rows sliced from
/// the new-side full text, and merge hunks whose ranges now touch — Zed's
/// normalize-not-track rule: a fully unfolded gap disappears because the
/// no-touching invariant is restored, not because anyone tracked the gap.
pub fn expand_context(
    doc: &mut DiffDocument,
    source: &dyn FileSource,
    highlighter: &Highlighter,
    hunk_index: usize,
    direction: ExpandDirection,
    amount: ExpandAmount,
) -> Result<(), AnnotError> {
    let new_len = doc
        .new_len
        .ok_or_else(|| AnnotError::Diff("no full text available to unfold".into()))?;
    if hunk_index >= doc.hunks.len() {
        return Err(AnnotError::Diff(format!(
            "hunk index {hunk_index} out of range"
        )));
    }

    let text = source
        .full_text(&doc.path, Side::New)?
        .ok_or_else(|| AnnotError::Diff("full text unavailable to unfold".into()))?;
    let new_lines: Vec<&str> = text.lines().collect();

    let step = match amount {
        ExpandAmount::Step => EXPAND_STEP,
        ExpandAmount::All => u32::MAX,
    };
    let cur_old = true_range(&doc.hunks[hunk_index].old_range);
    let cur_new = true_range(&doc.hunks[hunk_index].new_range);

    // The gap is all context, so old and new coordinates stay in lockstep:
    // an added row at new line n is old line n − delta, delta read off the
    // hunk edge being grown. `[add_start, add_end)` in new-side coords.
    let (add_start, add_end, delta) = match direction {
        ExpandDirection::Up => {
            let bound = if hunk_index == 0 {
                1
            } else {
                true_range(&doc.hunks[hunk_index - 1].new_range).end
            };
            let start = cur_new.start.saturating_sub(step).max(bound);
            (start, cur_new.start, cur_new.start - cur_old.start)
        }
        ExpandDirection::Down => {
            let bound = if hunk_index + 1 == doc.hunks.len() {
                new_len + 1
            } else {
                true_range(&doc.hunks[hunk_index + 1].new_range).start
            };
            let end = cur_new.end.saturating_add(step).min(bound);
            (cur_new.end, end, cur_new.end - cur_old.end)
        }
    };
    if add_start >= add_end {
        return Ok(()); // already at the boundary — nothing to unfold
    }

    let fake_path = format!("file.{}", doc.language);
    let rows: Vec<Row> = (add_start..add_end)
        .map(|n| {
            let content = line_at(&new_lines, n);
            let html = (!doc.language.is_empty())
                .then(|| highlighter.highlight_diff_row(content, &fake_path))
                .flatten();
            Row {
                old_line: Some(n - delta),
                new_line: Some(n),
                content: content.to_string(),
                html: html.map(LineHtml::Full),
                word_ranges: Vec::new(),
            }
        })
        .collect();
    let count = add_end - add_start;

    // A grown side is never empty again, so true coords are stored verbatim.
    let hunk = &mut doc.hunks[hunk_index];
    match direction {
        ExpandDirection::Up => {
            hunk.old_range = cur_old.start - count..cur_old.end;
            hunk.new_range = add_start..cur_new.end;
            hunk.rows.splice(0..0, rows);
        }
        ExpandDirection::Down => {
            hunk.old_range = cur_old.start..cur_old.end + count;
            hunk.new_range = cur_new.start..add_end;
            hunk.rows.extend(rows);
        }
    }

    merge_touching_hunks(doc);
    Ok(())
}

/// Restore the invariant that no two hunks overlap or touch: merge any
/// neighbor pair whose true new-side ranges meet (half-open ⇒ `end >= start`
/// covers both overlap and adjacency). The earlier hunk's function context
/// survives, matching what git would print for the merged hunk.
fn merge_touching_hunks(doc: &mut DiffDocument) {
    let hunks = std::mem::take(&mut doc.hunks);
    let mut merged: Vec<HunkV2> = Vec::with_capacity(hunks.len());
    for hunk in hunks {
        if let Some(prev) = merged.last_mut() {
            if true_range(&prev.new_range).end >= true_range(&hunk.new_range).start {
                prev.old_range = true_range(&prev.old_range).start..true_range(&hunk.old_range).end;
                prev.new_range = true_range(&prev.new_range).start..true_range(&hunk.new_range).end;
                prev.rows.extend(hunk.rows);
                continue;
            }
        }
        merged.push(hunk);
    }
    doc.hunks = merged;
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
    use crate::state::{ContentModel, ContentView, LineHtml};
    use crate::testutil::git;
    use crate::vcs::{enumerate, DiffTarget};
    use std::path::Path;

    fn render_repo(p: &Path, target: &DiffTarget) -> Vec<DiffDocument> {
        let entries = enumerate(p, target, &[]).unwrap();
        let source = GixSource::new(gix::discover(p).unwrap(), build_oid_map(&entries));
        render(&entries, &source, &Highlighter::new(), CONTEXT_LINES).unwrap()
    }

    /// Compact textual form of the documents: identity, hunk ranges, row
    /// numbers, raw content — the full wire-relevant surface except html.
    fn dump(docs: &[DiffDocument]) -> String {
        let num = |n: Option<u32>| n.map_or("·".to_string(), |n| n.to_string());
        docs.iter()
            .flat_map(|doc| {
                let renamed = doc
                    .old_path
                    .as_deref()
                    .map(|p| format!(" (from {p})"))
                    .unwrap_or_default();
                let unavailable = if doc.unavailable { " unavailable" } else { "" };
                let header = format!(
                    "=== {}{renamed} [{:?}] lang={}{unavailable}",
                    doc.path, doc.status, doc.language
                );
                std::iter::once(header).chain(doc.hunks.iter().flat_map(|hunk| {
                    let ctx = hunk
                        .function_context
                        .as_deref()
                        .map(|c| format!(" {c}"))
                        .unwrap_or_default();
                    let ranges = format!(
                        "@@ -{},{} +{},{} @@{ctx}",
                        hunk.old_range.start,
                        hunk.old_range.end - hunk.old_range.start,
                        hunk.new_range.start,
                        hunk.new_range.end - hunk.new_range.start,
                    );
                    std::iter::once(ranges).chain(hunk.rows.iter().map(|row| {
                        format!(
                            "{:>4} {:>4} |{}",
                            num(row.old_line),
                            num(row.new_line),
                            row.content
                        )
                    }))
                }))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Every row's line numbers land inside its hunk's declared ranges.
    fn assert_rows_within_ranges(docs: &[DiffDocument]) {
        for doc in docs {
            for hunk in &doc.hunks {
                for row in &hunk.rows {
                    if let Some(old) = row.old_line {
                        assert!(
                            hunk.old_range.contains(&old),
                            "{old} ∉ {:?}",
                            hunk.old_range
                        );
                    }
                    if let Some(new) = row.new_line {
                        assert!(
                            hunk.new_range.contains(&new),
                            "{new} ∉ {:?}",
                            hunk.new_range
                        );
                    }
                }
            }
        }
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
        let docs = render_repo(dir.path(), &head_range());
        insta::assert_snapshot!(dump(&docs));
        assert_rows_within_ranges(&docs);
    }

    #[test]
    fn utf16_ranges_convert_byte_offsets() {
        // "aé😀b" — é: 2 bytes / 1 code unit; 😀: 4 bytes / 2 code units
        // (non-BMP, where byte, char, and UTF-16 offsets all differ).
        let text = "aé😀b";
        assert_eq!(
            utf16_ranges(text, &[0..1, 1..3, 3..7, 7..8, 0..8]),
            vec![0..1, 1..2, 2..4, 4..5, 0..5]
        );
    }

    /// The fixture's `let d = 4;` → `let d = 40;` pair sits in a gated hunk:
    /// both sides of the pair must reach the wire with word ranges.
    #[test]
    fn gated_rows_carry_word_ranges() {
        let dir = range_fixture(Fixture::LegacySafe);
        let docs = render_repo(dir.path(), &head_range());
        let main = docs.iter().find(|d| d.path == "main.rs").unwrap();
        let with_ranges = main
            .hunks
            .iter()
            .flat_map(|h| &h.rows)
            .filter(|r| !r.word_ranges.is_empty())
            .count();
        assert_eq!(with_ranges, 2);
    }

    /// The strangler bar: the new pipeline's changed rows must be exactly the
    /// ones the legacy path (git CLI patch → parse_diff) produces.
    #[test]
    fn parity_with_legacy_parser_on_fixture() {
        let dir = range_fixture(Fixture::LegacySafe);
        let p = dir.path();
        let new_docs = render_repo(p, &head_range());

        let patch = git(p, &["diff", "HEAD~1..HEAD"]);
        let cli_source = ContentSource::Cli(CliSource::Stdin {
            label: "diff".into(),
        });
        let legacy = ContentModel::from_diff(&patch, cli_source).unwrap();
        let ContentView::Diff {
            documents: legacy_docs,
        } = &legacy.view
        else {
            panic!("legacy model is not a diff");
        };

        let names = |docs: &[DiffDocument]| {
            docs.iter()
                .map(|d| (d.path.clone(), d.old_path.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&new_docs), names(legacy_docs));

        // Changed rows: identical content and line numbers. Context rows are
        // excluded — hunk boundaries may differ cosmetically between engines
        // (accepted at design time). noeol.txt is excluded from the
        // side-by-side because the legacy parser miscounts there: it treats
        // the `\ No newline at end of file` marker as a context line, shifting
        // every following new-side number by one (asserted correct below).
        let changed = |docs: &[DiffDocument]| {
            docs.iter()
                .filter(|doc| doc.path != "noeol.txt")
                .flat_map(|doc| {
                    doc.hunks
                        .iter()
                        .flat_map(|h| &h.rows)
                        .filter(|r| r.old_line.is_none() || r.new_line.is_none())
                        .map(|r| (doc.path.clone(), r.old_line, r.new_line, r.content.clone()))
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(changed(&new_docs), changed(legacy_docs));

        // The re-added `beta` really is line 2 of the new file — the number
        // the legacy parser gets wrong.
        assert!(new_docs
            .iter()
            .flat_map(|d| d.hunks.iter().flat_map(|h| &h.rows))
            .any(|r| r.content == "beta" && r.old_line.is_none() && r.new_line == Some(2)));
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

        let docs = render_repo(p, &DiffTarget::WorkingTree);
        insta::assert_snapshot!(dump(&docs));
    }

    #[test]
    fn rows_are_highlighted_raw() {
        let dir = range_fixture(Fixture::WithRename);
        let docs = render_repo(dir.path(), &head_range());

        let added_rs = docs
            .iter()
            .flat_map(|d| d.hunks.iter().flat_map(|h| &h.rows))
            .find(|r| r.old_line.is_none() && r.content.contains("let d = 40;"))
            .unwrap();
        match &added_rs.html {
            // Highlighted, and no textual sign — the sign is presentation.
            Some(LineHtml::Full(html)) => assert!(!html.starts_with('+')),
            other => panic!("expected highlighted row, got {other:?}"),
        }

        let main_doc = docs.iter().find(|d| d.path == "main.rs").unwrap();
        assert!(main_doc.hunks[0].function_context_html.is_some());
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
        let new_docs = render_repo(repo_root, &DiffTarget::WorkingTree);

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
        let ContentView::Diff {
            documents: legacy_docs,
        } = &legacy.view
        else {
            panic!("legacy model is not a diff");
        };

        let changed = |docs: &[DiffDocument]| {
            docs.iter()
                .flat_map(|doc| {
                    doc.hunks
                        .iter()
                        .flat_map(|h| &h.rows)
                        .filter(|r| r.old_line.is_none() || r.new_line.is_none())
                        .map(|r| {
                            format!(
                                "{} {:?}/{:?} {}",
                                doc.path, r.old_line, r.new_line, r.content
                            )
                        })
                })
                .collect::<std::collections::BTreeSet<_>>()
        };
        let (ours, theirs) = (changed(&new_docs), changed(legacy_docs));
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

    // ========== Unfold (S3) ==========

    /// 100-line file, lines 10 and 60 changed → two hunks (7..14, 57..64)
    /// with a leading gap, a 43-line interior gap, and a trailing gap.
    fn two_hunk_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        let numbered =
            |edit: &dyn Fn(u32) -> String| (1..=100).map(edit).collect::<Vec<_>>().join("");
        std::fs::write(p.join("big.txt"), numbered(&|n| format!("line {n}\n"))).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "one"]);
        std::fs::write(
            p.join("big.txt"),
            numbered(&|n| {
                if n == 10 || n == 60 {
                    format!("line {n} changed\n")
                } else {
                    format!("line {n}\n")
                }
            }),
        )
        .unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "two"]);
        dir
    }

    fn docs_and_source(p: &Path) -> (Vec<DiffDocument>, GixSource) {
        let entries = enumerate(p, &head_range(), &[]).unwrap();
        let source = GixSource::new(gix::discover(p).unwrap(), build_oid_map(&entries));
        let docs = render(&entries, &source, &Highlighter::new(), CONTEXT_LINES).unwrap();
        (docs, source)
    }

    fn expand(
        doc: &mut DiffDocument,
        source: &GixSource,
        hunk: usize,
        direction: ExpandDirection,
        amount: ExpandAmount,
    ) {
        expand_context(doc, source, &Highlighter::new(), hunk, direction, amount).unwrap();
    }

    #[test]
    fn new_len_is_the_capability_signal() {
        let dir = two_hunk_repo();
        let (docs, _) = docs_and_source(dir.path());
        assert_eq!(docs[0].new_len, Some(100));

        // Raw patch mode: no full text, no unfold.
        let patch = git(dir.path(), &["diff", "HEAD~1..HEAD"]);
        let raw = ContentModel::from_diff(
            &patch,
            ContentSource::Cli(CliSource::Stdin {
                label: "diff".into(),
            }),
        )
        .unwrap();
        let ContentView::Diff { documents } = &raw.view else {
            panic!("not a diff");
        };
        assert_eq!(documents[0].new_len, None);

        let mut doc = documents[0].clone();
        let err = expand_context(
            &mut doc,
            &crate::source::RawPatchSource,
            &Highlighter::new(),
            0,
            ExpandDirection::Up,
            ExpandAmount::Step,
        );
        assert!(err.is_err());
    }

    #[test]
    fn expand_up_step_splices_context_rows() {
        let dir = two_hunk_repo();
        let (mut docs, source) = docs_and_source(dir.path());
        let doc = &mut docs[0];
        assert_eq!(doc.hunks[1].new_range, 57..64);

        expand(doc, &source, 1, ExpandDirection::Up, ExpandAmount::Step);
        assert_eq!(doc.hunks.len(), 2, "43-line gap survives a 20-line step");
        assert_eq!(doc.hunks[1].new_range, 37..64);
        assert_eq!(doc.hunks[1].old_range, 37..64);
        let first = &doc.hunks[1].rows[0];
        assert_eq!(
            (first.old_line, first.new_line, first.content.as_str()),
            (Some(37), Some(37), "line 37")
        );
        assert_rows_within_ranges(&docs);
    }

    #[test]
    fn expand_down_all_consumes_gap_and_merges() {
        let dir = two_hunk_repo();
        let (mut docs, source) = docs_and_source(dir.path());
        let doc = &mut docs[0];
        let h0_ctx = doc.hunks[0].function_context.clone();

        expand(doc, &source, 0, ExpandDirection::Down, ExpandAmount::All);
        assert_eq!(doc.hunks.len(), 1, "touching hunks merge");
        assert_eq!(doc.hunks[0].new_range, 7..64);
        assert_eq!(doc.hunks[0].old_range, 7..64);
        assert_eq!(doc.hunks[0].function_context, h0_ctx);
        // 8 original (3+del+add+3) + 43 unfolded + 8 original rows.
        assert_eq!(doc.hunks[0].rows.len(), 59);
        let numbers: Vec<u32> = doc.hunks[0]
            .rows
            .iter()
            .filter_map(|r| r.new_line)
            .collect();
        assert!(numbers.windows(2).all(|w| w[0] < w[1]));
        assert_rows_within_ranges(&docs);
    }

    #[test]
    fn expand_up_all_merges_into_previous_hunk() {
        let dir = two_hunk_repo();
        let (mut docs, source) = docs_and_source(dir.path());
        let doc = &mut docs[0];

        expand(doc, &source, 1, ExpandDirection::Up, ExpandAmount::All);
        assert_eq!(doc.hunks.len(), 1);
        assert_eq!(doc.hunks[0].new_range, 7..64);
    }

    #[test]
    fn expand_clamps_at_file_edges() {
        let dir = two_hunk_repo();
        let (mut docs, source) = docs_and_source(dir.path());
        let doc = &mut docs[0];

        // Leading gap is 6 lines — a 20-line step clamps to the file top.
        expand(doc, &source, 0, ExpandDirection::Up, ExpandAmount::Step);
        assert_eq!(doc.hunks[0].new_range, 1..14);
        assert_eq!(doc.hunks[0].rows[0].new_line, Some(1));

        // Trailing gap is 37 lines — All clamps to new_len.
        expand(doc, &source, 1, ExpandDirection::Down, ExpandAmount::All);
        assert_eq!(doc.hunks[1].new_range, 57..101);
        assert_eq!(doc.hunks[1].rows.last().unwrap().new_line, Some(100));
        assert_eq!(
            doc.hunks[1].rows.last().unwrap().content,
            "line 100",
            "slice reads the real file tail"
        );
        assert_rows_within_ranges(&docs);

        // Both hunks now sit at their boundaries — further expansion no-ops.
        let before = dump(&docs);
        let (doc, before) = (&mut docs[0], before);
        expand(doc, &source, 0, ExpandDirection::Up, ExpandAmount::All);
        expand(doc, &source, 1, ExpandDirection::Down, ExpandAmount::Step);
        assert_eq!(dump(&docs), before);
    }

    #[test]
    fn expand_on_added_file_is_a_noop() {
        // An added file's single hunk already covers every line; its old
        // side is the printed-empty `0..0` — the true-range conversion must
        // not underflow or invent rows.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        std::fs::write(p.join("seed.txt"), "seed\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "one"]);
        std::fs::write(p.join("fresh.txt"), "alpha\nbeta\ngamma\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "two"]);

        let (mut docs, source) = docs_and_source(p);
        let doc = &mut docs[0];
        assert_eq!(doc.new_len, Some(3));
        let before = dump(&docs);
        let doc = &mut docs[0];
        expand(doc, &source, 0, ExpandDirection::Up, ExpandAmount::All);
        expand(doc, &source, 0, ExpandDirection::Down, ExpandAmount::All);
        assert_eq!(dump(&docs), before);
    }

    /// The keystone's proof: expansion lives in backend state, so an
    /// annotation on an unfolded row resolves in output instead of
    /// degrading to a bare file header.
    #[test]
    fn annotation_on_expanded_row_lands_in_output() {
        use crate::anchor::{Anchor, Endpoint};
        use crate::output::{format_output, OutputMode};
        use crate::review::{FileKey, Review, View};
        use crate::state::{ContentNode, UserConfig};

        let dir = two_hunk_repo();
        let content = ContentModel::from_git(
            dir.path(),
            &head_range(),
            &[],
            ContentSource::Cli(CliSource::Stdin {
                label: "diff".into(),
            }),
        )
        .unwrap();
        let mut review = Review::cli(content, UserConfig::empty(), "main".to_string());

        // Mirror the expand_context command's mutation path.
        let View::Diff { content, .. } = &mut review.root_view else {
            panic!("not a diff review");
        };
        let file_source = content.file_source.clone();
        let ContentView::Diff { documents } = &mut content.view else {
            panic!("not a diff view");
        };
        expand_context(
            &mut documents[0],
            file_source.as_ref(),
            &Highlighter::new(),
            1,
            ExpandDirection::Up,
            ExpandAmount::Step,
        )
        .unwrap();

        // Line 40 exists only as an unfolded context row.
        let target = review.files.get_mut(&FileKey::diff_file(0)).unwrap();
        let endpoint = |line| Endpoint {
            side: Side::New,
            line,
        };
        target.upsert_annotation(
            "a1".to_string(),
            Anchor::Diff {
                path: "big.txt".to_string(),
                start: endpoint(40),
                end: endpoint(40),
            },
            vec![ContentNode::Text {
                text: "unfolded context annotation".to_string(),
            }],
        );

        let output = format_output(&review, OutputMode::Cli).text;
        assert!(
            output.contains("line 40"),
            "expanded row must resolve in output, got:\n{output}"
        );
        assert!(output.contains("unfolded context annotation"));
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
