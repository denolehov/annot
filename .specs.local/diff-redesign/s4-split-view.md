---
id: S4
kind: story
wave: 3
depends_on: [C1]
status: fogged
---

# Primer: S4 — Split (side-by-side) view + persisted toggle

> Fogged. Clear after C1; likely the fattest leaf — consider slicing when specing.

**Goal:** Two-column view: old left, new right; context rows span-aligned; deleted/added rows paired within a change run, shorter side padded with filler cells. Toggle via shortcut + palette; **unified stays default**; choice persisted in config (`src-tauri/src/config.rs` — same persistence path as tags/exit-modes).

**Settled constraints:**
- Split is a **projection of the same rows** (C1's `(side, old_line, new_line)` model) — no second data pipeline, no new wire format. Pairing walks hunk rows: context → one row both cells; change runs → pair deletions/additions by index up to `max(dels, adds)` (hunk's `buildSplitRows` pattern), padding with empty cells.
- Annotations: side is implicit from the column clicked; anchors are already side-aware (A1/A2), so the model needs zero changes — this story is UI only.
- Selection model in split view: column-scoped ranges; mixed-side range creation stays a unified-view gesture (settled — replacements are selected in unified).
- Word-level highlight spans (S5) must render in both views if S5 lands first.

**Risks to plan around when clearing fog:** keyboard nav semantics across two columns (j/k walks rows; h/l or focus model for columns?); `LineRow.svelte` reuse vs a `SplitRow` sibling; line-wrap alignment between cells (CSS grid row auto-height keeps pairs aligned — hunk pads with terminal cells, DOM can align naturally).
