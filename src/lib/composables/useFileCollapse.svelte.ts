import { SvelteSet } from 'svelte/reactivity';
import type { FileEntry } from '$lib/file-tree';
import { autoCollapsedIndices } from '$lib/file-collapse';

export interface FileCollapseOptions {
  /** Fired for each file transitioning expanded → collapsed (selection/hover cleanup). */
  onCollapse?: (entry: FileEntry) => void;
}

/**
 * Per-file collapse state for diff views.
 *
 * Presentation state only — collapse never touches the lines array; the render
 * layer skips collapsed sections. If collapse ever becomes structural, only
 * that render-skip mechanism gets replaced.
 */
export function useFileCollapse(getEntries: () => FileEntry[], options: FileCollapseOptions = {}) {
  const collapsed = new SvelteSet<number>();

  function collapse(index: number) {
    if (collapsed.has(index)) return;
    collapsed.add(index);
    const entry = getEntries().find((e) => e.index === index);
    if (entry) options.onCollapse?.(entry);
  }

  return {
    get anyCollapsed() {
      return collapsed.size > 0;
    },
    isCollapsed(index: number): boolean {
      return collapsed.has(index);
    },
    collapse,
    expand(index: number) {
      collapsed.delete(index);
    },
    toggle(index: number) {
      if (collapsed.has(index)) {
        collapsed.delete(index);
      } else {
        collapse(index);
      }
    },
    collapseAll() {
      for (const entry of getEntries()) collapse(entry.index);
    },
    expandAll() {
      collapsed.clear();
    },
    /** Seed auto-collapsed files after content load. Fires no onCollapse — nothing to clean up yet. */
    init() {
      for (const index of autoCollapsedIndices(getEntries())) collapsed.add(index);
    },
  };
}

export type FileCollapse = ReturnType<typeof useFileCollapse>;
