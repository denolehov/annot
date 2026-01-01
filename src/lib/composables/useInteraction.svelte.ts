import type { Range } from '$lib/range';
import { isLineInRange } from '$lib/range';

/**
 * Unified interaction state machine phases:
 * - idle: No interaction
 * - hovering: Mouse over a line (preview selection)
 * - selecting: Drag in progress
 * - committed: Selection made, waiting for action
 * - editing: Annotation editor is open
 */
export type Phase = 'idle' | 'hovering' | 'selecting' | 'committed' | 'editing';

export interface UseInteractionOptions {
  /** Check if a line can be selected (e.g., skip header lines in diff mode) */
  isLineSelectable: (displayIdx: number) => boolean;
  /** Constrain selection to bounds (e.g., hunk bounds in diff mode) */
  constrainToBounds: (displayIdx: number, anchorIdx: number) => number;
}

interface InteractionState {
  phase: Phase;
  hoverLine: number | null;   // Only set in 'hovering' phase
  anchor: number | null;      // Drag start point in 'selecting' phase
  range: Range | null;        // Selection range in selecting/committed/editing
}

export function useInteraction(options: UseInteractionOptions) {
  let state = $state<InteractionState>({
    phase: 'idle',
    hoverLine: null,
    anchor: null,
    range: null,
  });

  // Shift key tracking (for cursor styling)
  let isShiftHeld = $state(false);

  // Derived convenience getters
  let phase = $derived(state.phase);
  let range = $derived(state.range);
  let hoverLine = $derived(state.hoverLine);

  /**
   * Check if a line should show selection highlight.
   * In 'hovering' phase, the hovered line shows preview.
   * In selecting/committed/editing, lines in range show selection.
   */
  function isLineHighlighted(displayIdx: number): boolean {
    if (state.phase === 'hovering' && state.hoverLine === displayIdx) {
      return true;
    }
    if (state.range && (state.phase === 'selecting' || state.phase === 'committed' || state.phase === 'editing')) {
      return isLineInRange(displayIdx, state.range);
    }
    return false;
  }

  /**
   * Check if a line is in preview mode (hover, not committed).
   * Used for lighter visual treatment.
   */
  function isLinePreview(displayIdx: number): boolean {
    return state.phase === 'hovering' && state.hoverLine === displayIdx;
  }

  /**
   * Check if the "+" button should be visible on this line.
   * Only visible in hovering phase on the hovered line.
   */
  function showAddButton(displayIdx: number): boolean {
    return state.phase === 'hovering' && state.hoverLine === displayIdx;
  }

  // --- Pointer handlers (using Pointer Capture API) ---

  function handlePointerDown(displayIdx: number, e: PointerEvent) {
    if (!options.isLineSelectable(displayIdx)) return;

    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);

    state = {
      phase: 'selecting',
      hoverLine: null,
      anchor: displayIdx,
      range: { start: displayIdx, end: displayIdx },
    };
  }

  function handlePointerMove(e: PointerEvent) {
    if (state.phase !== 'selecting' || state.anchor === null) return;

    // Get element under pointer (works even with capture)
    const el = document.elementFromPoint(e.clientX, e.clientY);
    const displayIdx = getDisplayIdxFromElement(el);

    if (displayIdx !== null && options.isLineSelectable(displayIdx)) {
      const constrained = options.constrainToBounds(displayIdx, state.anchor);
      state = {
        ...state,
        range: { start: state.anchor, end: constrained },
      };
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (state.phase !== 'selecting') return;

    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);

    if (state.range) {
      state = {
        phase: 'committed',
        hoverLine: null,
        anchor: null,
        range: state.range,
      };
    } else {
      state = { phase: 'idle', hoverLine: null, anchor: null, range: null };
    }
  }

  // Fallback for pointerup outside the captured element
  function handleGlobalPointerUp() {
    if (state.phase === 'selecting') {
      if (state.range) {
        state = {
          phase: 'committed',
          hoverLine: null,
          anchor: null,
          range: state.range,
        };
      } else {
        state = { phase: 'idle', hoverLine: null, anchor: null, range: null };
      }
    }
  }

  // --- Content-level shift+click for starting drag ---

  function handleContentPointerDown(e: PointerEvent) {
    if (!e.shiftKey) return;

    const el = document.elementFromPoint(e.clientX, e.clientY);
    const displayIdx = getDisplayIdxFromElement(el);

    if (displayIdx === null) return;
    if (!options.isLineSelectable(displayIdx)) return;

    e.preventDefault();

    state = {
      phase: 'selecting',
      hoverLine: null,
      anchor: displayIdx,
      range: { start: displayIdx, end: displayIdx },
    };
  }

  // --- Line hover handlers ---

  function handleLineEnter(displayIdx: number) {
    // Only update hover if we're idle, hovering, or committed (not selecting/editing)
    if (state.phase === 'idle') {
      if (options.isLineSelectable(displayIdx)) {
        state = {
          phase: 'hovering',
          hoverLine: displayIdx,
          anchor: null,
          range: null,
        };
      }
    } else if (state.phase === 'hovering') {
      if (options.isLineSelectable(displayIdx)) {
        state = { ...state, hoverLine: displayIdx };
      }
    }
    // In committed/editing phases, hover doesn't change state
  }

  function handleLineLeave() {
    if (state.phase === 'hovering') {
      state = { phase: 'idle', hoverLine: null, anchor: null, range: null };
    }
  }

  function handleContentLeave() {
    if (state.phase === 'hovering') {
      state = { phase: 'idle', hoverLine: null, anchor: null, range: null };
    }
  }

  // --- Gutter click (toggle selection) ---

  function handleGutterClick(displayIdx: number) {
    // If we just finished selecting, ignore the click (it was part of the drag)
    // The pointerup already transitioned us to 'committed'
    if (state.phase === 'committed') return;

    if (!options.isLineSelectable(displayIdx)) return;

    // Toggle: if clicking same single-line selection, clear it
    if (state.range?.start === displayIdx && state.range?.end === displayIdx) {
      state = { phase: 'idle', hoverLine: null, anchor: null, range: null };
    } else {
      state = {
        phase: 'committed',
        hoverLine: null,
        anchor: null,
        range: { start: displayIdx, end: displayIdx },
      };
    }
  }

  // --- Editor state transitions ---

  function openEditor() {
    if (state.phase === 'committed' && state.range) {
      state = { ...state, phase: 'editing' };
    }
  }

  function closeEditor() {
    if (state.phase === 'editing') {
      state = { ...state, phase: 'committed' };
    }
  }

  function clearSelection() {
    state = { phase: 'idle', hoverLine: null, anchor: null, range: null };
  }

  function setSelection(range: Range) {
    state = {
      phase: 'committed',
      hoverLine: null,
      anchor: null,
      range,
    };
  }

  function selectLine(displayIdx: number) {
    if (options.isLineSelectable(displayIdx)) {
      state = {
        phase: 'committed',
        hoverLine: null,
        anchor: null,
        range: { start: displayIdx, end: displayIdx },
      };
    }
  }

  // --- Shift key handlers (for cursor styling) ---

  function handleShiftKeyDown() {
    isShiftHeld = true;
  }

  function handleShiftKeyUp() {
    isShiftHeld = false;
  }

  /** Get context for bookmark creation: committed selection or hovered line. */
  function getBookmarkContext(): { start: number; end: number } | null {
    if (state.phase === 'committed' && state.range) {
      return { start: state.range.start, end: state.range.end };
    }
    if (state.phase === 'hovering' && state.hoverLine !== null) {
      return { start: state.hoverLine, end: state.hoverLine };
    }
    return null;
  }

  return {
    // State getters
    get phase() { return phase; },
    get range() { return range; },
    get hoverLine() { return hoverLine; },
    get isShiftHeld() { return isShiftHeld; },

    // Query functions
    isLineHighlighted,
    isLinePreview,
    showAddButton,

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

    // Keyboard
    handleShiftKeyDown,
    handleShiftKeyUp,

    // Bookmark context
    getBookmarkContext,
  };
}

// --- Helper ---

function getDisplayIdxFromElement(el: Element | null): number | null {
  if (!el) return null;

  // Walk up to find element with data-display-idx
  const line = el.closest('[data-display-idx]');
  if (!line) return null;

  const idx = line.getAttribute('data-display-idx');
  if (idx === null) return null;

  const parsed = parseInt(idx, 10);
  return isNaN(parsed) ? null : parsed;
}
