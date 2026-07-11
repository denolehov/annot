<script lang="ts">
  import CopyDropdown from '$lib/CopyDropdown.svelte';
  import Icon from '$lib/CommandPalette/Icon.svelte';
  import { ChevronUpDownIcon, ChevronDownUpIcon } from '$lib/icons';
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

  const isDiff = $derived(ctx.diffDisplay !== null);
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
</script>

<header class="header" data-tauri-drag-region="deep">
  <div class="header-left">
    {#if isDiff && currentFile}
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
          {fileName}
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
    {#if isDiff && docs.length > 0}
      <span class="diff-header-summary" data-tauri-drag-region="false">
        <span class="file-tree-counts">
          <span class="added">+{totals.added}</span>
          <span class="deleted">−{totals.deleted}</span>
        </span>
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
      </span>
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
  .close-btn {
    font-size: 16px;
    line-height: 1;
  }

  .close-btn:hover {
    color: #ef4444;
  }
</style>
