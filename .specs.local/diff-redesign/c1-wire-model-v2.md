---
id: C1
kind: refactor
wave: 2
depends_on: [A2, B4]
status: fogged
---

# Primer: C1 — Wire model v2 (per-file documents) — THE JOIN

> Fogged, deliberately: this node is shaped by what A2 and B4 leave behind.
> Keep it starved — mostly reshaping and deletion. If it's accumulating logic,
> something belonged in A2/B4 and should be pushed back.

**Goal:** `ContentResponse` for diff mode becomes per-file documents — `{ path, status, hunks, rows }`, rows carrying `(side, old_line, new_line)` — replacing the single flat `Vec<Line>` for diffs. Frontend renders per-file sections. Flat contract retired for diff mode (file/markdown modes keep theirs).

**Why after A2:** annotations no longer key on display index, so restructuring the render array breaks nothing.
**Why after B4:** backend already *has* the per-file model internally; this node exposes it instead of flattening it.

**Settled constraints:**
- S1/S2 refit here: tree binds to documents (rename `old → new` display arrives), collapse becomes structural instead of render-skip. Their specs name the single derivation points to rebind.
- `review.rs` already tracks per-file `AnnotationTarget`s (`FileKey::diff_file(index)`) — backend identity mostly survives; it's the wire + frontend spine that changes.
- Virtual scrolling consideration: per-file sections change the scroll container geometry — check `adaptiveScrollOverscan`-style logic if any exists frontend-side before assuming free.

**Exit criteria to write when clearing fog:** demo:diff renders identically (modulo settled cosmetics) through the new model; annotations created pre-C1 sessions aren't a concern (sessions are ephemeral — no migration).
