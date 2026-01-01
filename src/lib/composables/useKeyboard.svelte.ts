/** Context for creating a selection bookmark (start === end for single line). */
export type BookmarkContext = { start: number; end: number };

export interface KeyboardHandlers {
  onShiftDown?: () => void;
  onShiftUp?: () => void;
  onTabCycle?: (direction: 'forward' | 'backward') => void;
  onOpenSessionEditor?: () => void;
  onOpenCommandPalette?: () => void;
  onOpenSaveModal?: () => void;
  onOpenSearch?: () => void;
  onCreateSessionBookmark?: () => void;
  onCreateSelectionBookmark?: (context: BookmarkContext) => void;
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
  /** Get bookmark context (hover or selection), null if neither */
  getBookmarkContext: () => BookmarkContext | null;
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

    // Shift+C for global/session comment
    if (e.key === 'C' && !e.metaKey && !e.ctrlKey && !state.isEditorActive()) {
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

    // Shift+B for session bookmark (full document)
    if (e.key === 'B' && !e.metaKey && !e.ctrlKey && !state.isEditorActive()) {
      if (isInEditorOrInput()) return;
      e.preventDefault();
      handlers.onCreateSessionBookmark?.();
      return;
    }

    // 'b' for selection bookmark (hover/selection context only)
    if (e.key === 'b' && !e.metaKey && !e.ctrlKey && !state.isEditorActive()) {
      if (isInEditorOrInput()) return;
      const context = state.getBookmarkContext();
      if (!context) return;
      e.preventDefault();
      handlers.onCreateSelectionBookmark?.(context);
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
