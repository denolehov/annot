// Theme namespace for CommandPalette
// Action-only namespace — items emit theme change events

import type { Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';
import { SimpleItem } from '../items';

export const themeNamespace: Namespace = {
  id: 'theme',
  label: 'Theme',
  icon: 'sun',
  ItemComponent: SimpleItem,
  fields: [],
  hotkeys: [],
  capabilities: { delete: false },
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
  return fuzzySearch(themeItems, query, [{ name: 'name', weight: 1 }]);
}
