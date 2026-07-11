// Files namespace for CommandPalette
// Action-only namespace — items jump the viewport to a file in the diff

import type { Namespace, Item } from '../engine/types';
import type { DocView } from '$lib/display-rows';
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

// Seeded from the session's diff display walk; empty for non-diff content
let fileItems: Item[] = [];

export function setFileItems(docs: DocView[]): void {
  fileItems = docs.map((dv) => ({
    id: `file-${dv.index}`,
    name: dv.path,
    values: {},
    action: { type: 'EMIT_EVENT' as const, event: 'JUMP_TO_FILE', payload: dv.headerDisplayIndex },
  }));
}

export function getFileItems(): Item[] {
  return fileItems;
}

export function filterFileItems(query: string): Item[] {
  return fuzzySearch(fileItems, query, [{ name: 'name', weight: 1 }]);
}
