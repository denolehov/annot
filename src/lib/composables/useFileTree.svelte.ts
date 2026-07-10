/** Sidebar visibility. Hidden by default; not persisted across sessions. */
export function useFileTree() {
  let isOpen = $state(false);

  return {
    get isOpen() { return isOpen; },
    toggle() { isOpen = !isOpen; },
    open() { isOpen = true; },
    close() { isOpen = false; },
  };
}
