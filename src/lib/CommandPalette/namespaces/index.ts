// Namespace registry and QueryContext factory for CommandPalette

import type { QueryContext, Namespace } from '../engine/types';
import { tagsNamespace, getTagItems, filterTagItems } from './tags';
import { exitModesNamespace, getExitModeItems, filterExitModeItems } from './exit-modes';
import { copyNamespace, getCopyItems, filterCopyItems } from './copy';

const namespaces: Namespace[] = [tagsNamespace, exitModesNamespace, copyNamespace];

export function createQueryContext(): QueryContext {
  return {
    namespaces,

    filterNamespaces(query: string): Namespace[] {
      if (!query) return namespaces;
      const q = query.toLowerCase();
      return namespaces.filter((ns) => ns.label.toLowerCase().includes(q));
    },

    getItems(namespace: Namespace) {
      if (namespace.id === 'tags') return getTagItems();
      if (namespace.id === 'exit-modes') return getExitModeItems();
      if (namespace.id === 'copy') return getCopyItems();
      return [];
    },

    filterItems(namespace: Namespace, query: string) {
      if (namespace.id === 'tags') return filterTagItems(query);
      if (namespace.id === 'exit-modes') return filterExitModeItems(query);
      if (namespace.id === 'copy') return filterCopyItems(query);
      return [];
    },
  };
}

// Re-export namespace modules for direct item manipulation
export { tagsNamespace, getTagItems, setTagItems, filterTagItems, saveTagItem, deleteTagItem, generateTagId } from './tags';
export { exitModesNamespace, getExitModeItems, setExitModeItems, filterExitModeItems, saveExitModeItem, deleteExitModeItem, reorderExitModeItems, generateExitModeId } from './exit-modes';
export { copyNamespace, getCopyItems, filterCopyItems } from './copy';
