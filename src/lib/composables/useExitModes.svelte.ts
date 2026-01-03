import { invoke } from '@tauri-apps/api/core';
import type { ExitMode } from '$lib/types';

export function useExitModes() {
  let modes: ExitMode[] = $state([]);
  let selectedIndex: number | null = $state(null);

  const selectedMode = $derived(
    selectedIndex !== null && modes.length > 0 ? modes[selectedIndex] : null
  );

  function initialize(initialModes: ExitMode[], selectedId: string | null) {
    modes = initialModes;
    if (selectedId) {
      const idx = initialModes.findIndex(m => m.id === selectedId);
      if (idx >= 0) selectedIndex = idx;
    }
  }

  function cycleForward(): void {
    if (modes.length === 0) return;
    if (selectedIndex === null) {
      selectedIndex = 0;
    } else if (selectedIndex === modes.length - 1) {
      selectedIndex = null;
    } else {
      selectedIndex = selectedIndex + 1;
    }
    syncToBackend();
  }

  function cycleBackward(): void {
    if (modes.length === 0) return;
    if (selectedIndex === null) {
      selectedIndex = modes.length - 1;
    } else if (selectedIndex === 0) {
      selectedIndex = null;
    } else {
      selectedIndex = selectedIndex - 1;
    }
    syncToBackend();
  }

  function selectById(modeId: string): void {
    const idx = modes.findIndex(m => m.id === modeId);
    if (idx >= 0) {
      selectedIndex = idx;
      syncToBackend();
    }
  }

  function syncToBackend(): void {
    const modeId = selectedIndex !== null ? modes[selectedIndex].id : null;
    invoke('set_exit_mode', { modeId });
  }

  function setModes(newModes: ExitMode[]): void {
    modes = newModes;
    // Clamp selected index if mode was deleted
    if (selectedIndex !== null && selectedIndex >= modes.length) {
      selectedIndex = modes.length > 0 ? modes.length - 1 : null;
    }
  }

  return {
    get modes() { return modes; },
    get selectedIndex() { return selectedIndex; },
    set selectedIndex(val: number | null) { selectedIndex = val; },
    get selectedMode() { return selectedMode; },
    initialize,
    cycleForward,
    cycleBackward,
    selectById,
    setModes,
    syncToBackend,
  };
}
