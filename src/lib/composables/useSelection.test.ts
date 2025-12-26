import { describe, it, expect } from 'vitest';
import { flushSync } from 'svelte';
import { useSelection } from './useSelection.svelte';

describe('useSelection', () => {
  // Default options - all lines selectable, no constraints
  const defaultOptions = {
    isLineSelectable: () => true,
    constrainToBounds: (displayIdx: number) => displayIdx,
    getDisplayIdxFromEvent: () => null as number | null,
  };

  it('starts with null selection', () => {
    const state = useSelection(defaultOptions);
    expect(state.selection).toBeNull();
    expect(state.isDragging).toBe(false);
    expect(state.isShiftHeld).toBe(false);
    expect(state.hoveredDisplayIdx).toBeNull();
  });

  it('selects a line', () => {
    const state = useSelection(defaultOptions);

    flushSync(() => {
      state.selectLine(5);
    });

    expect(state.selection).toEqual({ start: 5, end: 5 });
  });

  it('clears selection', () => {
    const state = useSelection(defaultOptions);

    flushSync(() => {
      state.selectLine(5);
    });
    expect(state.selection).not.toBeNull();

    flushSync(() => {
      state.clearSelection();
    });
    expect(state.selection).toBeNull();
  });

  it('tracks shift key state', () => {
    const state = useSelection(defaultOptions);

    expect(state.isShiftHeld).toBe(false);

    flushSync(() => {
      state.handleShiftKeyDown();
    });
    expect(state.isShiftHeld).toBe(true);

    flushSync(() => {
      state.handleShiftKeyUp();
    });
    expect(state.isShiftHeld).toBe(false);
  });

  it('handles gutter mouse down', () => {
    const state = useSelection(defaultOptions);
    const mockEvent = {
      preventDefault: () => {},
    } as MouseEvent;

    flushSync(() => {
      state.handleGutterMouseDown(10, mockEvent);
    });

    expect(state.selection).toEqual({ start: 10, end: 10 });
    expect(state.isDragging).toBe(true);
  });

  it('handles mouse up to stop dragging', () => {
    const state = useSelection(defaultOptions);
    const mockEvent = {
      preventDefault: () => {},
    } as MouseEvent;

    flushSync(() => {
      state.handleGutterMouseDown(10, mockEvent);
    });
    expect(state.isDragging).toBe(true);

    flushSync(() => {
      state.handleMouseUp();
    });
    expect(state.isDragging).toBe(false);
  });

  it('toggles selection on gutter click for same line', () => {
    const state = useSelection(defaultOptions);

    // First click selects
    flushSync(() => {
      state.handleGutterClick(5);
    });
    expect(state.selection).toEqual({ start: 5, end: 5 });

    // Second click on same line deselects
    flushSync(() => {
      state.handleGutterClick(5);
    });
    expect(state.selection).toBeNull();
  });

  it('respects isLineSelectable option', () => {
    const state = useSelection({
      ...defaultOptions,
      isLineSelectable: (displayIdx) => displayIdx !== 5,
    });

    // Line 5 is not selectable
    flushSync(() => {
      state.selectLine(5);
    });
    expect(state.selection).toBeNull();

    // Line 10 is selectable
    flushSync(() => {
      state.selectLine(10);
    });
    expect(state.selection).toEqual({ start: 10, end: 10 });
  });

  it('handles add button mouse down', () => {
    const state = useSelection(defaultOptions);
    const mockEvent = {
      preventDefault: () => {},
    } as MouseEvent;

    flushSync(() => {
      state.handleAddMouseDown(15, mockEvent);
    });

    expect(state.selection).toEqual({ start: 15, end: 15 });
    expect(state.isDragging).toBe(true);
  });

  it('allows setting hovered display index', () => {
    const state = useSelection(defaultOptions);

    expect(state.hoveredDisplayIdx).toBeNull();

    flushSync(() => {
      state.hoveredDisplayIdx = 7;
    });
    expect(state.hoveredDisplayIdx).toBe(7);

    flushSync(() => {
      state.hoveredDisplayIdx = null;
    });
    expect(state.hoveredDisplayIdx).toBeNull();
  });

  it('allows setting selection directly', () => {
    const state = useSelection(defaultOptions);

    flushSync(() => {
      state.selection = { start: 5, end: 15 };
    });
    expect(state.selection).toEqual({ start: 5, end: 15 });
  });

  it('extends selection during drag', () => {
    const state = useSelection({
      ...defaultOptions,
      getDisplayIdxFromEvent: () => 15,
    });
    const mockEvent = { preventDefault: () => {} } as MouseEvent;

    // Start dragging
    flushSync(() => {
      state.handleGutterMouseDown(10, mockEvent);
    });
    expect(state.selection).toEqual({ start: 10, end: 10 });

    // Drag to extend
    flushSync(() => {
      state.handleMouseMove(mockEvent);
    });
    expect(state.selection).toEqual({ start: 10, end: 15 });
  });

  it('constrains selection to bounds during drag', () => {
    const state = useSelection({
      ...defaultOptions,
      getDisplayIdxFromEvent: () => 25,
      constrainToBounds: (displayIdx, anchorIdx) => {
        // Constrain to max of anchorIdx + 5
        const maxAllowed = anchorIdx + 5;
        return Math.min(displayIdx, maxAllowed);
      },
    });
    const mockEvent = { preventDefault: () => {} } as MouseEvent;

    // Start at line 10
    flushSync(() => {
      state.handleGutterMouseDown(10, mockEvent);
    });

    // Try to drag to line 25, but constraint limits to 15
    flushSync(() => {
      state.handleMouseMove(mockEvent);
    });
    expect(state.selection).toEqual({ start: 10, end: 15 });
  });

  it('does not extend selection when not dragging', () => {
    const state = useSelection({
      ...defaultOptions,
      getDisplayIdxFromEvent: () => 15,
    });
    const mockEvent = { preventDefault: () => {} } as MouseEvent;

    // Set selection but don't start dragging
    flushSync(() => {
      state.selection = { start: 10, end: 10 };
    });

    // Mouse move should not extend
    flushSync(() => {
      state.handleMouseMove(mockEvent);
    });
    expect(state.selection).toEqual({ start: 10, end: 10 });
  });

  it('handles content mouse down with shift key', () => {
    const state = useSelection({
      ...defaultOptions,
      getDisplayIdxFromEvent: () => 20,
    });
    const mockEvent = {
      preventDefault: () => {},
      shiftKey: true,
    } as unknown as MouseEvent;

    flushSync(() => {
      state.handleContentMouseDown(mockEvent);
    });

    expect(state.selection).toEqual({ start: 20, end: 20 });
    expect(state.isDragging).toBe(true);
  });

  it('ignores content mouse down without shift key', () => {
    const state = useSelection({
      ...defaultOptions,
      getDisplayIdxFromEvent: () => 20,
    });
    const mockEvent = {
      preventDefault: () => {},
      shiftKey: false,
    } as unknown as MouseEvent;

    flushSync(() => {
      state.handleContentMouseDown(mockEvent);
    });

    expect(state.selection).toBeNull();
    expect(state.isDragging).toBe(false);
  });
});
