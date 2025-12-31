export interface KeyboardHandlers {
  onShiftDown?: () => void;
  onShiftUp?: () => void;
  onTabCycle?: (direction: 'forward' | 'backward') => void;
  onOpenSessionEditor?: () => void;
  onOpenCommandPalette?: () => void;
  onOpenSaveModal?: () => void;
  onOpenSearch?: () => void;
  onCreateBookmark?: () => void;
  onEditLastBookmark?: () => void;
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onZoomReset?: () => void;
  onCommentHoveredLine?: () => void;
  onSelectAllContent?: () => void;
}

export interface KeyboardState {
  /** Whether any editor is currently active (selection or session) */
  isEditorActive: () => boolean;
  /** Whether command palette is open */
  isCommandPaletteOpen: () => boolean;
  /** Whether save modal is open */
  isSaveModalOpen: () => boolean;
  /** Whether search bar is open */
  isSearchOpen: () => boolean;
  /** Whether a line is currently hovered */
  hasHoveredLine: () => boolean;
  /** Whether exit modes are available */
  hasExitModes: () => boolean;
  /** Whether the hovered line is selectable */
  isHoveredLineSelectable: () => boolean;
  /** Whether there's a last created bookmark that can be edited */
  hasLastCreatedBookmark: () => boolean;
}

export function useKeyboard(handlers: KeyboardHandlers, state: KeyboardState) {
  function isInEditorOrInput(): boolean {
    const activeEl = document.activeElement;
    const isInEditor = activeEl?.closest('.annotation-editor, .session-editor');
    const isInInput = activeEl instanceof HTMLInputElement || activeEl instanceof HTMLTextAreaElement;
    const isContentEditable = activeEl instanceof HTMLElement && activeEl.isContentEditable;
    return !!(isInEditor || isInInput || isContentEditable);
  }

  function handleKeyDown(e: KeyboardEvent): void {
    if (e.key === 'Shift') {
      handlers.onShiftDown?.();
      return;
    }

    if (e.key === 'Tab') {
      e.preventDefault();
      if (state.hasExitModes() && !state.isEditorActive() && !state.isCommandPaletteOpen()) {
        handlers.onTabCycle?.(e.shiftKey ? 'backward' : 'forward');
      }
      return;
    }

    // 'c' to comment hovered line
    if (e.key === 'c' && !e.metaKey && !e.ctrlKey && state.hasHoveredLine() && !state.isEditorActive()) {
      if (isInEditorOrInput()) return;
      if (!state.isHoveredLineSelectable()) return;
      e.preventDefault();
      handlers.onCommentHoveredLine?.();
      return;
    }

    // 'g' for global/session comment
    if (e.key === 'g' && !state.isEditorActive()) {
      if (isInEditorOrInput()) return;
      e.preventDefault();
      handlers.onOpenSessionEditor?.();
      return;
    }

    // ':' for command palette
    if (e.key === ':' && !state.isEditorActive() && !state.isCommandPaletteOpen()) {
      if (isInEditorOrInput()) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      e.preventDefault();
      handlers.onOpenCommandPalette?.();
      return;
    }

    // Cmd+S for save
    if (e.key === 's' && (e.metaKey || e.ctrlKey) && !state.isSaveModalOpen()) {
      e.preventDefault();
      handlers.onOpenSaveModal?.();
      return;
    }

    // 'b' for bookmark
    if (e.key === 'b' && !e.metaKey && !e.ctrlKey && !state.isEditorActive()) {
      if (isInEditorOrInput()) return;
      e.preventDefault();
      handlers.onCreateBookmark?.();
      return;
    }

    // 'e' to edit last created bookmark
    if (e.key === 'e' && !e.metaKey && !e.ctrlKey && state.hasLastCreatedBookmark() && !state.isEditorActive() && !state.isCommandPaletteOpen()) {
      if (isInEditorOrInput()) return;
      e.preventDefault();
      handlers.onEditLastBookmark?.();
      return;
    }

    // Cmd+F for search (blocked in editor or command palette)
    if (e.key === 'f' && (e.metaKey || e.ctrlKey) && !state.isSearchOpen() && !state.isEditorActive() && !state.isCommandPaletteOpen()) {
      e.preventDefault();
      handlers.onOpenSearch?.();
      return;
    }

    // Zoom controls
    if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
      e.preventDefault();
      handlers.onZoomIn?.();
    } else if ((e.metaKey || e.ctrlKey) && e.key === '-') {
      e.preventDefault();
      handlers.onZoomOut?.();
    } else if ((e.metaKey || e.ctrlKey) && e.key === '0') {
      e.preventDefault();
      handlers.onZoomReset?.();
    }
  }

  function handleKeyUp(e: KeyboardEvent): void {
    if (e.key === 'Shift') {
      handlers.onShiftUp?.();
    }
  }

  return {
    handleKeyDown,
    handleKeyUp,
  };
}
