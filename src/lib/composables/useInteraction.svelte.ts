import type { Range } from '$lib/range';
import { isLineInRange, keyToRange } from '$lib/range';

/**
 * Editor identification - which editor is currently active.
 */
export type EditorKind =
  | { kind: 'annotation'; rangeKey: string }
  | { kind: 'session' };

/**
 * Modal lock - blocks destructive transitions when a modal (like Excalidraw) is open.
 * Orthogonal to UiState to keep interaction state pure.
 */
export type ModalLock =
  | null
  | { kind: 'excalidraw'; editorKey: string };

/**
 * Discriminated union for UI interaction state.
 * Each phase only contains the data it needs — impossible states are unrepresentable.
 */
export type UiState =
  | { phase: 'idle' }
  | { phase: 'selecting'; anchor: number; current: number }
  | { phase: 'committed'; range: Range }
  | { phase: 'editing'; editor: EditorKind };

/** Derived type for phase names (for backwards compatibility) */
export type Phase = UiState['phase'];

export type UiAction =
  | { type: 'START_SELECT'; anchor: number }
  | { type: 'EXTEND_SELECT'; to: number }
  | { type: 'COMMIT_SELECT' }
  | { type: 'OPEN_EDITOR'; editor: EditorKind }
  | { type: 'CLOSE_EDITOR' }
  | { type: 'SET_SELECTION'; range: Range }
  | { type: 'RESET' };

/** Actions that are blocked when a modal lock is active */
const DESTRUCTIVE_ACTIONS: UiAction['type'][] = ['START_SELECT', 'CLOSE_EDITOR', 'RESET', 'SET_SELECTION'];

/**
 * Pure reducer for UI state transitions.
 * All state changes go through here for predictability.
 */
export function uiReducer(state: UiState, action: UiAction): UiState {
  switch (action.type) {
    case 'START_SELECT':
      // Can start selecting from any phase (interrupts current state)
      return { phase: 'selecting', anchor: action.anchor, current: action.anchor };

    case 'EXTEND_SELECT':
      if (state.phase !== 'selecting') return state;
      return { ...state, current: action.to };

    case 'COMMIT_SELECT':
      if (state.phase !== 'selecting') return state;
      const range = normalizeRange(state.anchor, state.current);
      return { phase: 'committed', range };

    case 'OPEN_EDITOR':
      // Can open from committed, idle, or editing (to switch editors)
      if (state.phase === 'committed' || state.phase === 'idle' || state.phase === 'editing') {
        return { phase: 'editing', editor: action.editor };
      }
      return state;

    case 'CLOSE_EDITOR':
      if (state.phase !== 'editing') return state;
      return { phase: 'idle' };

    case 'SET_SELECTION':
      return { phase: 'committed', range: action.range };

    case 'RESET':
      return { phase: 'idle' };

    default:
      return state;
  }
}

function normalizeRange(anchor: number, current: number): Range {
  return {
    start: Math.min(anchor, current),
    end: Math.max(anchor, current),
  };
}

export interface UseInteractionOptions {
  /** Check if a line can be selected (e.g., skip header lines in diff mode) */
  isLineSelectable: (displayIdx: number) => boolean;
  /** Constrain selection to bounds (e.g., hunk bounds in diff mode) */
  constrainToBounds: (displayIdx: number, anchorIdx: number) => number;
}

export function useInteraction(options: UseInteractionOptions) {
  let state = $state<UiState>({ phase: 'idle' });

  // Modal lock - blocks destructive transitions when a modal is open
  let modalLock = $state<ModalLock>(null);

  // Shift key tracking (for cursor styling) - separate from phase state
  let isShiftHeld = $state(false);

  // Hovered line — deliberately NOT part of the reducer state. Hover changes on
  // every mouse-move; if 10k LineRows derived from it they'd all re-run each move.
  // The hover *visual* is pure CSS (:hover); this value only feeds keyboard actions
  // that need "the line under the cursor" (annotate without a selection).
  let hoverLine = $state<number | null>(null);

  // Dispatch action through reducer, respecting modal lock
  function dispatch(action: UiAction): { blocked: boolean } {
    if (modalLock !== null) {
      if (DESTRUCTIVE_ACTIONS.includes(action.type)) {
        return { blocked: true };
      }
      // Also block switching editors (OPEN_EDITOR while already editing)
      if (action.type === 'OPEN_EDITOR' && state.phase === 'editing') {
        return { blocked: true };
      }
    }
    state = uiReducer(state, action);
    return { blocked: false };
  }

  // --- Derived getters ---

  function getRange(): Range | null {
    switch (state.phase) {
      case 'selecting':
        return normalizeRange(state.anchor, state.current);
      case 'committed':
        return state.range;
      case 'editing':
        // Editing phase: derive range from editor kind
        if (state.editor.kind === 'annotation') {
          return keyToRange(state.editor.rangeKey);
        }
        return null; // Session editor has no range
      default:
        return null;
    }
  }

  function getHoverLine(): number | null {
    return hoverLine;
  }

  /**
   * Check if a line should show selection highlight.
   */
  function isLineHighlighted(displayIdx: number): boolean {
    const range = getRange();
    if (range && (state.phase === 'selecting' || state.phase === 'committed' || state.phase === 'editing')) {
      return isLineInRange(displayIdx, range);
    }
    return false;
  }

  /**
   * Check if a line is in preview mode (hover, not committed).
   */
  function isLinePreview(displayIdx: number): boolean {
    return hoverLine === displayIdx;
  }

  /**
   * Check if the "+" button should be visible on this line.
   */
  function showAddButton(displayIdx: number): boolean {
    return hoverLine === displayIdx;
  }

  // --- Pointer handlers ---

  function handlePointerDown(displayIdx: number, e: PointerEvent) {
    if (!options.isLineSelectable(displayIdx)) return;

    e.preventDefault();
    clearNativeSelection();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);

    dispatch({ type: 'START_SELECT', anchor: displayIdx });
  }

  function handlePointerMove(e: PointerEvent) {
    if (state.phase !== 'selecting') return;

    e.preventDefault();

    const el = document.elementFromPoint(e.clientX, e.clientY);
    const displayIdx = getDisplayIdxFromElement(el);

    if (displayIdx !== null && options.isLineSelectable(displayIdx)) {
      const constrained = options.constrainToBounds(displayIdx, state.anchor);
      dispatch({ type: 'EXTEND_SELECT', to: constrained });
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (state.phase !== 'selecting') return;

    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    commitSelection();
  }

  function handleGlobalPointerUp() {
    if (state.phase === 'selecting') {
      commitSelection();
    }
  }

  function commitSelection() {
    if (state.phase !== 'selecting') return;

    const range = normalizeRange(state.anchor, state.current);
    const rangeKey = `${range.start}-${range.end}`;

    // Releasing a selection opens the annotation editor directly.
    dispatch({ type: 'COMMIT_SELECT' });
    dispatch({ type: 'OPEN_EDITOR', editor: { kind: 'annotation', rangeKey } });
  }

  function handleContentPointerDown(e: PointerEvent) {
    if (!e.shiftKey) return;

    const el = document.elementFromPoint(e.clientX, e.clientY);
    const displayIdx = getDisplayIdxFromElement(el);

    if (displayIdx === null) return;
    if (!options.isLineSelectable(displayIdx)) return;

    e.preventDefault();
    clearNativeSelection();

    dispatch({ type: 'START_SELECT', anchor: displayIdx });
  }

  // --- Line hover handlers ---

  function handleLineEnter(displayIdx: number) {
    // Only track hover when idle — matches the old "hover only from idle" behavior.
    // No dispatch: this must not reassign `state`, or every LineRow re-renders.
    if (state.phase === 'idle' && options.isLineSelectable(displayIdx)) {
      hoverLine = displayIdx;
    }
  }

  function handleLineLeave() {
    hoverLine = null;
  }

  function handleContentLeave() {
    hoverLine = null;
  }

  // --- Gutter click ---

  function handleGutterClick(displayIdx: number) {
    if (state.phase === 'committed') return;
    if (!options.isLineSelectable(displayIdx)) return;

    clearNativeSelection();

    // Toggle: if clicking same single-line selection, clear it
    const currentRange = getRange();
    if (currentRange?.start === displayIdx && currentRange?.end === displayIdx) {
      dispatch({ type: 'RESET' });
    } else {
      dispatch({ type: 'SET_SELECTION', range: { start: displayIdx, end: displayIdx } });
    }
  }

  // --- Editor state transitions ---

  function openEditor(editor: EditorKind): { blocked: boolean } {
    return dispatch({ type: 'OPEN_EDITOR', editor });
  }

  function closeEditor(): { blocked: boolean } {
    return dispatch({ type: 'CLOSE_EDITOR' });
  }

  /** Check if an annotation is sealed (not being edited) */
  function isAnnotationSealed(rangeKey: string): boolean {
    if (state.phase !== 'editing') return true;
    if (state.editor.kind !== 'annotation') return true;
    return state.editor.rangeKey !== rangeKey;
  }

  /** Check if the session editor is open */
  function isSessionEditorOpen(): boolean {
    if (state.phase !== 'editing') return false;
    return state.editor.kind === 'session';
  }

  /** Set modal lock (blocks destructive actions) */
  function setModalLock(lock: ModalLock): void {
    modalLock = lock;
  }

  function clearSelection() {
    dispatch({ type: 'RESET' });
  }

  function setSelection(range: Range) {
    dispatch({ type: 'SET_SELECTION', range });
  }

  function selectLine(displayIdx: number) {
    if (options.isLineSelectable(displayIdx)) {
      dispatch({ type: 'SET_SELECTION', range: { start: displayIdx, end: displayIdx } });
    }
  }

  // --- Shift key handlers ---

  function handleShiftKeyDown() {
    isShiftHeld = true;
  }

  function handleShiftKeyUp() {
    isShiftHeld = false;
  }

  return {
    // State getters
    get phase() { return state.phase; },
    get state() { return state; },
    get range() { return getRange(); },
    get hoverLine() { return getHoverLine(); },
    get isShiftHeld() { return isShiftHeld; },
    get modalLock() { return modalLock; },

    // Query functions
    isLineHighlighted,
    isLinePreview,
    showAddButton,
    isAnnotationSealed,
    isSessionEditorOpen,

    // Pointer handlers
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
    handleGlobalPointerUp,
    handleContentPointerDown,

    // Line hover handlers
    handleLineEnter,
    handleLineLeave,
    handleContentLeave,

    // Click handlers
    handleGutterClick,

    // Editor transitions
    openEditor,
    closeEditor,
    clearSelection,
    setSelection,
    selectLine,
    setModalLock,

    // Keyboard
    handleShiftKeyDown,
    handleShiftKeyUp,
  };
}

// --- Helpers ---

function clearNativeSelection(): void {
  window.getSelection()?.removeAllRanges();
}

function getDisplayIdxFromElement(el: Element | null): number | null {
  if (!el) return null;

  const line = el.closest('[data-display-idx]');
  if (!line) return null;

  const idx = line.getAttribute('data-display-idx');
  if (idx === null) return null;

  const parsed = parseInt(idx, 10);
  return isNaN(parsed) ? null : parsed;
}
