<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { ExcalidrawHandle } from '$lib/excalidraw-loader';
  import { initTheme } from '$lib/theme';

  interface NodeRef {
    type: 'Chip' | 'Placeholder';
    id: string;
  }

  interface ExcalidrawContext {
    elements: string;
    range_key: string;
    node_ref: NodeRef;
    parent_label: string;
  }

  let containerEl: HTMLDivElement | undefined = $state();
  let handle: ExcalidrawHandle | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showConfirmDialog = $state(false);
  let closeHandled = false;  // Prevent re-entry on Cmd-W
  let unlistenClose: (() => void) | undefined;
  let initialElementCount = 0;
  let initialHash = '';

  interface ExcalidrawElement {
    id?: string;
    isDeleted?: boolean;
  }

  function hashElements(elements: ExcalidrawElement[]): string {
    const active = elements.filter((el) => !el.isDeleted);
    const sorted = [...active].sort((a, b) => (a.id || '').localeCompare(b.id || ''));
    return JSON.stringify(sorted);
  }

  function hasUnsavedChanges(): boolean {
    if (!handle) return false;
    const currentElements = handle.getElements() as ExcalidrawElement[];
    return hashElements(currentElements) !== initialHash;
  }

  async function handleSave() {
    if (!handle) return;

    const elements = handle.getElements();
    const { exportToBlob } = await import('@excalidraw/excalidraw');

    try {
      const blob = await exportToBlob({
        elements,
        mimeType: 'image/png',
        appState: handle.getAppState(),
        files: {},
      });

      const reader = new FileReader();
      reader.onloadend = async () => {
        try {
          unlistenClose?.();  // Remove listener before closing to prevent re-entry
          await invoke('excalidraw_save', {
            elements: JSON.stringify(elements),
            png: reader.result as string,
          });
        } catch (e) {
          console.error('Failed to save:', e);
        }
      };
      reader.readAsDataURL(blob);
    } catch (e) {
      console.error('Failed to export PNG:', e);
      // Save without PNG if export fails
      try {
        unlistenClose?.();  // Remove listener before closing to prevent re-entry
        await invoke('excalidraw_save', {
          elements: JSON.stringify(elements),
          png: '',
        });
      } catch (err) {
        console.error('Failed to save:', err);
      }
    }
  }

  async function handleCancel() {
    // Prevent onCloseRequested from triggering save
    closeHandled = true;
    try {
      await invoke('excalidraw_cancel');
    } catch (e) {
      console.error('Failed to cancel:', e);
      // Close window anyway
      const win = getCurrentWindow();
      await win.close();
    }
  }

  function tryCancel() {
    if (hasUnsavedChanges()) {
      showConfirmDialog = true;
    } else {
      handleCancel();
    }
  }

  function confirmCancel() {
    showConfirmDialog = false;
    handleCancel();
  }

  function dismissConfirm() {
    showConfirmDialog = false;
    closeHandled = false;  // Allow future close attempts
  }

  onMount(async () => {
    if (!containerEl) return;

    try {
      // Initialize theme before mounting
      const effectiveTheme = await initTheme();

      // Set asset path for offline fonts
      (window as unknown as { EXCALIDRAW_ASSET_PATH: string }).EXCALIDRAW_ASSET_PATH =
        '/excalidraw-assets/';

      // Get context from backend
      const context = await invoke<ExcalidrawContext>('get_excalidraw_context');

      const { mountExcalidraw } = await import('$lib/excalidraw-loader');

      let parsedElements: ExcalidrawElement[] = [];
      try {
        parsedElements = JSON.parse(context.elements || '[]');
      } catch {
        console.warn('Failed to parse initial elements, using empty array');
      }

      handle = await mountExcalidraw({
        container: containerEl,
        initialElements: parsedElements,
        theme: effectiveTheme,
      });

      // Track initial state for change detection AFTER mounting
      // (Excalidraw normalizes elements internally, so we must hash the mounted state)
      const mountedElements = handle.getElements() as ExcalidrawElement[];
      initialElementCount = mountedElements.filter((el) => !el.isDeleted).length;
      initialHash = hashElements(mountedElements);

      loading = false;

      // Focus Excalidraw
      const focusExcalidraw = () => {
        const excalidrawWrapper = containerEl?.querySelector('.excalidraw');
        if (excalidrawWrapper) {
          const tabbableElement = excalidrawWrapper.querySelector(
            '[tabindex]'
          ) as HTMLElement | null;
          if (tabbableElement) {
            tabbableElement.focus({ preventScroll: true });
          } else {
            (excalidrawWrapper as HTMLElement).setAttribute('tabindex', '-1');
            (excalidrawWrapper as HTMLElement).focus({ preventScroll: true });
          }
          return true;
        }
        return false;
      };

      let attempts = 0;
      const maxAttempts = 20;
      const tryFocus = () => {
        if (focusExcalidraw() || attempts >= maxAttempts) {
          return;
        }
        attempts++;
        setTimeout(tryFocus, 50);
      };
      tryFocus();

      // Show the window
      const win = getCurrentWindow();
      await win.show();

      // Intercept window close (Cmd-W, traffic light) to auto-save
      unlistenClose = await win.onCloseRequested(async (event) => {
        if (!closeHandled) {
          closeHandled = true;
          event.preventDefault();
          handleSave();
        }
      });
    } catch (e) {
      error = String(e);
      loading = false;
      // Still show window on error
      const win = getCurrentWindow();
      await win.show();
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (showConfirmDialog) {
        dismissConfirm();
      } else {
        tryCancel();
      }
    } else if (e.key === 'Enter' && showConfirmDialog) {
      e.preventDefault();
      e.stopPropagation();
      confirmCancel();
    }
  }

  onDestroy(() => {
    handle?.unmount();
    unlistenClose?.();
    // Remove capture listener
    window.removeEventListener('keydown', handleKeyDown, true);
  });

  // Use capture phase to intercept Escape before Excalidraw handles it
  $effect(() => {
    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  });
</script>

<div class="excalidraw-window">
  <header class="window-header" data-tauri-drag-region>
    <span class="window-title">Excalidraw</span>
  </header>
  {#if loading}
    <div class="excalidraw-loading">Loading Excalidraw...</div>
  {:else if error}
    <div class="excalidraw-error">{error}</div>
  {/if}
  <div bind:this={containerEl} class="excalidraw-container"></div>
  <footer class="status-bar">
    <div class="status-bar-left"></div>
    <div class="status-bar-right">
      <span class="kbd-hint"><kbd>Esc</kbd> dismiss</span>
      <span class="kbd-hint"><kbd>⌘W</kbd> save and close</span>
    </div>
  </footer>
</div>

{#if showConfirmDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="confirm-backdrop" onclick={dismissConfirm} role="presentation">
    <div
      class="confirm-dialog"
      role="alertdialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <p class="confirm-message">Discard unsaved drawing?</p>
      <div class="confirm-buttons">
        <button class="confirm-btn" onclick={dismissConfirm}>
          <kbd>Esc</kbd>
          <span>Keep editing</span>
        </button>
        <button class="confirm-btn confirm-btn-discard" onclick={confirmCancel}>
          <kbd>⏎</kbd>
          <span>Discard</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .excalidraw-window {
    width: 100vw;
    height: 100vh;
    background: var(--bg-window);
    overflow: hidden;
    position: relative;
  }

  .window-header {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 40px;
    -webkit-app-region: drag;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    /* Match main window header styling */
    border-bottom: 1px solid rgba(0, 0, 0, 0.06);
    background: color-mix(in srgb, var(--bg-panel) 85%, transparent);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
  }

  .window-title {
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .excalidraw-container {
    width: 100%;
    height: calc(100% - 40px - 32px);
    margin-top: 40px;
    position: relative;
  }

  /* Excalidraw wrapper needs explicit dimensions */
  :global(.excalidraw-container .excalidraw-wrapper) {
    height: 100% !important;
  }

  :global(.excalidraw-container .excalidraw) {
    height: 100% !important;
  }

  :global(.excalidraw-container .excalidraw .excalidraw-container) {
    height: 100% !important;
  }

  .excalidraw-loading,
  .excalidraw-error {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-ui);
    font-size: 16px;
    color: var(--text-secondary);
    z-index: 10;
    background: var(--bg-window);
  }

  .excalidraw-error {
    color: var(--error-text);
  }

  /* Footer status bar */
  .status-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 6px 16px;
    height: 32px;
    box-sizing: border-box;
    background: var(--bg-main);
    backdrop-filter: blur(16px) saturate(150%);
    -webkit-backdrop-filter: blur(16px) saturate(150%);
    border-top: 1px solid rgba(0, 0, 0, 0.06);
    font-size: 12px;
  }

  .status-bar-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .status-bar-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .kbd-hint {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-muted);
    font-family: var(--font-ui);
    font-size: 11px;
  }

  .kbd-hint kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    background: var(--bg-panel);
    border: 1px solid var(--border-normal);
    border-radius: 4px;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  /* Confirm dialog */
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop-dark);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
  }

  .confirm-dialog {
    background: var(--bg-window);
    border-radius: var(--radius-xl);
    padding: 24px;
    max-width: 320px;
    box-shadow: var(--shadow-lg);
  }

  .confirm-message {
    margin: 0 0 20px 0;
    font-family: var(--font-ui);
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary);
    text-align: center;
  }

  .confirm-buttons {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-window);
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
    transition: border-color 0.1s ease, background 0.1s ease;
  }

  .confirm-btn:hover,
  .confirm-btn:focus {
    background: var(--bg-main);
    border-color: var(--selection-border);
    outline: none;
  }

  .confirm-btn-discard:hover,
  .confirm-btn-discard:focus {
    border-color: var(--danger);
  }
</style>
