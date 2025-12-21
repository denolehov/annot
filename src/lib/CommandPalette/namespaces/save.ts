// Save namespace for CommandPalette
// Action-only namespace — opens save modal on selection

import type { Namespace, Item } from '../engine/types';

export const saveNamespace: Namespace = {
  id: 'save',
  label: 'Save',
  icon: 'save',
  fields: [], // No fields — not editable
  hotkeys: [], // No hotkeys for action namespaces
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
  if (!query) return saveItems;
  const q = query.toLowerCase();
  return saveItems.filter((item) => item.name.toLowerCase().includes(q));
}
