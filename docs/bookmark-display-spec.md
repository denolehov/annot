# Spec: Bookmark Display in Command Palette

> **Status**: Implemented

## Goal

Redesign bookmark display in the command palette using a slot-based component architecture. Each namespace owns its item rendering, enabling scalable visual diversity across item types.

---

## Design

### Architecture: Slot-based Item Components

Each namespace provides its own item component. No default fallback—all item types get explicit components.

```
CommandPalette.svelte
  └─ renders namespace.ItemComponent for each item
       ├─ BookmarkItem.svelte   (bookmarks namespace)
       └─ SimpleItem.svelte     (tags, exit-modes, theme, copy, save, obsidian)
```

### Bookmark Display: Two-Line Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ k3u  Cache strategy decision                                     │
│      cache-spec.md • Dec 30                                      │
└──────────────────────────────────────────────────────────────────┘
```

### Selection Bookmark (Phase 4)

```
┌──────────────────────────────────────────────────────────────────┐
│ t7x  let query = format!("SELECT…                                │
│      auth.rs:42-45 • Dec 28                                      │
└──────────────────────────────────────────────────────────────────┘
```

- Primary: ID + first ~50 chars of `selected_text` (or user label if set)
- Secondary: `{source_title}:{start_line}-{end_line}` + date

---

## Implementation

### Files Created

- `src/lib/CommandPalette/items/BookmarkItem.svelte` — Two-line bookmark display
- `src/lib/CommandPalette/items/SimpleItem.svelte` — Single-line item for other namespaces
- `src/lib/CommandPalette/items/index.ts` — Re-exports

### Files Modified

- `src/lib/CommandPalette/engine/types.ts` — Added `ItemComponent` to `Namespace` interface
- `src/lib/CommandPalette/namespaces/*.ts` — All namespaces now set `ItemComponent`
- `src/lib/CommandPalette/namespaces/bookmarks.ts` — Updated `bookmarkToItem()` to set `name` to just label
- `src/lib/CommandPalette/CommandPalette.svelte` — Uses dynamic component rendering
- `src/lib/CommandPalette/engine/*.test.ts` — Added mock `ItemComponent` for tests

---

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Architecture | Slot-based components | Scales as item types diverge |
| Default fallback | None | All namespaces explicit—no magic |
| ID display | 3 chars | Matches toast, sufficient uniqueness |
| Styles location | Component-scoped | Each item owns its layout |

---

## Scope

**In**:
- Two-line bookmark display with ID prefix
- `ItemComponent` field on `Namespace` interface
- Slot-based component architecture
- Selection bookmark format ready for Phase 4

**Out**:
- Context preview (tooltip/expandable)
- Bookmark grouping by project
- Custom components for tags/exit-modes (use SimpleItem for now)
