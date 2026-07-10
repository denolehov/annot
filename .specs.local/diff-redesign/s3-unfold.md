---
id: S3
kind: story
wave: 3
depends_on: [C1, B1]
status: fogged
---

# Primer: S3 — Unfold context between hunks

> Fogged. Clear after C1: the row/section model it splices into is C1's output.

**Goal:** GitHub-style gap bars between hunks ("⋯ 20 unchanged lines" with expand up/down/all); clicking slices rows from B1's cached full text and splices them in.

**Settled constraints:**
- Affordance renders **only when `FileSource::full_text` can return content** — raw `diff_content` mode shows no arrows at all (settled: no unfold there; avoid hunk-the-tool's silent-failure bug).
- Fetch whole file once (B1 caches); every unfold is a local slice. Loading/error/too-large states on the gap bar (hunk's state machine: `loading | loaded | error | too-large`).
- Expansion rows are **tagged** and excluded from hunk bounds / annotation-anchor derivation (hunk's `isExpansionRow` discipline) — anchors must not drift when context is unfolded.
- Annotating an expanded (context) row is allowed and anchors new-side like any context line.
- Gap identity: `(file, position before/after hunkIndex)` — expansion state is per-session, ephemeral.

**Mechanics reference:** hunk's `expandCollapsedRows.ts` splice (keep the gap bar in place, rewrite label, insert synthesized rows keyed separately) and Zed's merge-adjacent-regions rule (fully unfolded gap disappears; adjacent expansions merge).

**IPC to design when clearing fog:** frontend asks backend for gap lines (`expand_gap(file, old_range, new_range) → rows` with html-highlighted lines) vs shipping full texts to the frontend at load. Lean backend-slicing — keeps highlighting (`syntect`) and memory in one place.
