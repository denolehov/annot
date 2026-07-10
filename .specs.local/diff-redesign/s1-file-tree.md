---
id: S1
kind: story
wave: 0
depends_on: []
status: done
---

# [Spec]: S1 — File tree sidebar + palette fuzzy-jump

## Requirements
- **Problem:** In a big multi-file diff there is no way to see which files changed or jump to one — the anchor pain ("I get lost") in its purest form.
- **Beneficiary:** Anyone reviewing multi-file diffs; ships on the *current* model, no trunk dependency.
- **Done when:** In `pnpm demo:diff` (and a real multi-file `git_diff_args` session): sidebar toggles via shortcut, clicking a file scrolls to its header; `:` palette has a files namespace with fuzzy jump.

## Entities
N/A — consumes existing `DiffMetadata.files: DiffFileInfo[]` (`src/lib/types.ts:112-119`; `start_line`/`end_line` per file), which is currently computed and unused by any navigation.

## Approach

**Keystone:** navigation reads existing metadata; zero backend changes.
Rejected alternative: waiting for C1's per-file documents — weeks of delay for
data that's already on the wire.

- Sidebar: new `FileTree.svelte`, hidden by default, toggled by shortcut +
  palette action. Flat list with directory-prefix grouping (GitHub-style
  nested/collapsed-dir tree is a later nicety — flat first).
- Row: filename, dimmed dir prefix, +/− counts (derivable by counting
  added/deleted semantics lines within `start_line..end_line`).
- Click → scroll the line list to the file's `start_line` (same scroll
  mechanism the search feature uses — see `useSearch.svelte.ts`).
- Palette: new `files` namespace following the existing pattern
  (`src/lib/CommandPalette/namespaces/theme.ts` — `Namespace` + `Item[]` with
  `EMIT_EVENT` actions, `fuzzySearch` from `$lib/fuzzy`).
- Current-file tracking (highlight in tree while scrolling): IntersectionObserver
  or scroll-position → binary search over `start_line`s.

**Seams:**
- Refit at C1: tree rebinds from `metadata.files` to per-file documents; keep data access behind one derivation function so the refit touches one place.
- Rename display (`old → new`) arrives with B2/C1 data; flat name until then.

## Structure
- New: `src/lib/components/FileTree.svelte`
- New: `src/lib/CommandPalette/namespaces/files.ts` (+ register in `namespaces/index.ts`)
- `src/routes/+page.svelte` — layout slot for sidebar
- `src/lib/HelpOverlay.svelte` + `docs/features.md` — shortcut + feature docs (CLAUDE.md requires both)

## Norms
- Composables pattern for any state (`src/lib/composables/`).
- Frontend tests mock Tauri IPC via `vi.mock("@tauri-apps/api/core")`.
- Only render for `metadata.type === 'diff'`.

## Safeguards
- Sidebar must not steal keyboard focus from the line list (annot is keyboard-driven — verify j/k/selection still work with sidebar open).
- No layout shift of the line list content that would confuse in-flight selection.

## Scope
- In: sidebar, palette namespace, scroll-jump, current-file highlight, shortcut, help/docs updates.
- Out: collapse (S2), viewed-state (parked), nested dir tree, rename arrows (post-C1 refit).
