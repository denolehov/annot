---
id: S2
kind: story
wave: 0
depends_on: []
status: ready
---

# [Spec]: S2 — Per-file collapse + "N files changed" header

## Requirements
- **Problem:** A 15-file diff is one undifferentiated wall; you can't set aside files you're done with or don't care about, and nothing summarizes the changeset.
- **Beneficiary:** Multi-file diff reviewers; ships on the current model.
- **Done when:** File headers show a collapse chevron; collapsing hides the file's lines; a summary header shows "N files changed, +A −D"; existing annotations still resolve correctly after collapse/expand cycles.

## Entities
N/A — presentation state only: `collapsedFiles: Set<fileIndex>` in a small composable.

## Approach

**Keystone:** collapse is **render-skip, never array mutation** — the `lines`
array and therefore every display index stays byte-identical, because
annotations are still display-index-keyed until A2 lands. Rejected
alternative: filtering the lines array — silently detaches every annotation
below the first collapsed file.

- File boundaries from `DiffFileInfo.start_line/end_line` (same data as S1).
- Render loop (`RegularLines.svelte`) skips rows whose index falls inside a
  collapsed file's range (keep the `file_header` row visible as the collapsed
  bar, GitHub-style: path, +/− counts, chevron).
- Summary header: derive counts once from `metadata.files` + line semantics
  (`added`/`deleted`), render above the first file.
- Auto-collapse: files whose changed-line count exceeds a threshold (~500)
  start collapsed, like GitHub's "Load diff" barrier. Threshold is a constant,
  not config, until someone asks.
- Selection interaction: if the cursor/selection sits inside a file being
  collapsed, move selection to the file header row.

**Seams:**
- At C1 collapse becomes structural (per-file document sections) — keep the
  collapsed-set composable; only the render-skip mechanism gets replaced.
- Collapsed bar is where a parked "viewed" checkbox would live later.

## Structure
- New: `src/lib/composables/useFileCollapse.svelte.ts`
- `src/lib/components/embedded/RegularLines.svelte` — render-skip + collapsed bar
- `src/lib/components/embedded/LineRow.svelte` — chevron on `file_header` rows (inside the `{#if trailing}` block per CLAUDE.md UI patterns)
- `src/lib/HelpOverlay.svelte`, `docs/features.md` — shortcut + docs

## Norms
- `.line-action` class for the chevron button (CLAUDE.md UI patterns).
- Composables pattern; runes.

## Safeguards
- **Invariant: `lines` array is never mutated by collapse** — test: annotate line in file 3, collapse file 1, annotation still renders on the same content.
- Keyboard nav (j/k) must skip hidden rows without getting stuck.
- Search hits inside a collapsed file: either auto-expand on jump or skip — pick auto-expand (GitHub behavior), test it.

## Scope
- In: collapse/expand per file, auto-collapse threshold, summary header, selection/search interaction, docs.
- Out: viewed-state (parked), remembering collapse across sessions, S1's sidebar (independent — no edge between S1 and S2).
