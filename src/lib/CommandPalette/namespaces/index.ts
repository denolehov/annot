// Namespace registry and QueryContext factory for CommandPalette

import type { QueryContext, Namespace } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';
import { tagsNamespace, getTagItems, filterTagItems } from './tags';
import { exitModesNamespace, getExitModeItems, filterExitModeItems } from './exit-modes';
import { bookmarksNamespace, getBookmarkItems, filterBookmarkItems } from './bookmarks';
import { copyNamespace, getCopyItems, filterCopyItems } from './copy';
import { saveNamespace, getSaveItems, filterSaveItems } from './save';
import { obsidianNamespace, getObsidianItems, filterObsidianItems } from './obsidian';
import { themeNamespace, getThemeItems, filterThemeItems } from './theme';

const namespaces: Namespace[] = [tagsNamespace, exitModesNamespace, bookmarksNamespace, copyNamespace, obsidianNamespace, saveNamespace, themeNamespace];

export function createQueryContext(): QueryContext {
  return {
    namespaces,

    filterNamespaces(query: string): Namespace[] {
      return fuzzySearch(namespaces, query, [{ name: 'label', weight: 1 }]);
    },

    getItems(namespace: Namespace) {
      if (namespace.id === 'tags') return getTagItems();
      if (namespace.id === 'exit-modes') return getExitModeItems();
      if (namespace.id === 'bookmarks') return getBookmarkItems();
      if (namespace.id === 'copy') return getCopyItems();
      if (namespace.id === 'save') return getSaveItems();
      if (namespace.id === 'obsidian') return getObsidianItems();
      if (namespace.id === 'theme') return getThemeItems();
      return [];
    },

    filterItems(namespace: Namespace, query: string) {
      if (namespace.id === 'tags') return filterTagItems(query);
      if (namespace.id === 'exit-modes') return filterExitModeItems(query);
      if (namespace.id === 'bookmarks') return filterBookmarkItems(query);
      if (namespace.id === 'copy') return filterCopyItems(query);
      if (namespace.id === 'save') return filterSaveItems(query);
      if (namespace.id === 'obsidian') return filterObsidianItems(query);
      if (namespace.id === 'theme') return filterThemeItems(query);
      return [];
    },
  };
}

// Re-export namespace modules for direct item manipulation
export { tagsNamespace, getTagItems, setTagItems, filterTagItems, saveTagItem, deleteTagItem, generateTagId } from './tags';
export { exitModesNamespace, getExitModeItems, setExitModeItems, filterExitModeItems, saveExitModeItem, deleteExitModeItem, reorderExitModeItems, generateExitModeId } from './exit-modes';
export { bookmarksNamespace, getBookmarkItems, setBookmarkItems, filterBookmarkItems, saveBookmarkItem, deleteBookmarkItem, loadBookmarks, createBookmark, bookmarkToItem } from './bookmarks';
export { copyNamespace, getCopyItems, filterCopyItems } from './copy';
export { saveNamespace, getSaveItems, filterSaveItems } from './save';
export { obsidianNamespace, getObsidianItems, filterObsidianItems, setObsidianVaults, saveObsidianVault, deleteObsidianVault, getVaultNames, generateVaultId, getRawVaultItems } from './obsidian';
export { themeNamespace, getThemeItems, filterThemeItems } from './theme';
