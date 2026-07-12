import type { Range } from '$lib/range';
import { isLineInRange } from '$lib/range';
import { anchorSides, type Anchor, type Side } from '$lib/anchor';

/**
 * Editor identification - which editor is currently active.
 * Annotations are referenced by id (saved entry or draft).
 */
export type EditorKind =
  | { kind: 'annotation'; id: string }
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
 *
 * The drag gesture (`selecting`) lives in display space: it may transiently
 * cross virtual lines or portal boundaries, judged only at commit. `side` is
 * the column the drag started in (split view); null in unified/flat modes.
 * A committed selection is a draft Anchor — source coordinates, the same
 * shape annotations persist — resolved back to display rows on demand.
 */
export type UiState =
  | { phase: 'idle' }
  | { phase: 'selecting'; anchor: number; current: number; side: Side | null }
  | { phase: 'committed'; draft: Anchor }
  | { phase: 'editing'; editor: EditorKind };

/** Derived type for phase names (for backwards compatibility) */
export type Phase = UiState['phase'];

export type UiAction =
  | { type: 'START_SELECT'; anchor: number; side: Side | null }
  | { type: 'EXTEND_SELECT'; to: number }
  | { type: 'COMMIT_SELECT'; draft: Anchor }
  | { type: 'OPEN_EDITOR'; editor: EditorKind }
  | { type: 'CLOSE_EDITOR' }
  | { type: 'SET_SELECTION'; draft: Anchor }
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
      return { phase: 'selecting', anchor: action.anchor, current: action.anchor, side: action.side };

    case 'EXTEND_SELECT':
      if (state.phase !== 'selecting') return state;
      return { ...state, current: action.to };

    case 'COMMIT_SELECT':
      if (state.phase !== 'selecting') return state;
      return { phase: 'committed', draft: action.draft };

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
      return { phase: 'committed', draft: action.draft };

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
  /** Resolve an annotation's display span (editing-phase highlight). */
  spanForAnnotation: (id: string) => Range | null;
  /**
   * Display selection → draft anchor at commit, routed by mode. `side` scopes
   * a split-view drag to one column. Null = unanchorable — the gesture
   * dissolves instead of committing.
   */
  anchorForRange: (range: Range, side: Side | null) => Anchor | null;
  /** Resolve a committed draft anchor back to its display span. */
  spanForDraft: (anchor: Anchor) => Range | null;
  /** An annotation's anchor (editing-phase column coverage in split view). */
  anchorForAnnotation: (id: string) => Anchor | null;
  /**
   * Which editor a just-committed selection opens; null blocks. Reads the
   * draft onSelectionChange already minted for the committed anchor — no
   * param needed, dispatch() calls onSelectionChange before this.
   */
  editorForSelection: () => EditorKind | null;
  /** Fired on selection state changes: the committed draft anchor, or null when idle. */
  onSelectionChange: (draft: Anchor | null) => void;
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
    // Notify on the committed draft / return to idle — the page keys its
    // draft-annotation lifecycle off this. Editing keeps the last-committed
    // state alive; selecting is transient (the slot is hidden while dragging).
    if (state.phase === 'committed') {
      options.onSelectionChange(state.draft);
    } else if (state.phase === 'idle') {
      options.onSelectionChange(null);
    }
    return { blocked: false };
  }

  // --- Derived getters ---

  function getRange(): Range | null {
    switch (state.phase) {
      case 'selecting':
        return normalizeRange(state.anchor, state.current);
      case 'committed':
        return options.spanForDraft(state.draft);
      case 'editing':
        // Editing phase: resolve the annotation's span from its anchor
        if (state.editor.kind === 'annotation') {
          return options.spanForAnnotation(state.editor.id);
        }
        return null; // Session editor has no range
      default:
        return null;
    }
  }

  function getHoverLine(): number | null {
    return hoverLine;
  }

  const BOTH_SIDES = { old: true, new: true };

  /** Which split-view columns the active selection/edit covers. */
  function activeSides(): { old: boolean; new: boolean } {
    switch (state.phase) {
      case 'selecting':
        return state.side ? { old: state.side === 'old', new: state.side === 'new' } : BOTH_SIDES;
      case 'committed':
        return anchorSides(state.draft);
      case 'editing': {
        if (state.editor.kind !== 'annotation') return BOTH_SIDES;
        const anchor = options.anchorForAnnotation(state.editor.id);
        return anchor ? anchorSides(anchor) : BOTH_SIDES;
      }
      default:
        return BOTH_SIDES;
    }
  }

  /**
   * Check if a split-view cell should show selection highlight. `side` is
   * the cell's column; null (unified rows, context cells — the same line in
   * both columns) matches on the display span alone.
   */
  function isCellHighlighted(displayIdx: number, side: Side | null): boolean {
    const range = getRange();
    if (!range || !isLineInRange(displayIdx, range)) return false;
    return side ? activeSides()[side] : true;
  }

  /**
   * Check if a line should show selection highlight.
   */
  function isLineHighlighted(displayIdx: number): boolean {
    return isCellHighlighted(displayIdx, null);
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

    // The drag is scoped to the split-view column it starts in (the cell's
    // data-side wrapper); null in unified/flat modes.
    dispatch({ type: 'START_SELECT', anchor: displayIdx, side: getSideFromElement(e.currentTarget as Element) });
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

    // Release mints the draft anchor; an unanchorable gesture (portal
    // boundary, mixed paths) dissolves back to idle instead of committing.
    const draft = options.anchorForRange(normalizeRange(state.anchor, state.current), state.side);
    if (!draft) {
      dispatch({ type: 'RESET' });
      return;
    }

    // Releasing a selection opens the annotation editor directly. COMMIT_SELECT
    // fires onSelectionChange first, so the page has minted a draft slot (or
    // found the existing annotation) by the time we ask which editor to open.
    dispatch({ type: 'COMMIT_SELECT', draft });
    const editor = options.editorForSelection();
    if (editor) dispatch({ type: 'OPEN_EDITOR', editor });
  }

  function handleContentPointerDown(e: PointerEvent) {
    if (!e.shiftKey) return;

    const el = document.elementFromPoint(e.clientX, e.clientY);
    const displayIdx = getDisplayIdxFromElement(el);

    if (displayIdx === null) return;
    if (!options.isLineSelectable(displayIdx)) return;

    e.preventDefault();
    clearNativeSelection();

    dispatch({ type: 'START_SELECT', anchor: displayIdx, side: getSideFromElement(el) });
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

    // Toggle: if clicking same single-line selection, clear it. A single-line
    // selection needs no side scoping — a lone row anchors on its own side.
    const currentRange = getRange();
    if (currentRange?.start === displayIdx && currentRange?.end === displayIdx) {
      dispatch({ type: 'RESET' });
    } else {
      setSelection({ start: displayIdx, end: displayIdx });
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
  function isAnnotationSealed(id: string): boolean {
    if (state.phase !== 'editing') return true;
    if (state.editor.kind !== 'annotation') return true;
    return state.editor.id !== id;
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

  function setSelection(range: Range, side: Side | null = null) {
    // Unanchorable programmatic selections are a no-op — nothing to commit.
    const draft = options.anchorForRange(range, side);
    if (draft) dispatch({ type: 'SET_SELECTION', draft });
  }

  function selectLine(displayIdx: number, side: Side | null = null) {
    if (options.isLineSelectable(displayIdx)) {
      setSelection({ start: displayIdx, end: displayIdx }, side);
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
    isCellHighlighted,
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

/** The split-view column an element sits in; null outside split view. */
function getSideFromElement(el: Element | null): Side | null {
  const side = el?.closest('[data-side]')?.getAttribute('data-side');
  return side === 'old' || side === 'new' ? side : null;
}
