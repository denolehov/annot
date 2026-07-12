<script lang="ts">
  import type { DocView } from '$lib/display-rows';
  import { buildFileTree, flattenFileTree } from '$lib/file-tree';
  import { ChevronDownIcon } from '$lib/icons';

  interface Props {
    docs: DocView[];
    currentIndex: number;
    onJump: (displayIndex: number) => void;
    isDirExpanded: (path: string) => boolean;
    toggleDir: (path: string) => void;
  }

  let { docs, currentIndex, onJump, isDirExpanded, toggleDir }: Props = $props();

  // Matches .tree-row-icon width (16px) + .tree-row gap (4px) — each depth's
  // icon lands directly under the previous depth's label start.
  const INDENT_BASE = 12;
  const INDENT_STEP = 20;
  const indent = (depth: number) => `padding-left: ${INDENT_BASE + depth * INDENT_STEP}px`;

  let totals = $derived({
    added: docs.reduce((sum, dv) => sum + dv.added, 0),
    deleted: docs.reduce((sum, dv) => sum + dv.deleted, 0),
  });

  let rows = $derived(flattenFileTree(buildFileTree(docs), isDirExpanded));

  function jump(e: MouseEvent, dv: DocView) {
    onJump(dv.headerDisplayIndex);
    // Give focus back to the window so line shortcuts (c, :, ?) keep working.
    (e.currentTarget as HTMLButtonElement).blur();
  }

  function toggle(e: MouseEvent, path: string) {
    toggleDir(path);
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
    {#each rows as row (row.kind === 'dir' ? `dir:${row.path}` : `file:${row.dv.index}`)}
      <li>
        {#if row.kind === 'dir'}
          <button
            class="tree-row dir-row"
            style={indent(row.depth)}
            aria-expanded={isDirExpanded(row.path)}
            title={row.path}
            onclick={(e) => toggle(e, row.path)}
          >
            <span class="tree-row-icon" class:collapsed={!isDirExpanded(row.path)}>
              <ChevronDownIcon />
            </span>
            <span class="dir-row-name">{row.name}</span>
          </button>
        {:else}
          <button
            class="tree-row file-row"
            class:current={row.dv.index === currentIndex}
            aria-current={row.dv.index === currentIndex ? 'true' : undefined}
            style={indent(row.depth)}
            title={row.dv.path}
            onclick={(e) => jump(e, row.dv)}
          >
            <span class="tree-row-icon"></span>
            <span class="file-row-name">{row.dv.name}</span>
            <span class="file-tree-counts">
              {#if row.dv.added}<span class="added">+{row.dv.added}</span>{/if}
              {#if row.dv.deleted}<span class="deleted">−{row.dv.deleted}</span>{/if}
            </span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
</aside>
