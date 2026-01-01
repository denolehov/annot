<script lang="ts">
  import type { Item, ItemSelectionState } from '../engine/types';

  interface Props {
    item: Item;
    selectionState: ItemSelectionState;
  }

  let { item, selectionState }: Props = $props();

  const shortId = $derived(item.id.slice(0, 3));
  const isEmpty = $derived(!item.name || item.name.trim() === '');
  const dateStr = $derived(
    new Date(item.values.created_at).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    })
  );
</script>

<div class="bookmark-item" data-state={selectionState}>
  <div class="primary">
    <span class="id">{shortId}</span>
    {#if isEmpty}
      <span class="label placeholder">&lt;empty&gt;</span>
    {:else}
      <span class="label">{item.name}</span>
    {/if}
  </div>
  <div class="secondary">
    {item.values.source_title} &bull; {dateStr}
  </div>
</div>

<style>
  .bookmark-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 80ms ease;
  }

  .bookmark-item:hover {
    background: var(--bg-panel);
  }

  /* Preselected: user is filtering, item matches (outline) */
  .bookmark-item[data-state='preselected'] {
    background: transparent;
    outline: 2px solid var(--border-strong);
    outline-offset: -2px;
  }

  /* Selected: user navigated to this item (solid) */
  .bookmark-item[data-state='selected'] {
    background: var(--bg-selected);
  }

  /* Pending delete: awaiting dd confirmation */
  .bookmark-item[data-state='pending-delete'] {
    background: var(--error-bg);
  }

  .primary {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .id {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-tertiary);
    min-width: 28px;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .label.placeholder {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 400;
  }

  .secondary {
    font-size: 11px;
    color: var(--text-secondary);
    padding-left: 36px;
  }
</style>
