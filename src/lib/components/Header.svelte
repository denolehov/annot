<script lang="ts">
  import CopyDropdown from '$lib/CopyDropdown.svelte';
  import type { DiffMetadata, MarkdownMetadata, DiffFileInfo, HunkInfo, SectionInfo, JSONContent } from '$lib/types';

  interface Props {
    label: string;
    metadata: { type: 'plain' } | ({ type: 'diff' } & DiffMetadata) | ({ type: 'markdown' } & MarkdownMetadata);
    currentFile: DiffFileInfo | null;
    currentFileIndex: number;
    currentHunk: HunkInfo | null;
    sectionBreadcrumb: SectionInfo[];
    headerRootSection: SectionInfo | null;
    headerH2Section: SectionInfo | null;
    headerCurrentSection: SectionInfo | null;
    headerCurrentDepth: number;
    hasSessionComment: boolean;
    onOpenSessionEditor: () => void;
    onOpenSaveModal: () => void;
    showToast: (message: string) => void;
  }

  let {
    label,
    metadata,
    currentFile,
    currentFileIndex,
    currentHunk,
    sectionBreadcrumb,
    headerRootSection,
    headerH2Section,
    headerCurrentSection,
    headerCurrentDepth,
    hasSessionComment,
    onOpenSessionEditor,
    onOpenSaveModal,
    showToast
  }: Props = $props();

  const diffMetadata = $derived(metadata.type === 'diff' ? metadata : null);
  const markdownMetadata = $derived(metadata.type === 'markdown' ? metadata : null);
</script>

<header class="header" data-tauri-drag-region>
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
        ><span class="md-header-title">{label}</span></span>

        <!-- H1 (root) - always shown -->
        {#if headerRootSection}
          <span class="md-header-sep">·</span>
          <span class="md-header-section md-header-root">
            <span class="md-header-level">#</span>
            <span class="md-header-title">{headerRootSection.title}</span>
          </span>
        {/if}

        <!-- H2 shown only when current depth is exactly 2 -->
        {#if headerCurrentDepth === 2 && headerH2Section}
          <span class="md-header-sep">·</span>
          <span class="md-header-section md-header-current">
            <span class="md-header-level">##</span>
            <span class="md-header-title">{headerH2Section.title}</span>
          </span>
        {/if}

        <!-- Ellipsis + current section when depth >= 3 -->
        {#if headerCurrentDepth >= 3 && headerCurrentSection}
          <span class="md-header-sep">·</span>
          <span class="md-header-ellipsis">…</span>
          <span class="md-header-sep">·</span>
          <span class="md-header-section md-header-current">
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
      >{label}</span>
    {/if}
  </div>
  <div class="header-right">
    <CopyDropdown {showToast} />
    <button class="header-btn" onclick={onOpenSaveModal} title="Save to file (Cmd+S)">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" width="16" height="16">
        <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
      </svg>
    </button>
  </div>
</header>
