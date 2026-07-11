import { SvelteSet } from 'svelte/reactivity';
import type { DocView } from '$lib/display-rows';

/** Files with more changed lines than this start collapsed. */
export const AUTO_COLLAPSE_THRESHOLD = 500;

export interface FileCollapseOptions {
  /** Fired for each file transitioning expanded → collapsed (selection/hover cleanup). */
  onCollapse?: (dv: DocView) => void;
}

/**
 * Per-file collapse state for diff views.
 *
 * Presentation state only — collapse never touches the display walk; the
 * render layer skips collapsed sections. Display indexes stay stable under
 * toggle by construction.
 */
export function useFileCollapse(getDocs: () => DocView[], options: FileCollapseOptions = {}) {
  const collapsed = new SvelteSet<number>();

  function collapse(index: number) {
    if (collapsed.has(index)) return;
    collapsed.add(index);
    const dv = getDocs().find((d) => d.index === index);
    if (dv) options.onCollapse?.(dv);
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
      for (const dv of getDocs()) collapse(dv.index);
    },
    expandAll() {
      collapsed.clear();
    },
    /** Seed auto-collapsed files after content load. Fires no onCollapse — nothing to clean up yet. */
    init() {
      for (const dv of getDocs()) {
        if (dv.added + dv.deleted > AUTO_COLLAPSE_THRESHOLD) collapsed.add(dv.index);
      }
    },
  };
}

export type FileCollapse = ReturnType<typeof useFileCollapse>;
