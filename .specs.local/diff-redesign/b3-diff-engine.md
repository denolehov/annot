---
id: B3
kind: refactor
wave: 0
depends_on: []
status: ready
---

# [Spec]: B3 — In-process diff engine + word diffs

## Requirements
- **Problem:** Hunks are parsed from `git diff` text (`unidiff` in `src-tauri/src/diff.rs:87`), so annot can only know what the patch says — no re-diff, no word-level ranges, no control over context.
- **Beneficiary:** B4 (hunk computation), S5 (word highlights fall out), S4 (old↔new line mapping for split pairing), future re-diff on file change.
- **Done when:** `compute_hunks(old, new, context)` matches `git diff` semantics on a corpus of fixture pairs (insta-snapshotted), word diffs emitted for small hunks.

## Entities

```rust
pub struct FileDiff {
    pub hunks: Vec<Hunk>,
}
pub struct Hunk {
    pub old_range: Range<u32>,          // 1-indexed lines in old text (incl. context)
    pub new_range: Range<u32>,
    pub rows: Vec<DiffRow>,             // ordered: context | deleted | added
}
pub enum DiffRow {
    Context { old_line: u32, new_line: u32 },
    Deleted { old_line: u32, word_ranges: Vec<Range<usize>> },   // byte ranges within the line
    Added   { new_line: u32, word_ranges: Vec<Range<usize>> },
}
```

Pure function, no I/O:
`pub fn compute_hunks(old: &str, new: &str, context: u32) -> FileDiff`

## Approach

**Keystone:** hunks become a *derived overlay over two full texts* (Zed's
model), computed by us — the patch is no longer the source of truth for git
mode. Rejected alternative: keep parsing git's patch and bolt word-diffs on
top — leaves T3 false, blocks re-diff, and word alignment against parsed
text is guess-work.

**Engine choice — UNSETTLED, decide at implementation start:**
- `similar = "3"` is **already a dependency with the `inline` feature**
  (Cargo.toml) — `TextDiff` line diffs + built-in inline (word-level) change
  ranges. No new dep. Algorithms: Myers/Patience/LCS (no Histogram).
- `imara-diff` — what Zed uses; Histogram algorithm (better hunk quality on
  code), faster; new dep, word-diff hand-rolled (token-level second pass).
- Lean: **similar-first** — the engine hides behind `compute_hunks`, so a swap
  is contained if hunk quality or perf disappoints. The signature is the
  contract; the crate is an implementation detail.

Word-diff gate (Zed's discipline): only when a hunk's deleted/added line
counts are equal and ≤ 5 lines; token-level, word boundaries. Prevents
noise-highlighting on rewrites.

Note: `--diff-algorithm` differences mean output may diverge cosmetically from
`git diff`. Accepted at design time (grill session).

**Seams:**
- `context` param → S3 unfold and a future "more context" setting share the machinery.
- Old↔new line mapping implicit in `rows` → S4 split pairing walks it directly.

## Structure
- New: `src-tauri/src/engine.rs` (name TBD; NOT in `diff.rs` — that stays the legacy patch parser until C1 shrinks it to raw-mode-only)
- Heavy unit + insta tests: `src-tauri/src/engine.rs` tests module, snapshots beside existing `output/snapshots/` pattern.

## Operations
1. Line diff → grouped ops → hunk assembly with `context` merging (adjacent hunks whose context overlaps merge — mirrors git).
2. Line-number bookkeeping (the four running counters: old/new × index/line-number).
3. Word-diff pass on gated hunks; byte ranges per line.
4. Corpus tests: empty→content, content→empty, pure add/delete, replacement, adjacent-hunk merge, no-trailing-newline, CRLF, unicode.

## Norms
- Pure function, zero I/O — the most unit-testable node in the graph; build and trust it first.
- Declarative style per CLAUDE.md.

## Safeguards
- Property test worth writing: reconstructing `new` from `old` + hunks round-trips exactly (including trailing-newline edge cases).
- Word ranges are byte offsets into the line — must slice at char boundaries (test with multibyte).

## Scope
- In: engine, word diffs, corpus tests.
- Out: file loading (B1), enumeration (B2), rendering/HTML, syntax highlighting, wiring (B4).
