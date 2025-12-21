<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    defaultPath: string;
    onSave: (path: string) => Promise<void>;
    onCancel: () => void;
  }

  let { defaultPath, onSave, onCancel }: Props = $props();

  let path = $state('');
  let saving = $state(false);
  let error = $state<string | null>(null);
  let inputEl: HTMLInputElement | undefined = $state();

  onMount(() => {
    path = defaultPath;
    inputEl?.focus();
    inputEl?.select();
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!path.trim()) {
      error = 'Path is required';
      return;
    }

    saving = true;
    error = null;

    try {
      await onSave(path);
    } catch (e) {
      error = String(e);
      saving = false;
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onCancel();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="save-backdrop" onclick={handleBackdropClick} role="presentation">
  <div class="save-modal" role="dialog" aria-modal="true">
    <button class="save-close" onclick={onCancel} title="Close (Escape)">
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>

    <h3 class="save-title">Save to File</h3>

    <form onsubmit={handleSubmit}>
      <input
        bind:this={inputEl}
        bind:value={path}
        type="text"
        placeholder="path/to/file.md"
        class="save-input"
        disabled={saving}
      />

      {#if error}
        <div class="save-error">{error}</div>
      {/if}

      <div class="save-actions">
        <button type="button" class="save-btn-cancel" onclick={onCancel} disabled={saving}>
          Cancel
        </button>
        <button type="submit" class="save-btn-save" disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </form>
  </div>
</div>

<style>
  .save-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .save-modal {
    width: 400px;
    max-width: 90vw;
    background: var(--bg-panel, #fff);
    border-radius: 12px;
    padding: 20px;
    position: relative;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.15);
  }

  .save-close {
    position: absolute;
    top: 12px;
    right: 12px;
    background: transparent;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-secondary, #6b7280);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .save-close:hover {
    background: var(--bg-hover, #f3f4f6);
    color: var(--text-primary, #1f2937);
  }

  .save-title {
    font-family: var(--font-ui, system-ui);
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary, #1f2937);
    margin: 0 0 16px 0;
  }

  .save-input {
    width: 100%;
    padding: 10px 12px;
    font-family: var(--font-mono, monospace);
    font-size: 14px;
    border: 1px solid var(--border-default, #d1d5db);
    border-radius: 6px;
    background: var(--bg-input, #fff);
    color: var(--text-primary, #1f2937);
    box-sizing: border-box;
  }

  .save-input:focus {
    outline: none;
    border-color: var(--accent-primary, #3b82f6);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .save-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .save-error {
    margin-top: 8px;
    font-family: var(--font-ui, system-ui);
    font-size: 13px;
    color: var(--error-text, #dc2626);
  }

  .save-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }

  .save-btn-cancel,
  .save-btn-save {
    padding: 8px 16px;
    border-radius: 6px;
    font-family: var(--font-ui, system-ui);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s;
  }

  .save-btn-cancel:hover,
  .save-btn-save:hover {
    opacity: 0.9;
  }

  .save-btn-cancel:disabled,
  .save-btn-save:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .save-btn-cancel {
    background: var(--bg-panel, #f3f4f6);
    color: var(--text-primary, #1f2937);
    border: 1px solid var(--border-strong, #d1d5db);
  }

  .save-btn-save {
    background: var(--accent-primary, #3b82f6);
    color: white;
  }
</style>
