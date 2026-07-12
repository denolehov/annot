// View namespace for CommandPalette
// Action-only namespace — items switch the diff view projection (unified/split)

import type { Namespace, Item } from '../engine/types';
import type { DiffViewMode } from '$lib/display-rows';
import { fuzzySearch } from '$lib/fuzzy';
import { SimpleItem } from '../items';

export const viewNamespace: Namespace = {
  id: 'view',
  label: 'View',
  icon: 'columns',
  ItemComponent: SimpleItem,
  fields: [],
  hotkeys: [],
  capabilities: { delete: false },
};

// Seeded from the session's diff view mode; empty for non-diff content, which
// hides the namespace (same mechanism as `files`).
let viewItems: Item[] = [];

export function setViewItems(mode: DiffViewMode | null): void {
  viewItems =
    mode === null
      ? []
      : (['unified', 'split'] as const).map((m) => ({
          id: `view-${m}`,
          name: m === 'unified' ? 'Unified view' : 'Split view',
          values: {},
          action: { type: 'EMIT_EVENT' as const, event: 'SET_DIFF_VIEW', payload: m },
        }));
}

export function getViewItems(): Item[] {
  return viewItems;
}

export function filterViewItems(query: string): Item[] {
  return fuzzySearch(viewItems, query, [{ name: 'name', weight: 1 }]);
}
