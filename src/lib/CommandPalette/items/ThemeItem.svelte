<script lang="ts">
  import { getContext } from 'svelte';
  import type { Item, ItemSelectionState } from '../engine/types';
  import Icon from '../Icon.svelte';

  interface Props {
    item: Item;
    selectionState: ItemSelectionState;
  }

  let { item, selectionState }: Props = $props();

  const getCurrentThemeId = getContext<() => string>('currentThemeId');
  const isCurrentTheme = $derived(item.id === getCurrentThemeId());
</script>

<div class="theme-item" data-state={selectionState}>
  <span class="name">{item.name}</span>
  {#if isCurrentTheme}
    <span class="current-indicator"><Icon name="check" /></span>
  {/if}
</div>

<style>
  .theme-item {
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

  .theme-item:hover {
    background: var(--bg-panel);
  }

  .theme-item[data-state='preselected'] {
    background: transparent;
    outline: 2px solid var(--border-strong);
    outline-offset: -2px;
  }

  .theme-item[data-state='selected'] {
    background: var(--bg-selected);
  }

  .theme-item[data-state='pending-delete'] {
    background: var(--error-bg);
  }

  .name {
    flex: 1;
    font-weight: 500;
    color: var(--text-primary);
  }

  .current-indicator {
    color: var(--accent);
    display: flex;
    align-items: center;
  }
</style>
