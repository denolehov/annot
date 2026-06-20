<script lang="ts">
  import CopyDropdown from '$lib/CopyDropdown.svelte';
  import Icon from '$lib/CommandPalette/Icon.svelte';
  import { getAnnotContext } from '$lib/context';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { DiffFileInfo, HunkInfo, SectionInfo } from '$lib/types';

  interface Props {
    label: string;
    currentFile: DiffFileInfo | null;
    currentFileIndex: number;
    currentHunk: HunkInfo | null;
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

  const diffMetadata = $derived(metadata.type === 'diff' ? metadata : null);
  const markdownMetadata = $derived(metadata.type === 'markdown' ? metadata : null);

  // Extract filename from path for display (label is full path for consistency with LineOrigin)
  const displayLabel = $derived(label.includes('/') ? label.split('/').pop() ?? label : label);
</script>

<header class="header" data-tauri-drag-region="deep">
  <div class="header-left">
    {#if diffMetadata && currentFile}
      <!-- Diff mode: show hunk metadata -->
      {@const fileName = currentFile.new_name ?? currentFile.old_name ?? 'unknown'}
      {@const fileCount = diffMetadata.files.length}
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
            <span class="diff-header-old">-{currentHunk.old_start},{currentHunk.old_count}</span>
            <span class="diff-header-new">+{currentHunk.new_start},{currentHunk.new_count}</span>
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
