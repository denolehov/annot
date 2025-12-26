import type { Range } from '$lib/range';

/** Display index info for a line */
export interface LineInfo {
  /** 1-indexed display index (position in lines array) */
  displayIdx: number;
}

export interface UseSelectionOptions {
  /** Check if a line can be selected (e.g., skip header lines in diff mode) */
  isLineSelectable: (displayIdx: number) => boolean;
  /** Constrain selection to bounds (e.g., hunk bounds in diff mode) */
  constrainToBounds: (displayIdx: number, anchorIdx: number) => number;
  /** Get display index from mouse event */
  getDisplayIdxFromEvent: (e: MouseEvent) => number | null;
}

export function useSelection(options: UseSelectionOptions) {
  let selection: Range | null = $state(null);
  let isDragging = $state(false);
  let isShiftHeld = $state(false);
  let hoveredDisplayIdx: number | null = $state(null);
  let mouseDownHandled = false; // Prevents click from undoing mousedown

  function handleGutterMouseDown(displayIdx: number, e: MouseEvent) {
    if (!options.isLineSelectable(displayIdx)) return;
    e.preventDefault();
    isDragging = true;
    mouseDownHandled = true;
    selection = { start: displayIdx, end: displayIdx };
  }

  function handleContentMouseDown(e: MouseEvent) {
    if (!e.shiftKey) return;
    const displayIdx = options.getDisplayIdxFromEvent(e);
    if (displayIdx === null) return;
    if (!options.isLineSelectable(displayIdx)) return;

    e.preventDefault();
    isDragging = true;
    selection = { start: displayIdx, end: displayIdx };
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging || !selection) return;
    const displayIdx = options.getDisplayIdxFromEvent(e);
    if (displayIdx !== null) {
      const constrainedIdx = options.constrainToBounds(displayIdx, selection.start);
      selection = { start: selection.start, end: constrainedIdx };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleGutterClick(displayIdx: number) {
    if (mouseDownHandled) {
      mouseDownHandled = false;
      return;
    }
    if (!options.isLineSelectable(displayIdx)) return;

    // Toggle off if clicking same single-line selection
    const sameSelection = selection?.start === displayIdx && selection?.end === displayIdx;
    if (sameSelection) {
      selection = null;
    } else {
      selection = { start: displayIdx, end: displayIdx };
    }
  }

  function handleAddMouseDown(displayIdx: number, e: MouseEvent) {
    if (!options.isLineSelectable(displayIdx)) return;
    e.preventDefault();
    isDragging = true;
    mouseDownHandled = true;
    selection = { start: displayIdx, end: displayIdx };
  }

  function handleShiftKeyDown() {
    isShiftHeld = true;
  }

  function handleShiftKeyUp() {
    isShiftHeld = false;
  }

  function clearSelection() {
    selection = null;
  }

  function selectLine(displayIdx: number) {
    if (options.isLineSelectable(displayIdx)) {
      selection = { start: displayIdx, end: displayIdx };
    }
  }

  return {
    get selection() { return selection; },
    set selection(val: Range | null) { selection = val; },
    get isDragging() { return isDragging; },
    get isShiftHeld() { return isShiftHeld; },
    get hoveredDisplayIdx() { return hoveredDisplayIdx; },
    set hoveredDisplayIdx(val: number | null) { hoveredDisplayIdx = val; },
    handleGutterMouseDown,
    handleContentMouseDown,
    handleMouseMove,
    handleMouseUp,
    handleGutterClick,
    handleAddMouseDown,
    handleShiftKeyDown,
    handleShiftKeyUp,
    clearSelection,
    selectLine,
  };
}
