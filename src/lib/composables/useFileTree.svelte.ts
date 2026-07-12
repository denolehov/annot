import { SvelteSet } from 'svelte/reactivity';

/**
 * Sidebar visibility and per-directory tree expand state. Hidden/all-expanded
 * by default; neither is persisted past the review session.
 */
export function useFileTree() {
  let isOpen = $state(false);
  const collapsedDirs = new SvelteSet<string>();

  return {
    get isOpen() { return isOpen; },
    toggle() { isOpen = !isOpen; },
    open() { isOpen = true; },
    close() { isOpen = false; },

    /** Absence from the set means expanded — matches the tree's default-open shape. */
    isDirExpanded(path: string): boolean {
      return !collapsedDirs.has(path);
    },
    toggleDir(path: string) {
      if (collapsedDirs.has(path)) collapsedDirs.delete(path);
      else collapsedDirs.add(path);
    },
  };
}
