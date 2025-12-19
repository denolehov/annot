// Tags namespace for CommandPalette
// In-memory storage (no persistence yet)

import type { Namespace, Item } from '../engine/types';

export const tagsNamespace: Namespace = {
  id: 'tags',
  label: 'Tags',
  icon: '#',  // Text icon, not emoji
  fields: [
    { key: 'name', label: 'Name', type: 'text', required: true },
    { key: 'instruction', label: 'Instruction', type: 'textarea', required: false },
  ],
  hotkeys: [
    { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
    { key: 'e', label: 'edit', action: 'EDIT' },
  ],
  examples: [
    { name: 'TODO', instruction: 'Mark for future implementation' },
    { name: 'SECURITY', instruction: 'Security-sensitive code requiring careful review' },
    { name: 'REFACTOR', instruction: 'Code that should be refactored or cleaned up' },
    { name: 'BUG', instruction: 'Potential bug or issue to investigate' },
    { name: 'PERF', instruction: 'Performance optimization opportunity' },
  ],
};

// In-memory storage
let items: Item[] = [];

export function getTagItems(): Item[] {
  return items;
}

export function setTagItems(data: Item[]): void {
  items = data;
}

export function filterTagItems(query: string): Item[] {
  if (!query) return items;
  const q = query.toLowerCase();
  return items.filter((item) => item.name.toLowerCase().includes(q));
}

export function saveTagItem(item: Item): void {
  const idx = items.findIndex((i) => i.id === item.id);
  if (idx >= 0) {
    items[idx] = item;
  } else {
    items.push(item);
  }
  items = [...items]; // Trigger reactivity if needed
}

export function deleteTagItem(id: string): void {
  items = items.filter((i) => i.id !== id);
}

// Generate a simple ID for new items
export function generateTagId(): string {
  return Math.random().toString(36).substring(2, 14);
}
