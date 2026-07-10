// Files namespace for CommandPalette
// Action-only namespace — items jump the viewport to a file in the diff

import type { Namespace, Item } from '../engine/types';
import type { FileEntry } from '$lib/file-tree';
import { fuzzySearch } from '$lib/fuzzy';
import { SimpleItem } from '../items';

export const filesNamespace: Namespace = {
  id: 'files',
  label: 'Files',
  icon: 'file',
  ItemComponent: SimpleItem,
  fields: [],
  hotkeys: [],
  capabilities: { delete: false },
};

// Seeded from the session's diff metadata; empty for non-diff content
let fileItems: Item[] = [];

export function setFileItems(entries: FileEntry[]): void {
  fileItems = entries.map((entry) => ({
    id: `file-${entry.index}`,
    name: entry.path,
    values: {},
    action: { type: 'EMIT_EVENT' as const, event: 'JUMP_TO_FILE', payload: entry.startLine },
  }));
}

export function getFileItems(): Item[] {
  return fileItems;
}

export function filterFileItems(query: string): Item[] {
  return fuzzySearch(fileItems, query, [{ name: 'name', weight: 1 }]);
}
