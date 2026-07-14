<script lang="ts">
  import CopyDropdown from '$lib/CopyDropdown.svelte';
  import Icon from '$lib/CommandPalette/Icon.svelte';
  import { ChevronUpDownIcon, ChevronDownUpIcon, ColumnsIcon } from '$lib/icons';
  import { getAnnotContext } from '$lib/context';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { HunkV2, SectionInfo } from '$lib/types';
  import type { DocView } from '$lib/display-rows';

  interface Props {
    label: string;
    currentFile: DocView | null;
    currentFileIndex: number;
    currentHunk: HunkV2 | null;
    sectionBreadcrumb: SectionInfo[];
    headerCurrentSection: SectionInfo | null;
    hasSessionComment: boolean;
    onOpenSessionEditor: () => void;
    onOpenSaveModal: () => void;
    zoomLevel: number;
  }

  let {
    label,
    currentFile,
    currentFileIndex,
    currentHunk,
    sectionBreadcrumb,
    headerCurrentSection,
    hasSessionComment,
    onOpenSessionEditor,
    onOpenSaveModal,
    zoomLevel
  }: Props = $props();

  const ctx = getAnnotContext();
  const metadata = $derived(ctx.metadata);
  const showToast = ctx.showToast;

  const markdownMetadata = $derived(metadata.type === 'markdown' ? metadata : null);

  // Extract filename from path for display (label is full path for consistency with LineOrigin)
  const displayLabel = $derived(label.includes('/') ? label.split('/').pop() ?? label : label);

  // Changeset summary (diff mode)
  const docs = $derived(ctx.diffDisplay?.docs ?? []);
  const totals = $derived({
    added: docs.reduce((sum, dv) => sum + dv.added, 0),
    deleted: docs.reduce((sum, dv) => sum + dv.deleted, 0),
  });

  function toggleAllFiles(e: MouseEvent) {
    (ctx.fileCollapse.anyCollapsed ? ctx.fileCollapse.expandAll : ctx.fileCollapse.collapseAll)();
    // Give focus back to the window so line shortcuts (c, :, ?) keep working.
    (e.currentTarget as HTMLButtonElement).blur();
  }

  function toggleDiffView(e: MouseEvent) {
    ctx.setDiffView(ctx.diffView === 'unified' ? 'split' : 'unified');
    (e.currentTarget as HTMLButtonElement).blur();
  }
</script>

<header class="header" data-tauri-drag-region="deep">
  <div class="header-left">
    {#if currentFile}
      <!-- Diff mode: show hunk metadata -->
      {@const fileName = currentFile.path || 'unknown'}
      {@const fileCount = docs.length}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span class="diff-header-info">
        <span
          class="diff-header-file"
          class:has-comment={hasSessionComment}
          onclick={onOpenSessionEditor}
          data-tauri-drag-region="false"
        >
          <span class="diff-header-path">{fileName}</span>
          {#if fileCount > 1}
            <span class="diff-header-counter">({currentFileIndex + 1}/{fileCount})</span>
          {/if}
        </span>
        {#if currentHunk}
          <span class="diff-header-sep">·</span>
          <span class="diff-header-range">
            <span class="diff-header-old">-{currentHunk.old_range.start},{currentHunk.old_range.end - currentHunk.old_range.start}</span>
            <span class="diff-header-new">+{currentHunk.new_range.start},{currentHunk.new_range.end - currentHunk.new_range.start}</span>
          </span>
          {#if currentHunk.function_context}
            <span class="diff-header-fn">
              {#if currentHunk.function_context_html}
                {@html currentHunk.function_context_html}
              {:else}
                {currentHunk.function_context}
              {/if}
            </span>
          {/if}
        {/if}
      </span>
    {:else if markdownMetadata && sectionBreadcrumb.length > 0}
      <!-- Markdown mode: depth-based breadcrumb -->
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span class="md-header-info">
        <!-- Filename -->
        <span
          class="md-header-file"
          class:has-comment={hasSessionComment}
          onclick={onOpenSessionEditor}
          title={label}
          data-tauri-drag-region="false"
        ><span class="md-header-title">{displayLabel}</span></span>

        <!-- Show only the current section (deepest in breadcrumb) -->
        {#if headerCurrentSection}
          <span class="md-header-sep">·</span>
          <span class="md-header-section">
            <span class="md-header-level">{'#'.repeat(headerCurrentSection.level)}</span>
            <span class="md-header-title">{headerCurrentSection.title}</span>
          </span>
        {/if}
      </span>
    {:else}
      <!-- Normal mode: show filename -->
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span
        class="file-name"
        class:has-comment={hasSessionComment}
        onclick={onOpenSessionEditor}
        title={label}
        data-tauri-drag-region="false"
      >{displayLabel}</span>
    {/if}
  </div>
  <div class="header-right">
    {#if docs.length > 0}
      <span class="diff-header-summary" data-tauri-drag-region="false">
        <span class="diff-summary-counts">
          <span class="added">+{totals.added}</span>
          <span class="deleted">−{totals.deleted}</span>
        </span>
      </span>
      <!-- Buttons sit directly in header-right so its gap spaces every
           titlebar button evenly — the summary's wider gap is counts-only. -->
      <button
        class="header-btn"
        class:active={ctx.diffView === 'split'}
        onclick={toggleDiffView}
        title={ctx.diffView === 'split' ? 'Switch to unified view' : 'Switch to split view'}
      >
        <ColumnsIcon />
      </button>
      <button
        class="header-btn"
        onclick={toggleAllFiles}
        title={ctx.fileCollapse.anyCollapsed ? 'Expand all files' : 'Collapse all files'}
      >
        {#if ctx.fileCollapse.anyCollapsed}
          <ChevronUpDownIcon />
        {:else}
          <ChevronDownUpIcon />
        {/if}
      </button>
    {/if}
    {#if zoomLevel !== 1.0}
      <span class="zoom-indicator">{Math.round(zoomLevel * 100)}%</span>
    {/if}
    <CopyDropdown {showToast} />
    <button class="header-btn" onclick={onOpenSaveModal} title="Save to file (Cmd+S)">
      <Icon name="save" />
    </button>
    {#if !__IS_MACOS__}
      <button class="header-btn close-btn" onclick={() => getCurrentWindow().close()} title="Close (Ctrl+W)">
        ×
      </button>
    {/if}
  </div>
</header>

<style>
  /* Buttons carry 6px transparent horizontal padding, so their icons sit
     inset from the 4px flex gap. Mirror that padding after the +/− counts
     so text-to-icon spacing optically matches icon-to-icon spacing. */
  .diff-header-summary {
    margin-right: 6px;
  }

  /* Own class, deliberately not shared with file-tree.css's .file-tree-counts
     (per-file counts in the sidebar, which do scale with --content-zoom) —
     this is titlebar chrome, fixed size like the rest of the titlebar
     (.file-name, .zoom-indicator, .diff-header-info). */
  .diff-summary-counts {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 11px; /* unscaled: chrome */
  }

  .diff-summary-counts .added {
    color: rgb(34, 197, 94);
  }

  .diff-summary-counts .deleted {
    color: rgb(239, 68, 68);
  }

  .close-btn {
    font-size: 16px; /* unscaled: chrome */
    line-height: 1;
  }

  .close-btn:hover {
    color: #ef4444;
  }

  /* Third stage of titlebar decluttering (see .diff-header-fn and
     .diff-header-range in code-viewer.css): once function context and the
     hunk LoC range are gone, drop the changeset +/- summary too so the
     filename keeps the space. */
  @media (max-width: 640px) {
    .diff-header-summary {
      display: none;
    }
  }
</style>
