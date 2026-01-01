import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useKeyboard } from './useKeyboard.svelte';

describe('useKeyboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Default state - nothing active
  const defaultState = {
    isEditorActive: () => false,
    isCommandPaletteOpen: () => false,
    isSaveModalOpen: () => false,
    isSearchOpen: () => false,
    hasHoveredLine: () => false,
    hasExitModes: () => true,
    isHoveredLineSelectable: () => true,
    hasLastCreatedBookmark: () => false,
    getBookmarkContext: () => null,
  };

  function createKeyboardEvent(key: string, options: Partial<KeyboardEvent> = {}): KeyboardEvent {
    return {
      key,
      shiftKey: false,
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      preventDefault: vi.fn(),
      ...options,
    } as unknown as KeyboardEvent;
  }

  it('calls onShiftDown when Shift is pressed', () => {
    const onShiftDown = vi.fn();
    const keyboard = useKeyboard({ onShiftDown }, defaultState);

    keyboard.handleKeyDown(createKeyboardEvent('Shift'));

    expect(onShiftDown).toHaveBeenCalled();
  });

  it('calls onShiftUp when Shift is released', () => {
    const onShiftUp = vi.fn();
    const keyboard = useKeyboard({ onShiftUp }, defaultState);

    keyboard.handleKeyUp(createKeyboardEvent('Shift'));

    expect(onShiftUp).toHaveBeenCalled();
  });

  it('calls onTabCycle forward on Tab', () => {
    const onTabCycle = vi.fn();
    const keyboard = useKeyboard({ onTabCycle }, defaultState);

    const event = createKeyboardEvent('Tab');
    keyboard.handleKeyDown(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(onTabCycle).toHaveBeenCalledWith('forward');
  });

  it('calls onTabCycle backward on Shift+Tab', () => {
    const onTabCycle = vi.fn();
    const keyboard = useKeyboard({ onTabCycle }, defaultState);

    const event = createKeyboardEvent('Tab', { shiftKey: true });
    keyboard.handleKeyDown(event);

    expect(onTabCycle).toHaveBeenCalledWith('backward');
  });

  it('does not cycle exit modes when editor is active', () => {
    const onTabCycle = vi.fn();
    const keyboard = useKeyboard({ onTabCycle }, {
      ...defaultState,
      isEditorActive: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent('Tab'));

    expect(onTabCycle).not.toHaveBeenCalled();
  });

  it('calls onOpenSessionEditor on Shift+C', () => {
    const onOpenSessionEditor = vi.fn();
    const keyboard = useKeyboard({ onOpenSessionEditor }, defaultState);

    const event = createKeyboardEvent('C'); // Capital C means Shift+C
    keyboard.handleKeyDown(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(onOpenSessionEditor).toHaveBeenCalled();
  });

  it('calls onOpenCommandPalette on : key', () => {
    const onOpenCommandPalette = vi.fn();
    const keyboard = useKeyboard({ onOpenCommandPalette }, defaultState);

    const event = createKeyboardEvent(':');
    keyboard.handleKeyDown(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(onOpenCommandPalette).toHaveBeenCalled();
  });

  it('does not open command palette when already open', () => {
    const onOpenCommandPalette = vi.fn();
    const keyboard = useKeyboard({ onOpenCommandPalette }, {
      ...defaultState,
      isCommandPaletteOpen: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent(':'));

    expect(onOpenCommandPalette).not.toHaveBeenCalled();
  });

  it('calls onOpenSaveModal on Cmd+S', () => {
    const onOpenSaveModal = vi.fn();
    const keyboard = useKeyboard({ onOpenSaveModal }, defaultState);

    const event = createKeyboardEvent('s', { metaKey: true });
    keyboard.handleKeyDown(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(onOpenSaveModal).toHaveBeenCalled();
  });

  it('calls onOpenSaveModal on Ctrl+S', () => {
    const onOpenSaveModal = vi.fn();
    const keyboard = useKeyboard({ onOpenSaveModal }, defaultState);

    const event = createKeyboardEvent('s', { ctrlKey: true });
    keyboard.handleKeyDown(event);

    expect(onOpenSaveModal).toHaveBeenCalled();
  });

  it('does not open save modal when already open', () => {
    const onOpenSaveModal = vi.fn();
    const keyboard = useKeyboard({ onOpenSaveModal }, {
      ...defaultState,
      isSaveModalOpen: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent('s', { metaKey: true }));

    expect(onOpenSaveModal).not.toHaveBeenCalled();
  });

  it('calls onZoomIn on Cmd+=', () => {
    const onZoomIn = vi.fn();
    const keyboard = useKeyboard({ onZoomIn }, defaultState);

    const event = createKeyboardEvent('=', { metaKey: true });
    keyboard.handleKeyDown(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(onZoomIn).toHaveBeenCalled();
  });

  it('calls onZoomOut on Cmd+-', () => {
    const onZoomOut = vi.fn();
    const keyboard = useKeyboard({ onZoomOut }, defaultState);

    const event = createKeyboardEvent('-', { metaKey: true });
    keyboard.handleKeyDown(event);

    expect(onZoomOut).toHaveBeenCalled();
  });

  it('calls onZoomReset on Cmd+0', () => {
    const onZoomReset = vi.fn();
    const keyboard = useKeyboard({ onZoomReset }, defaultState);

    const event = createKeyboardEvent('0', { metaKey: true });
    keyboard.handleKeyDown(event);

    expect(onZoomReset).toHaveBeenCalled();
  });

  it('calls onCommentHoveredLine on c key when line is hovered', () => {
    const onCommentHoveredLine = vi.fn();
    const keyboard = useKeyboard({ onCommentHoveredLine }, {
      ...defaultState,
      hasHoveredLine: () => true,
    });

    const event = createKeyboardEvent('c');
    keyboard.handleKeyDown(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(onCommentHoveredLine).toHaveBeenCalled();
  });

  it('does not comment on c key when no line is hovered', () => {
    const onCommentHoveredLine = vi.fn();
    const keyboard = useKeyboard({ onCommentHoveredLine }, {
      ...defaultState,
      hasHoveredLine: () => false,
    });

    keyboard.handleKeyDown(createKeyboardEvent('c'));

    expect(onCommentHoveredLine).not.toHaveBeenCalled();
  });

  it('does not comment on Cmd+C (copy)', () => {
    const onCommentHoveredLine = vi.fn();
    const keyboard = useKeyboard({ onCommentHoveredLine }, {
      ...defaultState,
      hasHoveredLine: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent('c', { metaKey: true }));

    expect(onCommentHoveredLine).not.toHaveBeenCalled();
  });

  it('calls onEditLastBookmark when "e" is pressed and hasLastCreatedBookmark', () => {
    const onEditLastBookmark = vi.fn();
    const keyboard = useKeyboard({ onEditLastBookmark }, {
      ...defaultState,
      hasLastCreatedBookmark: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent('e'));

    expect(onEditLastBookmark).toHaveBeenCalled();
  });

  it('does not edit bookmark when hasLastCreatedBookmark is false', () => {
    const onEditLastBookmark = vi.fn();
    const keyboard = useKeyboard({ onEditLastBookmark }, {
      ...defaultState,
      hasLastCreatedBookmark: () => false,
    });

    keyboard.handleKeyDown(createKeyboardEvent('e'));

    expect(onEditLastBookmark).not.toHaveBeenCalled();
  });

  it('does not edit bookmark when command palette is open', () => {
    const onEditLastBookmark = vi.fn();
    const keyboard = useKeyboard({ onEditLastBookmark }, {
      ...defaultState,
      hasLastCreatedBookmark: () => true,
      isCommandPaletteOpen: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent('e'));

    expect(onEditLastBookmark).not.toHaveBeenCalled();
  });

  it('does not edit bookmark when editor is active', () => {
    const onEditLastBookmark = vi.fn();
    const keyboard = useKeyboard({ onEditLastBookmark }, {
      ...defaultState,
      hasLastCreatedBookmark: () => true,
      isEditorActive: () => true,
    });

    keyboard.handleKeyDown(createKeyboardEvent('e'));

    expect(onEditLastBookmark).not.toHaveBeenCalled();
  });
});
