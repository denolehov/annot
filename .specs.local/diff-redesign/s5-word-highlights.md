---
id: S5
kind: story
wave: 3
depends_on: [C1]
status: fogged
---

# Primer: S5 — Word-level (intra-line) diff highlights

> Fogged, but thin: cheapest leaf, ship first of wave 3.

**Goal:** Within changed line pairs, the changed tokens get a stronger background (GitHub's darker red/green spans). Data already exists: B3 emits `word_ranges: Vec<Range<usize>>` (byte offsets per line) on `Deleted`/`Added` rows, gated to hunks ≤ ~5 equal lines.

**Settled constraints:**
- Rendering only — no computation frontend-side. Backend already merges word ranges into the line HTML (`highlight.rs` produces per-line html; word-diff spans must compose with syntect spans — nested `<span class="word-diff">` around highlighted tokens) **or** ships ranges for frontend wrapping. Decide when clearing fog; lean backend-composited (frontend stays a dumb renderer, consistent with the rest of the pipeline).
- Byte ranges slice at char boundaries (B3 safeguard) — trust but verify with multibyte fixture.
- Must render in unified now and split (S4) later without rework — style via a class on spans, not view-specific markup.

**Reference:** Zed gates at ≤5-line hunks with equal add/del counts (`MAX_WORD_DIFF_LINE_COUNT`) — the gate lives in B3; if tuning is needed, tune there, not here.
