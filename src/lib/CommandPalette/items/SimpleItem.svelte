<script lang="ts">
  import type { Item, ItemSelectionState } from '../engine/types';

  interface Props {
    item: Item;
    selectionState: ItemSelectionState;
  }

  let { item, selectionState }: Props = $props();
</script>

<div
  class="simple-item"
  data-state={selectionState}
  class:ephemeral={item.isEphemeral}
>
  <span class="name">{item.name}</span>
  {#if item.isEphemeral}
    <span class="ephemeral-badge">session</span>
  {/if}
</div>

<style>
  .simple-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-primary);
    transition: background 80ms ease;
  }

  .simple-item:hover {
    background: var(--bg-panel);
  }

  /* Preselected: user is filtering, item matches (outline) */
  .simple-item[data-state='preselected'] {
    background: transparent;
    outline: 2px solid var(--border-strong);
    outline-offset: -2px;
  }

  /* Selected: user navigated to this item (solid) */
  .simple-item[data-state='selected'] {
    background: var(--bg-selected);
  }

  /* Pending delete: awaiting dd confirmation */
  .simple-item[data-state='pending-delete'] {
    background: var(--error-bg);
  }

  .simple-item.ephemeral {
    border: 1px dashed var(--border-strong);
  }

  .name {
    flex: 1;
    font-weight: 500;
    color: var(--text-primary);
  }

  .ephemeral-badge {
    font-size: 10px;
    padding: 2px 4px;
    color: var(--text-muted);
  }
</style>
