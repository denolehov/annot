<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { ExcalidrawHandle } from '$lib/excalidraw-loader';

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
  }

  onMount(async () => {
    if (!containerEl) return;

    try {
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

      // Track initial state for change detection
      initialElementCount = parsedElements.filter((el) => !el.isDeleted).length;
      initialHash = hashElements(parsedElements);

      handle = await mountExcalidraw({
        container: containerEl,
        initialElements: parsedElements,
        onSave: handleSave,
        onCancel: tryCancel,
      });

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
    } catch (e) {
      error = String(e);
      loading = false;
      // Still show window on error
      const win = getCurrentWindow();
      await win.show();
    }
  });

  onDestroy(() => {
    handle?.unmount();
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (showConfirmDialog) {
        dismissConfirm();
      } else {
        tryCancel();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

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
        <button class="confirm-btn confirm-btn-cancel" onclick={dismissConfirm}>Keep editing</button>
        <button class="confirm-btn confirm-btn-discard" onclick={confirmCancel}>Discard</button>
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
    height: calc(100% - 40px);
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

  /* Control buttons positioned in window */
  :global(.excalidraw-controls) {
    position: absolute;
    bottom: 16px;
    right: 16px;
    display: flex;
    gap: 8px;
    z-index: 10;
  }

  :global(.excalidraw-save),
  :global(.excalidraw-cancel) {
    padding: 8px 16px;
    border-radius: var(--radius-md);
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: opacity var(--transition-normal);
  }

  :global(.excalidraw-save:hover),
  :global(.excalidraw-cancel:hover) {
    opacity: 0.9;
  }

  :global(.excalidraw-save) {
    background: var(--accent-primary);
    color: white;
  }

  :global(.excalidraw-cancel) {
    background: var(--bg-panel);
    color: var(--text-primary);
    border: 1px solid var(--border-strong);
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
    gap: 12px;
    justify-content: center;
  }

  .confirm-btn {
    padding: 10px 20px;
    border-radius: var(--radius-lg);
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition:
      opacity var(--transition-normal),
      transform 0.1s;
  }

  .confirm-btn:hover {
    opacity: 0.9;
  }

  .confirm-btn:active {
    transform: scale(0.98);
  }

  .confirm-btn-cancel {
    background: var(--bg-panel);
    color: var(--text-primary);
    border: 1px solid var(--border-strong);
  }

  .confirm-btn-discard {
    background: var(--danger);
    color: white;
  }
</style>
