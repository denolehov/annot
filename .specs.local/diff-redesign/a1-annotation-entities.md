---
id: A1
kind: refactor
wave: 0
depends_on: []
status: ready
---

# [Spec]: A1 — Backend annotation entities (id + anchor)

## Requirements
- **Problem:** Annotations are keyed by position — `HashMap<LineRange, Annotation>` per target (`src-tauri/src/review.rs:113-115`), no identity, no side. Unfold/split/threads/stateful reviews all need annotations that survive position changes.
- **Beneficiary:** Every downstream node (A2, C1, S3, S4) plus parked features (threads need ids, stateful reviews need source anchors).
- **Done when:** Backend stores `Annotation { id, anchor, content }` keyed by id; IPC takes id + anchor; `pnpm test:rust` green with **unchanged** output snapshots.

## Entities

```
Side      = Old | New
Endpoint  = { side: Side, line: u32 }          // line = 1-indexed source line
Anchor    = { path: String, start: Endpoint, end: Endpoint }
Annotation = { id: String /* uuid v4, dep exists */, anchor: Anchor, content: Vec<ContentNode> }

AnnotationTarget.annotations: HashMap<LineRange, Annotation>   // current
AnnotationTarget.annotations: IndexMap<String, Annotation>     // proposed (indexmap is a dep; preserves insertion order for output)
```

Degenerate cases: file/content/markdown modes use `side: New` everywhere — one
anchor type covers all three review modes. `start == end` for single-line.

## Approach

**Keystone:** the id is the identity; the anchor is a mutable property.
Rejected alternative: keeping position-composite keys (`LineRange`) — that
makes "same annotation, moved anchor" unrepresentable, which kills threads and
re-anchoring after re-diff.

Sequence:
1. New types (`Side`, `Endpoint`, `Anchor`) in `review.rs` (or new `anchor.rs`), serde-derived.
2. Re-key `AnnotationTarget.annotations`; rewrite `upsert_annotation`/`delete_annotation` (`review.rs:490-508`) to take `(id, anchor, content)` / `(id)`.
3. IPC: `upsert_annotation`/`delete_annotation` commands (`src-tauri/src/commands.rs:58-82`) accept the new shape. Frontend call site (`src/lib/composables/useAnnotations.svelte.ts` pending-sync block) adapts *minimally*: generate uuid client-side at creation, map its existing `coords` (path/start/end source lines) into an Anchor with `side` from `line.origin` (old_line-only lines → Old, else New). Full frontend re-keying is A2, not here.
4. Output builder (`src-tauri/src/output/`) iterates entities instead of range-keyed map. Ordering: sort by (file, anchor position) to keep snapshots byte-identical.
5. Tag-usage walk (`commands.rs:140-160`) — mechanical update.

**Seams:**
- `id` field → detached-review threads (parked): replies will reference annotation ids.
- `Anchor.side` → S4 split view annotates either column with no model change.

## Structure
- `src-tauri/src/review.rs` — types, storage, upsert/delete
- `src-tauri/src/commands.rs` — IPC signatures
- `src-tauri/src/output/builder.rs` — consume entities, keep rendering identical
- `src/lib/composables/useAnnotations.svelte.ts` — thin adapter only (id generation + side mapping)

## Norms
- Declarative style (map/collect) per CLAUDE.md.
- Insta snapshot workflow: `cargo test` → `cargo insta review` — but the bar here is **zero snapshot churn** (O1 is where output changes).

## Safeguards
- Two annotations on the same range must coexist (new capability — add a test).
- Mixed-side anchors (`start.side != end.side`) must serialize/deserialize round-trip (test now, rendered in O1).
- Frontend behavior unchanged: create/edit/delete annotation in `pnpm demo:diff` works as before.

## Scope
- In: backend model, IPC shape, minimal frontend adapter, tests.
- Out: frontend store re-keying (A2), output format changes (O1), any UI.
