<script lang="ts">
  import type { DocView } from '$lib/display-rows';
  import type { FileStatus } from '$lib/types';
  import { buildFileTree, flattenFileTree } from '$lib/file-tree';
  import {
    FolderIcon,
    FolderOpenIcon,
    FileAddedIcon,
    FileDeletedIcon,
    FileDiffIcon,
    FileRenamedIcon,
  } from '$lib/icons';

  // GitHub's octicon set has no distinct "copied" or "type changed" glyph —
  // copied reads as a rename (identity carried over), type-changed as a
  // modification (content changed either way).
  const STATUS_ICON: Record<FileStatus, typeof FileAddedIcon> = {
    added: FileAddedIcon,
    deleted: FileDeletedIcon,
    modified: FileDiffIcon,
    type_changed: FileDiffIcon,
    renamed: FileRenamedIcon,
    copied: FileRenamedIcon,
  };

  interface Props {
    docs: DocView[];
    currentIndex: number;
    onJump: (displayIndex: number) => void;
    isDirExpanded: (path: string) => boolean;
    toggleDir: (path: string) => void;
  }

  let { docs, currentIndex, onJump, isDirExpanded, toggleDir }: Props = $props();

  // Matches .tree-row-icon width (16px) + .tree-row gap (4px) — each depth's
  // icon lands directly under the previous depth's label start. Scaled by
  // --content-zoom (set on document.documentElement, +page.svelte) like the
  // rest of the tree's dimensions.
  const INDENT_BASE = 4;
  const INDENT_STEP = 20;
  const indent = (depth: number) =>
    `padding-left: calc(${INDENT_BASE + depth * INDENT_STEP}px * var(--content-zoom, 1))`;

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
  <div class="file-tree-inner">
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
              <span class="tree-row-icon">
                {#if isDirExpanded(row.path)}
                  <FolderOpenIcon />
                {:else}
                  <FolderIcon />
                {/if}
              </span>
              <span class="dir-row-name">{row.name}</span>
            </button>
          {:else}
            {@const StatusIcon = STATUS_ICON[row.dv.doc.status]}
            <button
              class="tree-row file-row"
              class:current={row.dv.index === currentIndex}
              aria-current={row.dv.index === currentIndex ? 'true' : undefined}
              style={indent(row.depth)}
              title={row.dv.path}
              onclick={(e) => jump(e, row.dv)}
            >
              <span class="tree-row-icon status-{row.dv.doc.status}">
                <StatusIcon />
              </span>
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
  </div>
</aside>
