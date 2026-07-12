// Namespace registry and QueryContext factory for CommandPalette

import type { QueryContext, Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';
import { tagsNamespace, getTagItems, filterTagItems } from './tags';
import { exitModesNamespace, getExitModeItems, filterExitModeItems } from './exit-modes';
import { copyNamespace, getCopyItems, filterCopyItems } from './copy';
import { saveNamespace, getSaveItems, filterSaveItems } from './save';
import { obsidianNamespace, getObsidianItems, filterObsidianItems } from './obsidian';
import { themeNamespace, getThemeItems, filterThemeItems } from './theme';
import { filesNamespace, getFileItems, filterFileItems } from './files';
import { viewNamespace, getViewItems, filterViewItems } from './view';

const namespaces: Namespace[] = [tagsNamespace, exitModesNamespace, filesNamespace, viewNamespace, copyNamespace, obsidianNamespace, saveNamespace, themeNamespace];

const getItemsMap: Record<string, () => Item[]> = {
  tags: getTagItems,
  files: getFileItems,
  view: getViewItems,
  'exit-modes': getExitModeItems,
  copy: getCopyItems,
  save: getSaveItems,
  obsidian: getObsidianItems,
  theme: getThemeItems,
};

const filterItemsMap: Record<string, (query: string) => Item[]> = {
  tags: filterTagItems,
  files: filterFileItems,
  view: filterViewItems,
  'exit-modes': filterExitModeItems,
  copy: filterCopyItems,
  save: filterSaveItems,
  obsidian: filterObsidianItems,
  theme: filterThemeItems,
};

/** Files and view modes only exist in diff sessions — don't surface empty namespaces elsewhere. */
function activeNamespaces(): Namespace[] {
  return namespaces.filter(
    (n) => (n.id !== 'files' || getFileItems().length > 0) && (n.id !== 'view' || getViewItems().length > 0),
  );
}

export function createQueryContext(): QueryContext {
  return {
    // Getter, not a snapshot: items are seeded after this context is built.
    get namespaces() {
      return activeNamespaces();
    },

    filterNamespaces(query: string): Namespace[] {
      return fuzzySearch(activeNamespaces(), query, [{ name: 'label', weight: 1 }]);
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
export { filesNamespace, getFileItems, setFileItems, filterFileItems } from './files';
export { viewNamespace, getViewItems, setViewItems, filterViewItems } from './view';
