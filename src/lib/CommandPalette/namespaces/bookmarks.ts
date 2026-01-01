// Bookmarks namespace for CommandPalette
// Stores captured moments of attention for later reference

import type { Namespace, Item } from '../engine/types';
import type { Bookmark } from '$lib/types';
import { fuzzySearch } from '$lib/fuzzy';
import { BookmarkItem } from '../items';

export const bookmarksNamespace: Namespace = {
  id: 'bookmarks',
  label: 'Bookmarks',
  icon: 'bookmark',
  ItemComponent: BookmarkItem,
  fields: [{ key: 'label', label: 'Label', type: 'text', required: true, placeholder: 'Enter a label...' }],
  hotkeys: [{ key: 'e', label: 'edit', action: 'EDIT' }],
  examples: [],
  capabilities: { create: false },
};

// In-memory storage
let items: Item[] = [];

export function getBookmarkItems(): Item[] {
  // Sort by created_at descending (newest first)
  return [...items].sort((a, b) =>
    (b.values.created_at as string).localeCompare(a.values.created_at as string)
  );
}

export function setBookmarkItems(data: Item[]): void {
  items = data;
}

export function filterBookmarkItems(query: string): Item[] {
  // Sort by created_at descending before filtering (fuzzy search preserves relevance order)
  const sorted = [...items].sort((a, b) =>
    (b.values.created_at as string).localeCompare(a.values.created_at as string)
  );
  return fuzzySearch(sorted, query, [
    { name: 'name', weight: 2 }, // Label
    { name: 'id', weight: 1 },
  ]);
}

/** Truncate string to max length with ellipsis. */
function truncate(str: string, maxLen: number): string {
  return str.length > maxLen ? str.slice(0, maxLen) + '…' : str;
}

/** Extract first markdown heading from content. */
function extractFirstHeading(content: string): string | null {
  const line = content.split('\n').find(l => l.startsWith('#'));
  return line ? line.replace(/^#+\s*/, '').trim() : null;
}

/** Get display label for a bookmark (user label, or derived from content). */
function getDisplayLabel(bookmark: Bookmark): string {
  if (bookmark.label) {
    return bookmark.label;
  }
  // Derive from content when no user label
  if (bookmark.snapshot.type === 'selection') {
    const firstLine = bookmark.snapshot.selected_text.split('\n')[0];
    return truncate(firstLine, 50);
  }
  // Session bookmark: try heading for .md, else source_title
  if (bookmark.snapshot.source_title.endsWith('.md')) {
    const heading = extractFirstHeading(bookmark.snapshot.context);
    if (heading) {
      return truncate(heading, 50);
    }
  }
  return bookmark.snapshot.source_title;
}

/** Convert a Bookmark from the backend to a command palette Item. */
export function bookmarkToItem(bookmark: Bookmark): Item {
  const displayLabel = getDisplayLabel(bookmark);

  return {
    id: bookmark.id,
    name: displayLabel, // For display and search
    values: {
      label: bookmark.label ?? '', // User-set label only (empty for selection bookmarks by default)
      source_title: bookmark.snapshot.source_title,
      created_at: bookmark.created_at,
      project_path: bookmark.project_path ?? '',
    },
  };
}

/** Optimistically update a bookmark item in local store (IPC handled by callback). */
export function saveBookmarkItem(item: Item): void {
  items = items.map((i) =>
    i.id === item.id
      ? { ...i, name: item.values.label || item.name, values: { ...i.values, label: item.values.label } }
      : i
  );
}

/** Optimistically remove a bookmark from local store (IPC handled by callback). */
export function deleteBookmarkItem(id: string): void {
  items = items.filter((i) => i.id !== id);
}
