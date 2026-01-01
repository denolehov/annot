// Bookmarks namespace for CommandPalette
// Stores captured moments of attention for later reference

import type { Namespace, Item } from '../engine/types';
import type { Bookmark } from '$lib/types';
import { fuzzySearch } from '$lib/fuzzy';
import { invoke } from '@tauri-apps/api/core';
import { BookmarkItem } from '../items';

export const bookmarksNamespace: Namespace = {
  id: 'bookmarks',
  label: 'Bookmarks',
  icon: 'bookmark',
  ItemComponent: BookmarkItem,
  fields: [{ key: 'label', label: 'Label', type: 'text', required: true }],
  hotkeys: [
    { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
    { key: 'e', label: 'edit', action: 'EDIT' },
  ],
  examples: [],
  capabilities: { create: false },
};

// In-memory storage
let items: Item[] = [];

export function getBookmarkItems(): Item[] {
  return items;
}

export function setBookmarkItems(data: Item[]): void {
  items = data;
}

export function filterBookmarkItems(query: string): Item[] {
  return fuzzySearch(items, query, [
    { name: 'id', weight: 3 }, // ID prefix highest priority
    { name: 'name', weight: 2 }, // Label
    { name: 'values.source_title', weight: 1 },
  ]);
}

/** Convert a Bookmark from the backend to a command palette Item. */
export function bookmarkToItem(bookmark: Bookmark): Item {
  // Label derivation: user-set label, or source title as fallback
  const displayLabel = bookmark.label ?? bookmark.snapshot.source_title;

  return {
    id: bookmark.id,
    name: displayLabel, // Just the label for display and search
    values: {
      label: bookmark.label ?? '',
      source_title: bookmark.snapshot.source_title,
      created_at: bookmark.created_at,
      project_path: bookmark.project_path ?? '',
    },
  };
}

/** Load bookmarks from the backend and populate the in-memory store. */
export async function loadBookmarks(): Promise<void> {
  try {
    const bookmarks = await invoke<Bookmark[]>('get_bookmarks');
    items = bookmarks.map(bookmarkToItem);
  } catch (e) {
    console.error('Failed to load bookmarks:', e);
    items = [];
  }
}

/** Save a bookmark item (update label). */
export async function saveBookmarkItem(item: Item): Promise<void> {
  try {
    await invoke('update_bookmark', {
      id: item.id,
      label: item.values.label || item.name,
    });
    await loadBookmarks(); // Reload to sync
  } catch (e) {
    console.error('Failed to save bookmark:', e);
    throw e;
  }
}

/** Delete a bookmark by ID. */
export async function deleteBookmarkItem(id: string): Promise<void> {
  // Optimistically remove from UI first (like tags)
  items = items.filter((i) => i.id !== id);
  try {
    await invoke('delete_bookmark', { id });
  } catch (e) {
    console.error('Failed to delete bookmark:', e);
    // Could restore item here on failure, but for now just log
    throw e;
  }
}

/** Create a new bookmark for the current session. */
export async function createBookmark(label?: string): Promise<Bookmark> {
  const bookmark = await invoke<Bookmark>('create_bookmark', { label: label ?? null });
  await loadBookmarks(); // Reload to sync
  return bookmark;
}
