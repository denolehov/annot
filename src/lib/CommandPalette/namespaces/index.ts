// Namespace registry and QueryContext factory for CommandPalette

import type { QueryContext, Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';
import { tagsNamespace, getTagItems, filterTagItems } from './tags';
import { exitModesNamespace, getExitModeItems, filterExitModeItems } from './exit-modes';
import { copyNamespace, getCopyItems, filterCopyItems } from './copy';
import { saveNamespace, getSaveItems, filterSaveItems } from './save';
import { obsidianNamespace, getObsidianItems, filterObsidianItems } from './obsidian';
import { themeNamespace, getThemeItems, filterThemeItems } from './theme';

const namespaces: Namespace[] = [tagsNamespace, exitModesNamespace, copyNamespace, obsidianNamespace, saveNamespace, themeNamespace];

const getItemsMap: Record<string, () => Item[]> = {
  tags: getTagItems,
  'exit-modes': getExitModeItems,
  copy: getCopyItems,
  save: getSaveItems,
  obsidian: getObsidianItems,
  theme: getThemeItems,
};

const filterItemsMap: Record<string, (query: string) => Item[]> = {
  tags: filterTagItems,
  'exit-modes': filterExitModeItems,
  copy: filterCopyItems,
  save: filterSaveItems,
  obsidian: filterObsidianItems,
  theme: filterThemeItems,
};

export function createQueryContext(): QueryContext {
  return {
    namespaces,

    filterNamespaces(query: string): Namespace[] {
      return fuzzySearch(namespaces, query, [{ name: 'label', weight: 1 }]);
    },

    getItems(namespace: Namespace) {
      return getItemsMap[namespace.id]?.() ?? [];
    },

    filterItems(namespace: Namespace, query: string) {
      return filterItemsMap[namespace.id]?.(query) ?? [];
    },
  };
}

// Re-export namespace modules for direct item manipulation
export { tagsNamespace, getTagItems, setTagItems, filterTagItems, saveTagItem, deleteTagItem, generateTagId } from './tags';
export { exitModesNamespace, getExitModeItems, setExitModeItems, filterExitModeItems, saveExitModeItem, deleteExitModeItem, reorderExitModeItems, generateExitModeId } from './exit-modes';
export { copyNamespace, getCopyItems, filterCopyItems } from './copy';
export { saveNamespace, getSaveItems, filterSaveItems } from './save';
export { obsidianNamespace, getObsidianItems, filterObsidianItems, setObsidianVaults, saveObsidianVault, deleteObsidianVault, getVaultNames, generateVaultId, getRawVaultItems } from './obsidian';
export { themeNamespace, getThemeItems, filterThemeItems } from './theme';
