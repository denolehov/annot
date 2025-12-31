// Save namespace for CommandPalette
// Action-only namespace — opens save modal on selection

import type { Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';

export const saveNamespace: Namespace = {
  id: 'save',
  label: 'Save',
  icon: 'save',
  fields: [],
  hotkeys: [],
  capabilities: { delete: false },
};

// Single item — opens save modal
export const saveItems: Item[] = [
  {
    id: 'save-to-file',
    name: 'Save to file',
    values: {},
    action: { type: 'OPEN_SAVE_MODAL' },
  },
];

export function getSaveItems(): Item[] {
  return saveItems;
}

export function filterSaveItems(query: string): Item[] {
  return fuzzySearch(saveItems, query, [{ name: 'name', weight: 1 }]);
}
