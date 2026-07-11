<script lang="ts">
  /**
   * FileHeaderRow - sticky file header bar for diff views.
   *
   * Replaces the raw `diff --git` LineRow. The whole bar toggles collapse.
   * Keeps data-display-idx so scroll targeting and position tracking still
   * resolve the header row; the row itself is not selectable/annotatable.
   */
  import type { DocView } from '$lib/display-rows';
  import { ChevronDownIcon } from '$lib/icons';

  interface Props {
    dv: DocView;
    collapsed: boolean;
    onToggle: () => void;
  }

  let { dv, collapsed, onToggle }: Props = $props();
</script>

<div class="file-header-line" data-display-idx={dv.headerDisplayIndex} role="presentation">
  <button
    class="file-header-bar"
    onclick={onToggle}
    aria-expanded={!collapsed}
    title={dv.path}
  >
    <span class="file-header-chevron" class:collapsed>
      <ChevronDownIcon />
    </span>
    <span class="file-header-path">
      {#if dv.dir}<span class="file-header-dir">{dv.dir}</span>{/if}<span class="file-header-name">{dv.name}</span>
    </span>
    <span class="file-tree-counts">
      {#if dv.added}<span class="added">+{dv.added}</span>{/if}
      {#if dv.deleted}<span class="deleted">−{dv.deleted}</span>{/if}
    </span>
  </button>
</div>
