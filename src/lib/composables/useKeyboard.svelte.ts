declare const __IS_MACOS__: boolean;

export interface KeyboardHandlers {
  onShiftDown?: () => void;
  onShiftUp?: () => void;
  onTabCycle?: (direction: 'forward' | 'backward') => void;
  onOpenSessionEditor?: () => void;
  onOpenCommandPalette?: () => void;
  onOpenCommandPaletteWithNamespace?: (namespace: 'exit-modes') => void;
  onOpenSaveModal?: () => void;
  onCloseWindow?: () => void;
  onOpenSearch?: () => void;
  onOpenHelp?: () => void;
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
  /** Whether help overlay is open */
  isHelpOverlayOpen: () => boolean;
  /** Whether search bar is open */
  isSearchOpen: () => boolean;
  /** Whether a line is currently hovered */
  hasHoveredLine: () => boolean;
  /** Whether exit modes are available */
  hasExitModes: () => boolean;
  /** Whether the hovered line is selectable */
  isHoveredLineSelectable: () => boolean;
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

    // Block all shortcuts when help overlay is open (it handles its own Escape)
    if (state.isHelpOverlayOpen()) return;

    if (e.key === 'Tab') {
      e.preventDefault();
      if (state.isEditorActive() || state.isCommandPaletteOpen()) return;

      if (e.altKey) {
        handlers.onOpenCommandPaletteWithNamespace?.('exit-modes');
      } else if (state.hasExitModes()) {
        handlers.onTabCycle?.(e.shiftKey ? 'backward' : 'forward');
      }
      return;
    }

    // 'c' to comment the hovered line
    if (e.key === 'c' && !e.metaKey && !e.ctrlKey) {
      if (isInEditorOrInput()) return;

      // Block if editor is active
      if (state.isEditorActive()) return;

      // Default: comment hovered line
      if (state.hasHoveredLine() && state.isHoveredLineSelectable()) {
        e.preventDefault();
        window.getSelection()?.removeAllRanges();
        handlers.onCommentHoveredLine?.();
        return;
      }
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

    // '?' for help overlay
    if (e.key === '?' && !state.isEditorActive() && !state.isCommandPaletteOpen() && !state.isHelpOverlayOpen()) {
      if (isInEditorOrInput()) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      e.preventDefault();
      handlers.onOpenHelp?.();
      return;
    }

    // Ctrl+W to close window — non-macOS only (macOS closes windows natively)
    if (!__IS_MACOS__ && e.key === 'w' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handlers.onCloseWindow?.();
      return;
    }

    // Cmd+S for save
    if (e.key === 's' && (e.metaKey || e.ctrlKey) && !state.isSaveModalOpen()) {
      e.preventDefault();
      handlers.onOpenSaveModal?.();
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
