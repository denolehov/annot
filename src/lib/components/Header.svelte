<script lang="ts">
  import CopyDropdown from '$lib/CopyDropdown.svelte';
  import Icon from '$lib/CommandPalette/Icon.svelte';
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

  // Extract filename from path for display (label is full path for consistency with LineOrigin)
  const displayLabel = $derived(label.includes('/') ? label.split('/').pop() ?? label : label);
</script>

<header class="header" data-tauri-drag-region>
  <div class="header-left" data-tauri-drag-region>
    {#if diffMetadata && currentFile}
      <!-- Diff mode: show hunk metadata -->
      {@const fileName = currentFile.new_name ?? currentFile.old_name ?? 'unknown'}
      {@const fileCount = diffMetadata.files.length}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span class="diff-header-info" data-tauri-drag-region>
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
          <span class="diff-header-sep" data-tauri-drag-region>·</span>
          <span class="diff-header-range" data-tauri-drag-region>
            <span class="diff-header-old">-{currentHunk.old_start},{currentHunk.old_count}</span>
            <span class="diff-header-new">+{currentHunk.new_start},{currentHunk.new_count}</span>
          </span>
          {#if currentHunk.function_context}
            <span class="diff-header-fn" data-tauri-drag-region>
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
      <span class="md-header-info" data-tauri-drag-region>
        <!-- Filename -->
        <span
          class="md-header-file"
          class:has-comment={hasSessionComment}
          onclick={onOpenSessionEditor}
          title={label}
        ><span class="md-header-title">{displayLabel}</span></span>

        <!-- H1 (root) - always shown -->
        {#if headerRootSection}
          <span class="md-header-sep" data-tauri-drag-region>·</span>
          <span class="md-header-section md-header-root" data-tauri-drag-region>
            <span class="md-header-level" data-tauri-drag-region>#</span>
            <span class="md-header-title" data-tauri-drag-region>{headerRootSection.title}</span>
          </span>
        {/if}

        <!-- H2 shown only when current depth is exactly 2 -->
        {#if headerCurrentDepth === 2 && headerH2Section}
          <span class="md-header-sep" data-tauri-drag-region>·</span>
          <span class="md-header-section md-header-current" data-tauri-drag-region>
            <span class="md-header-level" data-tauri-drag-region>##</span>
            <span class="md-header-title" data-tauri-drag-region>{headerH2Section.title}</span>
          </span>
        {/if}

        <!-- Ellipsis + current section when depth >= 3 -->
        {#if headerCurrentDepth >= 3 && headerCurrentSection}
          <span class="md-header-sep" data-tauri-drag-region>·</span>
          <span class="md-header-ellipsis" data-tauri-drag-region>…</span>
          <span class="md-header-sep" data-tauri-drag-region>·</span>
          <span class="md-header-section md-header-current" data-tauri-drag-region>
            <span class="md-header-level" data-tauri-drag-region>{'#'.repeat(headerCurrentSection.level)}</span>
            <span class="md-header-title" data-tauri-drag-region>{headerCurrentSection.title}</span>
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
      >{displayLabel}</span>
    {/if}
  </div>
  <div class="header-right">
    <CopyDropdown {showToast} />
    <button class="header-btn" onclick={onOpenSaveModal} title="Save to file (Cmd+S)">
      <Icon name="save" />
    </button>
  </div>
</header>
