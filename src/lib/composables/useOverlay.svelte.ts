import type { State as CommandPaletteState } from '$lib/CommandPalette/engine/types';

/**
 * Discriminated union for overlay state.
 * Only one overlay can be open at a time — impossible states are unrepresentable.
 */
export type OverlayState =
  | { type: 'NONE' }
  | { type: 'COMMAND_PALETTE'; state: CommandPaletteState; initialNamespace?: 'exit-modes' }
  | { type: 'HELP' }
  | { type: 'TIMELINE'; selectedIndex: number; idBuffer: string };

export type OverlayAction =
  | { type: 'OPEN_COMMAND_PALETTE'; initialNamespace?: 'exit-modes' }
  | { type: 'OPEN_HELP' }
  | { type: 'OPEN_TIMELINE' }
  | { type: 'CLOSE' }
  | { type: 'UPDATE_COMMAND_PALETTE'; state: CommandPaletteState }
  | { type: 'TIMELINE_MOVE'; index: number }
  | { type: 'TIMELINE_TYPE'; char: string }
  | { type: 'TIMELINE_CLEAR_BUFFER' };

/**
 * Pure reducer for overlay state transitions.
 */
export function overlayReducer(state: OverlayState, action: OverlayAction): OverlayState {
  switch (action.type) {
    case 'OPEN_COMMAND_PALETTE':
      // Can only open if nothing is open
      if (state.type !== 'NONE') return state;
      // Command palette starts in IDLE and opens via its own OPEN action
      return {
        type: 'COMMAND_PALETTE',
        state: { type: 'IDLE' },
        initialNamespace: action.initialNamespace,
      };

    case 'OPEN_HELP':
      if (state.type !== 'NONE') return state;
      return { type: 'HELP' };

    case 'OPEN_TIMELINE':
      if (state.type !== 'NONE') return state;
      return { type: 'TIMELINE', selectedIndex: 0, idBuffer: '' };

    case 'CLOSE':
      return { type: 'NONE' };

    case 'UPDATE_COMMAND_PALETTE':
      if (state.type !== 'COMMAND_PALETTE') return state;
      // If command palette state becomes IDLE, close the overlay
      if (action.state.type === 'IDLE') {
        return { type: 'NONE' };
      }
      return { ...state, state: action.state };

    case 'TIMELINE_MOVE':
      if (state.type !== 'TIMELINE') return state;
      return { ...state, selectedIndex: action.index };

    case 'TIMELINE_TYPE':
      if (state.type !== 'TIMELINE') return state;
      return { ...state, idBuffer: state.idBuffer + action.char };

    case 'TIMELINE_CLEAR_BUFFER':
      if (state.type !== 'TIMELINE') return state;
      return { ...state, idBuffer: '' };

    default:
      return state;
  }
}

export function useOverlay() {
  let state = $state<OverlayState>({ type: 'NONE' });

  function dispatch(action: OverlayAction) {
    state = overlayReducer(state, action);
  }

  // --- Convenience methods ---

  function openCommandPalette(initialNamespace?: 'exit-modes') {
    dispatch({ type: 'OPEN_COMMAND_PALETTE', initialNamespace });
  }

  function openHelp() {
    dispatch({ type: 'OPEN_HELP' });
  }

  function openTimeline() {
    dispatch({ type: 'OPEN_TIMELINE' });
  }

  function close() {
    dispatch({ type: 'CLOSE' });
  }

  function updateCommandPaletteState(newState: CommandPaletteState) {
    dispatch({ type: 'UPDATE_COMMAND_PALETTE', state: newState });
  }

  // --- Timeline-specific methods ---

  function timelineMoveSelection(index: number) {
    dispatch({ type: 'TIMELINE_MOVE', index });
  }

  function timelineTypeChar(char: string) {
    dispatch({ type: 'TIMELINE_TYPE', char });
  }

  function timelineClearBuffer() {
    dispatch({ type: 'TIMELINE_CLEAR_BUFFER' });
  }

  // --- Query methods ---

  function isOpen(): boolean {
    return state.type !== 'NONE';
  }

  function isCommandPaletteOpen(): boolean {
    return state.type === 'COMMAND_PALETTE';
  }

  function isHelpOpen(): boolean {
    return state.type === 'HELP';
  }

  function isTimelineOpen(): boolean {
    return state.type === 'TIMELINE';
  }

  function getCommandPaletteState(): CommandPaletteState | null {
    return state.type === 'COMMAND_PALETTE' ? state.state : null;
  }

  function getCommandPaletteInitialNamespace(): 'exit-modes' | undefined {
    return state.type === 'COMMAND_PALETTE' ? state.initialNamespace : undefined;
  }

  function getTimelineState(): { selectedIndex: number; idBuffer: string } | null {
    return state.type === 'TIMELINE' ? { selectedIndex: state.selectedIndex, idBuffer: state.idBuffer } : null;
  }

  return {
    // State getter
    get current() { return state; },

    // Dispatch
    dispatch,

    // Convenience methods
    openCommandPalette,
    openHelp,
    openTimeline,
    close,
    updateCommandPaletteState,

    // Timeline methods
    timelineMoveSelection,
    timelineTypeChar,
    timelineClearBuffer,

    // Query methods
    isOpen,
    isCommandPaletteOpen,
    isHelpOpen,
    isTimelineOpen,
    getCommandPaletteState,
    getCommandPaletteInitialNamespace,
    getTimelineState,
  };
}

export type UseOverlayReturn = ReturnType<typeof useOverlay>;
