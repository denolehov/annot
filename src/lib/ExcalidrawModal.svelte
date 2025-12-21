<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { ExcalidrawHandle } from './excalidraw-loader';

  interface Props {
    initialElements?: string; // JSON string
    onSave: (elements: string, png: string) => void;
    onCancel: () => void;
  }

  let { initialElements = '[]', onSave, onCancel }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let modalEl: HTMLDivElement | undefined = $state();
  let handle: ExcalidrawHandle | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showConfirmDialog = $state(false);
  let initialElementCount = 0;

  function hasUnsavedChanges(): boolean {
    if (!handle) return false;
    const currentElements = handle.getElements();
    // Check if there are any elements beyond what we started with
    // Filter out deleted elements (Excalidraw marks deleted elements with isDeleted: true)
    const activeElements = currentElements.filter((el: { isDeleted?: boolean }) => !el.isDeleted);
    return activeElements.length > initialElementCount;
  }

  function tryCancel() {
    if (hasUnsavedChanges()) {
      showConfirmDialog = true;
    } else {
      onCancel();
    }
  }

  function confirmCancel() {
    showConfirmDialog = false;
    onCancel();
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

      const { mountExcalidraw } = await import('./excalidraw-loader');

      let parsedElements = [];
      try {
        parsedElements = JSON.parse(initialElements || '[]');
      } catch {
        console.warn('Failed to parse initial elements, using empty array');
      }

      // Track initial element count for change detection
      initialElementCount = parsedElements.filter((el: { isDeleted?: boolean }) => !el.isDeleted).length;

      handle = await mountExcalidraw({
        container: containerEl,
        initialElements: parsedElements,
        onSave: (elements, png) => {
          onSave(JSON.stringify(elements), png);
        },
        onCancel: tryCancel,
      });

      loading = false;

      // Focus Excalidraw so it receives keyboard events
      // Poll until .excalidraw wrapper appears (Excalidraw takes time to render)
      const focusExcalidraw = () => {
        const excalidrawWrapper = containerEl?.querySelector('.excalidraw');
        if (excalidrawWrapper) {
          const tabbableElement = excalidrawWrapper.querySelector('[tabindex]') as HTMLElement | null;
          if (tabbableElement) {
            tabbableElement.focus();
          } else {
            (excalidrawWrapper as HTMLElement).setAttribute('tabindex', '-1');
            (excalidrawWrapper as HTMLElement).focus();
          }
          return true;
        }
        return false;
      };

      // Try immediately, then poll every 50ms up to 1 second
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
    } catch (e) {
      error = String(e);
      loading = false;
    }
  });

  onDestroy(() => {
    handle?.unmount();
  });

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      tryCancel();
    }
  }

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

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="excalidraw-backdrop" onclick={handleBackdropClick} role="presentation">
  <div bind:this={modalEl} class="excalidraw-modal" role="dialog" aria-modal="true" tabindex="-1">
    {#if loading}
      <div class="excalidraw-loading">Loading Excalidraw...</div>
    {:else if error}
      <div class="excalidraw-error">{error}</div>
    {/if}
    <div bind:this={containerEl} class="excalidraw-container"></div>
  </div>
</div>

{#if showConfirmDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="confirm-backdrop" onclick={dismissConfirm} role="presentation">
    <div class="confirm-dialog" role="alertdialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <p class="confirm-message">Discard unsaved drawing?</p>
      <div class="confirm-buttons">
        <button class="confirm-btn confirm-btn-cancel" onclick={dismissConfirm}>Keep editing</button>
        <button class="confirm-btn confirm-btn-discard" onclick={confirmCancel}>Discard</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .excalidraw-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .excalidraw-modal {
    width: 90vw;
    height: 85vh;
    background: white;
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
    outline: none;
  }

  .excalidraw-container {
    flex: 1;
    position: relative;
    min-height: 0; /* Important for flex children */
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
    background: white;
  }

  .excalidraw-error {
    color: var(--error-text, #dc2626);
  }

  /* Control buttons positioned in modal */
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
    border-radius: 6px;
    font-family: var(--font-ui, system-ui);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s;
  }

  :global(.excalidraw-save:hover),
  :global(.excalidraw-cancel:hover) {
    opacity: 0.9;
  }

  :global(.excalidraw-save) {
    background: var(--accent-primary, #3b82f6);
    color: white;
  }

  :global(.excalidraw-cancel) {
    background: var(--bg-panel, #f3f4f6);
    color: var(--text-primary, #1f2937);
    border: 1px solid var(--border-strong, #d1d5db);
  }

  /* Confirm dialog */
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
  }

  .confirm-dialog {
    background: white;
    border-radius: 12px;
    padding: 24px;
    max-width: 320px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.2);
  }

  .confirm-message {
    margin: 0 0 20px 0;
    font-family: var(--font-ui, system-ui);
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary, #1f2937);
    text-align: center;
  }

  .confirm-buttons {
    display: flex;
    gap: 12px;
    justify-content: center;
  }

  .confirm-btn {
    padding: 10px 20px;
    border-radius: 8px;
    font-family: var(--font-ui, system-ui);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s, transform 0.1s;
  }

  .confirm-btn:hover {
    opacity: 0.9;
  }

  .confirm-btn:active {
    transform: scale(0.98);
  }

  .confirm-btn-cancel {
    background: var(--bg-panel, #f3f4f6);
    color: var(--text-primary, #1f2937);
    border: 1px solid var(--border-strong, #d1d5db);
  }

  .confirm-btn-discard {
    background: #ef4444;
    color: white;
  }
</style>
