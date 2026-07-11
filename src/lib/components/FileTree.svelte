<script lang="ts">
  import type { DocView } from '$lib/display-rows';

  interface Props {
    docs: DocView[];
    currentIndex: number;
    onJump: (displayIndex: number) => void;
  }

  let { docs, currentIndex, onJump }: Props = $props();

  let totals = $derived({
    added: docs.reduce((sum, dv) => sum + dv.added, 0),
    deleted: docs.reduce((sum, dv) => sum + dv.deleted, 0),
  });

  function jump(e: MouseEvent, dv: DocView) {
    onJump(dv.headerDisplayIndex);
    // Give focus back to the window so line shortcuts (c, :, ?) keep working.
    (e.currentTarget as HTMLButtonElement).blur();
  }
</script>

<aside class="file-tree" aria-label="Changed files">
  <div class="file-tree-summary">
    <span class="file-tree-summary-label">{docs.length} {docs.length === 1 ? 'file' : 'files'} changed</span>
    <span class="file-tree-counts">
      <span class="added">+{totals.added}</span>
      <span class="deleted">−{totals.deleted}</span>
    </span>
  </div>
  <ul class="file-tree-list">
    {#each docs as dv (dv.index)}
      <li>
        <button
          class="file-row"
          class:current={dv.index === currentIndex}
          aria-current={dv.index === currentIndex ? 'true' : undefined}
          title={dv.path}
          onclick={(e) => jump(e, dv)}
        >
          <span class="file-row-path">
            {#if dv.dir}<span class="file-row-dir">{dv.dir}</span>{/if}<span class="file-row-name">{dv.name}</span>
          </span>
          <span class="file-tree-counts">
            {#if dv.added}<span class="added">+{dv.added}</span>{/if}
            {#if dv.deleted}<span class="deleted">−{dv.deleted}</span>{/if}
          </span>
        </button>
      </li>
    {/each}
  </ul>
</aside>
