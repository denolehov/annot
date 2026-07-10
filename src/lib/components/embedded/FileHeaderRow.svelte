<script lang="ts">
  /**
   * FileHeaderRow - sticky file header bar for diff views.
   *
   * Replaces the raw `diff --git` LineRow. The whole bar toggles collapse.
   * Keeps data-display-idx so scroll targeting and position tracking still
   * resolve the header row; the row itself is not selectable/annotatable.
   */
  import type { FileEntry } from '$lib/file-tree';
  import { ChevronDownIcon } from '$lib/icons';

  interface Props {
    entry: FileEntry;
    displayIndex: number;
    collapsed: boolean;
    onToggle: () => void;
  }

  let { entry, displayIndex, collapsed, onToggle }: Props = $props();
</script>

<div class="file-header-line" data-display-idx={displayIndex} role="presentation">
  <button
    class="file-header-bar"
    onclick={onToggle}
    aria-expanded={!collapsed}
    title={entry.path}
  >
    <span class="file-header-chevron" class:collapsed>
      <ChevronDownIcon />
    </span>
    <span class="file-header-path">
      {#if entry.dir}<span class="file-header-dir">{entry.dir}</span>{/if}<span class="file-header-name">{entry.name}</span>
    </span>
    <span class="file-tree-counts">
      {#if entry.added}<span class="added">+{entry.added}</span>{/if}
      {#if entry.deleted}<span class="deleted">−{entry.deleted}</span>{/if}
    </span>
  </button>
</div>
