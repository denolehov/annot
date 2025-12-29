// Theme namespace for CommandPalette
// Action-only namespace — items emit theme change events

import type { Namespace, Item } from '../engine/types';

export const themeNamespace: Namespace = {
  id: 'theme',
  label: 'Theme',
  icon: 'sun',
  fields: [], // No fields — not editable
  hotkeys: [], // No hotkeys for action namespaces
};

// Static items — emit SET_THEME event on selection
export const themeItems: Item[] = [
  {
    id: 'theme-system',
    name: 'System',
    values: {},
    action: { type: 'EMIT_EVENT', event: 'SET_THEME', payload: 'system' },
  },
  {
    id: 'theme-light',
    name: 'Light',
    values: {},
    action: { type: 'EMIT_EVENT', event: 'SET_THEME', payload: 'light' },
  },
  {
    id: 'theme-dark',
    name: 'Dark',
    values: {},
    action: { type: 'EMIT_EVENT', event: 'SET_THEME', payload: 'dark' },
  },
];

export function getThemeItems(): Item[] {
  return themeItems;
}

export function filterThemeItems(query: string): Item[] {
  if (!query) return themeItems;
  const q = query.toLowerCase();
  return themeItems.filter((item) => item.name.toLowerCase().includes(q));
}
