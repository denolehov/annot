// Copy namespace for CommandPalette
// Action-only namespace — items execute on selection, no CRUD

import type { Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';

export const copyNamespace: Namespace = {
  id: 'copy',
  label: 'Copy',
  icon: 'copy',
  fields: [], // No fields — not editable
  hotkeys: [], // No hotkeys for action namespaces
};

// Static items — execute copy_to_clipboard on selection
export const copyItems: Item[] = [
  {
    id: 'copy-content',
    name: 'Content',
    values: {},
    action: { type: 'COPY_TO_CLIPBOARD', mode: 'content' },
  },
  {
    id: 'copy-annotations',
    name: 'Annotations',
    values: {},
    action: { type: 'COPY_TO_CLIPBOARD', mode: 'annotations' },
  },
  {
    id: 'copy-both',
    name: 'Both',
    values: {},
    action: { type: 'COPY_TO_CLIPBOARD', mode: 'all' },
  },
];

export function getCopyItems(): Item[] {
  return copyItems;
}

export function filterCopyItems(query: string): Item[] {
  return fuzzySearch(copyItems, query, [{ name: 'name', weight: 1 }]);
}
