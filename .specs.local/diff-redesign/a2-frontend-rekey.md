---
id: A2
kind: refactor
wave: 1
depends_on: [A1]
status: fogged
---

# Primer: A2 — Frontend id-keyed annotation store

> Fogged. Clear before starting: rewrite into full spec shape once A1 has landed
> (its final IPC shape and any surprises feed this).

**Goal:** Frontend annotations keyed by id with `{path, start:{side,line}, end:{side,line}}` anchors; display index demoted to ephemeral selection state.

**Why after A1:** the backend contract (id + anchor IPC) must exist to key against; A1 also leaves a thin frontend adapter marking exactly the seams this node replaces.

**Why before C1:** lands against the *current flat model* so the C1 join doesn't change identity and wire shape simultaneously. `line.origin` already carries `old_line`/`new_line` (`src/lib/types.ts:6-9`) — anchors are computable today.

**Settled constraints:**
- Anchor computed from selection at *creation*; resolved anchor → row at *render* (needs an anchor→displayIndex map derived from the lines array).
- Context lines anchor new-side; old-side only for deleted files.
- Mixed-side ranges (replacement spans) must be creatable from a unified-view selection.

**Blast radius (from A1-era code):** `src/lib/range.ts` (display-index Range dies here), `useAnnotations.svelte.ts` (rangeToKey store → id map), `useAnnotationEditor`, `useHistory` (undo/redo references), `AnnotationSlot.svelte` / `LineRow.svelte` (render lookup), selection plumbing in `useInteraction`.

**Risk to plan around:** undo/redo history currently stores range keys — decide entity-level history semantics before coding.
