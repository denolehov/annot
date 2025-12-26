import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushSync } from 'svelte';

// Mock @tauri-apps/api/core before importing the composable
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { useExitModes } from './useExitModes.svelte';
import { invoke } from '@tauri-apps/api/core';

describe('useExitModes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts with empty modes and null selected index', () => {
    const state = useExitModes();
    expect(state.modes).toEqual([]);
    expect(state.selectedIndex).toBeNull();
    expect(state.selectedMode).toBeNull();
  });

  it('initializes with modes and selected mode', () => {
    const state = useExitModes();
    const modes = [
      { id: 'apply', name: 'Apply', color: 'green', instruction: '', order: 0, origin: 'persisted' as const },
      { id: 'reject', name: 'Reject', color: 'red', instruction: '', order: 1, origin: 'persisted' as const },
    ];

    flushSync(() => {
      state.initialize(modes, 'reject');
    });

    expect(state.modes).toEqual(modes);
    expect(state.selectedIndex).toBe(1);
    expect(state.selectedMode?.id).toBe('reject');
  });

  it('cycles forward through modes', () => {
    const state = useExitModes();
    const modes = [
      { id: 'a', name: 'A', color: 'blue', instruction: '', order: 0, origin: 'persisted' as const },
      { id: 'b', name: 'B', color: 'red', instruction: '', order: 1, origin: 'persisted' as const },
    ];

    flushSync(() => {
      state.initialize(modes, null);
    });

    expect(state.selectedIndex).toBeNull();

    flushSync(() => {
      state.cycleForward();
    });
    expect(state.selectedIndex).toBe(0);
    expect(invoke).toHaveBeenCalledWith('set_exit_mode', { modeId: 'a' });

    flushSync(() => {
      state.cycleForward();
    });
    expect(state.selectedIndex).toBe(1);

    flushSync(() => {
      state.cycleForward();
    });
    expect(state.selectedIndex).toBeNull(); // wraps to null
  });

  it('cycles backward through modes', () => {
    const state = useExitModes();
    const modes = [
      { id: 'a', name: 'A', color: 'blue', instruction: '', order: 0, origin: 'persisted' as const },
      { id: 'b', name: 'B', color: 'red', instruction: '', order: 1, origin: 'persisted' as const },
    ];

    flushSync(() => {
      state.initialize(modes, null);
    });

    flushSync(() => {
      state.cycleBackward();
    });
    expect(state.selectedIndex).toBe(1); // null → last

    flushSync(() => {
      state.cycleBackward();
    });
    expect(state.selectedIndex).toBe(0);

    flushSync(() => {
      state.cycleBackward();
    });
    expect(state.selectedIndex).toBeNull(); // wraps to null
  });

  it('selects mode by id', () => {
    const state = useExitModes();
    const modes = [
      { id: 'a', name: 'A', color: 'blue', instruction: '', order: 0, origin: 'persisted' as const },
      { id: 'b', name: 'B', color: 'red', instruction: '', order: 1, origin: 'persisted' as const },
    ];

    flushSync(() => {
      state.initialize(modes, null);
      state.selectById('b');
    });

    expect(state.selectedIndex).toBe(1);
    expect(invoke).toHaveBeenCalledWith('set_exit_mode', { modeId: 'b' });
  });

  it('updates modes with setModes', () => {
    const state = useExitModes();
    const modes = [
      { id: 'a', name: 'A', color: 'blue', instruction: '', order: 0, origin: 'persisted' as const },
      { id: 'b', name: 'B', color: 'red', instruction: '', order: 1, origin: 'persisted' as const },
    ];

    flushSync(() => {
      state.initialize(modes, 'b');
    });
    expect(state.selectedIndex).toBe(1);

    // Remove the selected mode
    flushSync(() => {
      state.setModes([modes[0]]);
    });

    expect(state.modes.length).toBe(1);
    expect(state.selectedIndex).toBe(0); // clamped to last available
  });
});
