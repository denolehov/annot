<script lang="ts">
  import { diffTotals, type FileEntry } from '$lib/file-tree';

  interface Props {
    entries: FileEntry[];
    currentIndex: number;
    onJump: (startLine: number) => void;
  }

  let { entries, currentIndex, onJump }: Props = $props();

  let totals = $derived(diffTotals(entries));

  function jump(e: MouseEvent, entry: FileEntry) {
    onJump(entry.startLine);
    // Give focus back to the window so line shortcuts (c, :, ?) keep working.
    (e.currentTarget as HTMLButtonElement).blur();
  }
</script>

<aside class="file-tree" aria-label="Changed files">
  <div class="file-tree-summary">
    <span>{entries.length} {entries.length === 1 ? 'file' : 'files'} changed</span>
    <span class="file-tree-counts">
      <span class="added">+{totals.added}</span>
      <span class="deleted">−{totals.deleted}</span>
    </span>
  </div>
  <ul class="file-tree-list">
    {#each entries as entry (entry.index)}
      <li>
        <button
          class="file-row"
          class:current={entry.index === currentIndex}
          aria-current={entry.index === currentIndex ? 'true' : undefined}
          title={entry.path}
          onclick={(e) => jump(e, entry)}
        >
          <span class="file-row-path">
            {#if entry.dir}<span class="file-row-dir">{entry.dir}</span>{/if}<span class="file-row-name">{entry.name}</span>
          </span>
          <span class="file-tree-counts">
            {#if entry.added}<span class="added">+{entry.added}</span>{/if}
            {#if entry.deleted}<span class="deleted">−{entry.deleted}</span>{/if}
          </span>
        </button>
      </li>
    {/each}
  </ul>
</aside>
