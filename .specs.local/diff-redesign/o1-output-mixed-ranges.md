---
id: O1
kind: refactor
wave: 1
depends_on: [A1]
status: fogged
---

# Primer: O1 — Output rendering for mixed-side ranges

> Fogged. Clear before starting once A1's entity shape is final.

**Goal:** Structured output (`src-tauri/src/output/`) renders two-endpoint anchors, including mixed-side ranges (annotation spanning deleted+added replacement). The existing `old:new` gutter format (`file.rs (old:2)` style — see `output/snapshots/annot_lib__output__snapshot_tests__diff_annotation_deleted_line.snap`) already speaks sides; this extends it to ranges whose endpoints sit on different sides.

**Why after A1:** consumes the entity model; A1 deliberately kept snapshots byte-identical, this node is where snapshot churn is *allowed*.

**Settled constraints:**
- Agents parse this output — the format is a contract. Additive/unambiguous changes only; keep single-side annotations rendering exactly as today.
- Insta workflow: `cargo test` → `cargo insta review` → commit `.snap`.

**Design question to settle when clearing fog:** how a mixed-side range names itself in the header — e.g. `file.rs (old:2 → new:5)` — pick something an LLM can't misread, add corpus snapshots for: old-only, new-only, mixed, multi-line each.
