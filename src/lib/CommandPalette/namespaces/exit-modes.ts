// Exit modes namespace for CommandPalette
// In-memory storage (no persistence yet)

import type { Namespace, Item } from '../engine/types';

export const exitModesNamespace: Namespace = {
  id: 'exit-modes',
  label: 'Exit Modes',
  icon: '📤',
  fields: [
    { key: 'name', label: 'Name', type: 'text', required: true },
    { key: 'instruction', label: 'Instruction', type: 'textarea', required: true },
  ],
  hotkeys: [
    { key: 's', label: 'set', action: 'SET' },
    { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
    { key: 'e', label: 'edit', action: 'EDIT' },
    { key: 'r', label: 'reorder', action: 'REORDER' },
  ],
  examples: [
    { name: 'Apply', instruction: 'Apply all changes exactly as annotated' },
    { name: 'Revise', instruction: 'Revise the approach based on feedback' },
    { name: 'Reject', instruction: 'Reject and start over with a different approach' },
    { name: 'Discuss', instruction: 'Need to discuss before proceeding' },
  ],
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
  if (!query) return items;
  const q = query.toLowerCase();
  return items.filter((item) => item.name.toLowerCase().includes(q));
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

// Generate ID from name (slug-style)
export function generateExitModeId(name: string): string {
  return name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
}
