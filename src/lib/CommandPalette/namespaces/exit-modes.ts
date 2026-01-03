// Exit modes namespace for CommandPalette
// In-memory storage (no persistence yet)

import type { Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';
import { SimpleItem } from '../items';
import { generateId } from '$lib/utils/id';

export const exitModesNamespace: Namespace = {
  id: 'exit-modes',
  label: 'Exit Modes',
  icon: 'exit',
  ItemComponent: SimpleItem,
  fields: [
    { key: 'name', label: 'Name', type: 'text', required: true },
    { key: 'instruction', label: 'Instruction', type: 'textarea', required: true },
  ],
  hotkeys: [
    { key: 's', label: 'set', action: 'SET' },
    { key: 'e', label: 'edit', action: 'EDIT' },
    { key: 'r', label: 'reorder', action: 'REORDER' },
  ],
  examples: [
    { name: 'Apply', instruction: 'Apply all changes exactly as annotated' },
    { name: 'Revise', instruction: 'Revise the approach based on feedback' },
    { name: 'Reject', instruction: 'Reject and start over with a different approach' },
    { name: 'Discuss', instruction: 'Need to discuss before proceeding' },
  ],
  capabilities: { reorder: true },
};

// In-memory storage
let items: Item[] = [];

export function getExitModeItems(): Item[] {
  return items;
}

export function setExitModeItems(data: Item[]): void {
  items = data;
}

export function filterExitModeItems(query: string): Item[] {
  return fuzzySearch(items, query, [{ name: 'name', weight: 1 }]);
}

export function saveExitModeItem(item: Item): void {
  const idx = items.findIndex((i) => i.id === item.id);
  if (idx >= 0) {
    items[idx] = item;
  } else {
    items.push(item);
  }
  items = [...items];
}

export function deleteExitModeItem(id: string): void {
  items = items.filter((i) => i.id !== id);
}

export function reorderExitModeItems(orderedIds: string[]): void {
  const itemMap = new Map(items.map((i) => [i.id, i]));
  items = orderedIds.map((id) => itemMap.get(id)!).filter(Boolean);
}

// jj-style ID generator (name parameter kept for backwards compat, but ignored)
export function generateExitModeId(_name?: string): string {
  return generateId();
}
