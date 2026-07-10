//! In-process diff engine: hunks computed from two full texts.
//!
//! Hunks are a derived overlay over the two sides — the patch is not the
//! source of truth here. `diff.rs` stays the legacy parser for raw
//! `diff_content` input. The `similar` crate is an implementation detail
//! hidden behind `compute_hunks`; the signature is the contract.

use std::ops::Range;

use serde::Serialize;
use similar::{capture_diff_slices, group_diff_ops, Algorithm, ChangeTag, DiffOp, TextDiff};

/// Computed diff between two versions of one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileDiff {
    pub hunks: Vec<Hunk>,
}

/// A run of changes plus surrounding context.
///
/// Ranges are half-open over 1-indexed line numbers, context included.
/// A pure insertion with zero context has an empty `old_range` positioned
/// where the lines were inserted (and vice versa for deletions).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Hunk {
    pub old_range: Range<u32>,
    pub new_range: Range<u32>,
    pub rows: Vec<DiffRow>,
}

/// One row of a hunk. Replaced blocks emit all deleted rows, then all added.
///
/// `word_ranges` are byte ranges into the line content (terminator excluded),
/// always on char boundaries; non-empty only in word-diff-gated hunks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DiffRow {
    Context {
        old_line: u32,
        new_line: u32,
    },
    Deleted {
        old_line: u32,
        word_ranges: Vec<Range<usize>>,
    },
    Added {
        new_line: u32,
        word_ranges: Vec<Range<usize>>,
    },
}

/// Word ranges are computed only for hunks whose deleted/added line counts
/// are equal and at most this many lines — avoids noise-highlighting rewrites.
const WORD_DIFF_MAX_LINES: usize = 5;

/// Compute hunks between two full texts with `context` lines around changes.
/// Adjacent hunks whose context overlaps are merged, mirroring git.
///
/// Lines are compared with their terminators, so a missing trailing newline
/// is a change (git's `\ No newline at end of file`).
pub fn compute_hunks(old: &str, new: &str, context: u32) -> FileDiff {
    let old_raw: Vec<&str> = old.split_inclusive('\n').collect();
    let new_raw: Vec<&str> = new.split_inclusive('\n').collect();
    let ops = capture_diff_slices(Algorithm::Myers, &old_raw, &new_raw);

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let hunks = group_diff_ops(ops, context as usize)
        .iter()
        .map(|ops| build_hunk(ops, &old_lines, &new_lines))
        .collect();

    FileDiff { hunks }
}

fn build_hunk(ops: &[DiffOp], old_lines: &[&str], new_lines: &[&str]) -> Hunk {
    let mut rows: Vec<DiffRow> = ops.iter().flat_map(rows_for_op).collect();
    apply_word_diffs(&mut rows, old_lines, new_lines);

    let as_u32 = |r: Range<usize>| (r.start as u32 + 1)..(r.end as u32 + 1);
    Hunk {
        old_range: as_u32(span(ops.iter().map(|op| op.old_range()))),
        new_range: as_u32(span(ops.iter().map(|op| op.new_range()))),
        rows,
    }
}

/// Smallest range covering all input ranges (empty ones position the span).
fn span(ranges: impl Iterator<Item = Range<usize>> + Clone) -> Range<usize> {
    let start = ranges.clone().map(|r| r.start).min().unwrap_or(0);
    let end = ranges.map(|r| r.end).max().unwrap_or(0);
    start..end.max(start)
}

fn rows_for_op(op: &DiffOp) -> Vec<DiffRow> {
    let context = |old_index: usize, new_index: usize, len: usize| {
        (0..len)
            .map(|i| DiffRow::Context {
                old_line: (old_index + i + 1) as u32,
                new_line: (new_index + i + 1) as u32,
            })
            .collect::<Vec<_>>()
    };
    let deleted = |old_index: usize, len: usize| {
        (0..len).map(move |i| DiffRow::Deleted {
            old_line: (old_index + i + 1) as u32,
            word_ranges: Vec::new(),
        })
    };
    let added = |new_index: usize, len: usize| {
        (0..len).map(move |i| DiffRow::Added {
            new_line: (new_index + i + 1) as u32,
            word_ranges: Vec::new(),
        })
    };

    match *op {
        DiffOp::Equal {
            old_index,
            new_index,
            len,
        } => context(old_index, new_index, len),
        DiffOp::Delete {
            old_index, old_len, ..
        } => deleted(old_index, old_len).collect(),
        DiffOp::Insert {
            new_index, new_len, ..
        } => added(new_index, new_len).collect(),
        DiffOp::Replace {
            old_index,
            old_len,
            new_index,
            new_len,
        } => deleted(old_index, old_len)
            .chain(added(new_index, new_len))
            .collect(),
    }
}

/// Fill in `word_ranges` for gated hunks by pairing the i-th deleted row
/// with the i-th added row and diffing them token-wise.
fn apply_word_diffs(rows: &mut [DiffRow], old_lines: &[&str], new_lines: &[&str]) {
    let deleted: Vec<usize> = positions(rows, |r| matches!(r, DiffRow::Deleted { .. }));
    let added: Vec<usize> = positions(rows, |r| matches!(r, DiffRow::Added { .. }));

    let gated =
        !deleted.is_empty() && deleted.len() == added.len() && deleted.len() <= WORD_DIFF_MAX_LINES;
    if !gated {
        return;
    }

    for (&di, &ai) in deleted.iter().zip(&added) {
        let (DiffRow::Deleted { old_line, .. }, DiffRow::Added { new_line, .. }) =
            (&rows[di], &rows[ai])
        else {
            unreachable!("positions() matched these variants");
        };
        let (del_ranges, add_ranges) = word_ranges(
            old_lines[(*old_line - 1) as usize],
            new_lines[(*new_line - 1) as usize],
        );
        if let DiffRow::Deleted { word_ranges, .. } = &mut rows[di] {
            *word_ranges = del_ranges;
        }
        if let DiffRow::Added { word_ranges, .. } = &mut rows[ai] {
            *word_ranges = add_ranges;
        }
    }
}

fn positions(rows: &[DiffRow], pred: impl Fn(&DiffRow) -> bool) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(i, r)| pred(r).then_some(i))
        .collect()
}

/// Byte ranges of the changed tokens on each side of a paired line.
/// Tokens are words + whitespace runs, so they concatenate back to the
/// original line and offsets land on char boundaries by construction.
fn word_ranges(old_line: &str, new_line: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let diff = TextDiff::from_words(old_line, new_line);
    let mut old_offset = 0;
    let mut new_offset = 0;
    let mut deleted = Vec::new();
    let mut added = Vec::new();

    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Equal => {
                old_offset += len;
                new_offset += len;
            }
            ChangeTag::Delete => {
                deleted.push(old_offset..old_offset + len);
                old_offset += len;
            }
            ChangeTag::Insert => {
                added.push(new_offset..new_offset + len);
                new_offset += len;
            }
        }
    }

    (coalesce(deleted), coalesce(added))
}

/// Merge byte-adjacent ranges into one.
fn coalesce(ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.into_iter().fold(Vec::new(), |mut acc, r| {
        match acc.last_mut() {
            Some(last) if last.end == r.start => last.end = r.end,
            _ => acc.push(r),
        }
        acc
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// (name, old, new) fixture pairs mirroring git diff situations.
    const CORPUS: &[(&str, &str, &str)] = &[
        ("empty_to_content", "", "hello\nworld\n"),
        ("content_to_empty", "hello\nworld\n", ""),
        (
            "pure_add",
            "alpha\nbeta\ngamma\n",
            "alpha\nbeta\ninserted one\ninserted two\ngamma\n",
        ),
        (
            "pure_delete",
            "alpha\nbeta\nremoved\ngamma\n",
            "alpha\nbeta\ngamma\n",
        ),
        (
            "replacement",
            "fn main() {\n    old_code();\n}\n",
            "fn main() {\n    new_code();\n}\n",
        ),
        (
            "adjacent_hunk_merge",
            "l1\nl2 old\nl3\nl4\nl5\nl6 old\nl7\n",
            "l1\nl2 new\nl3\nl4\nl5\nl6 new\nl7\n",
        ),
        ("no_trailing_newline_added", "a\nb", "a\nb\n"),
        ("no_trailing_newline_changed", "a\nb", "a\nc"),
        ("crlf", "a\r\nb\r\nc\r\n", "a\r\nB\r\nc\r\n"),
        (
            "unicode",
            "café au lait ☕\nвітаю світ\n",
            "café du lait ☕\nвітаю всесвіт\n",
        ),
        (
            "rewrite_beyond_word_gate",
            "one\ntwo\nthree\nfour\nfive\nsix\n",
            "uno\ndos\ntres\ncuatro\ncinco\nseis\n",
        ),
        (
            "unequal_counts_no_word_diff",
            "keep\nold line\nkeep2\n",
            "keep\nnew line\nsecond new\nkeep2\n",
        ),
    ];

    /// Human-readable hunk dump; word ranges shown as »marked« spans.
    /// Slicing at range bounds doubles as the char-boundary safeguard.
    fn render(old: &str, new: &str, diff: &FileDiff) -> String {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        diff.hunks
            .iter()
            .map(|h| {
                let header = format!(
                    "@@ old {}..{} new {}..{} @@",
                    h.old_range.start, h.old_range.end, h.new_range.start, h.new_range.end
                );
                let rows = h.rows.iter().map(|row| match row {
                    DiffRow::Context { old_line, new_line } => format!(
                        "ctx {:>3} {:>3} |{}",
                        old_line,
                        new_line,
                        old_lines[(old_line - 1) as usize]
                    ),
                    DiffRow::Deleted {
                        old_line,
                        word_ranges,
                    } => format!(
                        "del {:>3}     |{}",
                        old_line,
                        mark(old_lines[(old_line - 1) as usize], word_ranges)
                    ),
                    DiffRow::Added {
                        new_line,
                        word_ranges,
                    } => format!(
                        "add     {:>3} |{}",
                        new_line,
                        mark(new_lines[(new_line - 1) as usize], word_ranges)
                    ),
                });
                std::iter::once(header)
                    .chain(rows)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn mark(line: &str, ranges: &[Range<usize>]) -> String {
        let mut out = String::new();
        let mut pos = 0;
        for r in ranges {
            out.push_str(&line[pos..r.start]);
            out.push('»');
            out.push_str(&line[r.start..r.end]);
            out.push('«');
            pos = r.end;
        }
        out.push_str(&line[pos..]);
        out
    }

    /// Rebuild `new` from `old` + hunks: gaps copy old lines, context rows
    /// must match byte-for-byte on both sides, added rows come from `new`.
    fn reconstruct(old: &str, new: &str, diff: &FileDiff) -> String {
        let old_raw: Vec<&str> = old.split_inclusive('\n').collect();
        let new_raw: Vec<&str> = new.split_inclusive('\n').collect();
        let mut out = String::new();
        let mut next_old = 1u32;
        for hunk in &diff.hunks {
            (next_old..hunk.old_range.start).for_each(|l| out.push_str(old_raw[(l - 1) as usize]));
            for row in &hunk.rows {
                match row {
                    DiffRow::Context { old_line, new_line } => {
                        assert_eq!(
                            old_raw[(old_line - 1) as usize],
                            new_raw[(new_line - 1) as usize],
                            "context row differs between sides"
                        );
                        out.push_str(new_raw[(*new_line - 1) as usize]);
                    }
                    DiffRow::Deleted { .. } => {}
                    DiffRow::Added { new_line, .. } => {
                        out.push_str(new_raw[(*new_line - 1) as usize])
                    }
                }
            }
            next_old = hunk.old_range.end;
        }
        (next_old..=old_raw.len() as u32).for_each(|l| out.push_str(old_raw[(l - 1) as usize]));
        out
    }

    fn word_ranges_of(diff: &FileDiff) -> Vec<Vec<Range<usize>>> {
        diff.hunks
            .iter()
            .flat_map(|h| &h.rows)
            .filter_map(|row| match row {
                DiffRow::Deleted { word_ranges, .. } | DiffRow::Added { word_ranges, .. } => {
                    Some(word_ranges.clone())
                }
                DiffRow::Context { .. } => None,
            })
            .collect()
    }

    #[test]
    fn corpus_snapshots() {
        for (name, old, new) in CORPUS {
            insta::assert_snapshot!(*name, render(old, new, &compute_hunks(old, new, 3)));
        }
    }

    #[test]
    fn adjacent_hunks_stay_split_with_small_context() {
        let (_, old, new) = CORPUS
            .iter()
            .find(|(n, ..)| *n == "adjacent_hunk_merge")
            .unwrap();
        insta::assert_snapshot!(
            "adjacent_hunk_split_context_1",
            render(old, new, &compute_hunks(old, new, 1))
        );
    }

    #[test]
    fn corpus_round_trips() {
        for (name, old, new) in CORPUS {
            for context in [0, 1, 3, 100] {
                let diff = compute_hunks(old, new, context);
                assert_eq!(
                    reconstruct(old, new, &diff),
                    *new,
                    "round trip failed: {name} (context {context})"
                );
            }
        }
    }

    #[test]
    fn identical_texts_produce_no_hunks() {
        let text = "a\nb\nc\n";
        assert!(compute_hunks(text, text, 3).hunks.is_empty());
        assert!(compute_hunks("", "", 3).hunks.is_empty());
    }

    #[test]
    fn gated_hunk_gets_word_ranges() {
        let diff = compute_hunks("fn old_name() {\n", "fn new_name() {\n", 0);
        let ranges = word_ranges_of(&diff);
        assert_eq!(ranges.len(), 2);
        assert!(ranges.iter().all(|r| !r.is_empty()));
    }

    #[test]
    fn rewrite_beyond_gate_gets_no_word_ranges() {
        let (_, old, new) = CORPUS
            .iter()
            .find(|(n, ..)| *n == "rewrite_beyond_word_gate")
            .unwrap();
        let diff = compute_hunks(old, new, 0);
        assert!(word_ranges_of(&diff).iter().all(|r| r.is_empty()));
    }

    #[test]
    fn unequal_counts_get_no_word_ranges() {
        let (_, old, new) = CORPUS
            .iter()
            .find(|(n, ..)| *n == "unequal_counts_no_word_diff")
            .unwrap();
        let diff = compute_hunks(old, new, 0);
        assert!(word_ranges_of(&diff).iter().all(|r| r.is_empty()));
    }

    #[test]
    fn multibyte_word_ranges_slice_at_char_boundaries() {
        let old = "вітаю світ 🌍\n";
        let new = "вітаю всесвіт 🌍\n";
        let diff = compute_hunks(old, new, 0);
        // mark() slices the line at every range bound — panics off-boundary.
        render(old, new, &diff);
        assert!(word_ranges_of(&diff).iter().any(|r| !r.is_empty()));
    }

    /// Lines drawn from a tiny pool so LCS finds real structure; multibyte
    /// entries keep char-boundary handling honest.
    fn arb_text() -> impl Strategy<Value = String> {
        let line = prop::sample::select(vec![
            "alpha",
            "beta",
            "gamma",
            "délta",
            "εψιλον",
            "fn x() {",
            "}",
            "",
            "  indented",
        ]);
        (prop::collection::vec(line, 0..12), any::<bool>()).prop_map(|(lines, trailing)| {
            let mut text = lines.join("\n");
            if trailing && !text.is_empty() {
                text.push('\n');
            }
            text
        })
    }

    proptest! {
        #[test]
        fn round_trips_for_arbitrary_pairs(
            old in arb_text(),
            new in arb_text(),
            context in 0u32..5,
        ) {
            let diff = compute_hunks(&old, &new, context);
            prop_assert_eq!(reconstruct(&old, &new, &diff), new);
        }

        #[test]
        fn word_ranges_are_valid_char_boundary_slices(
            old in arb_text(),
            new in arb_text(),
            context in 0u32..5,
        ) {
            let diff = compute_hunks(&old, &new, context);
            // render() slices every word range — panics on invalid bounds.
            render(&old, &new, &diff);
        }
    }
}
