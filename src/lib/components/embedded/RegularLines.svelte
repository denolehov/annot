<script lang="ts">
  /**
   * RegularLines - Renders non-special line segments (not portal/codeblock/table).
   *
   * Handles regular markdown lines, diff lines, and their annotations.
   */
  import type { Line, MarkdownMetadata, Tag, CodeBlockInfo, SectionInfo } from '$lib/types';
  import type { Range } from '$lib/range';
  import type { JSONContent } from '@tiptap/core';
  import type { SearchMatch } from '$lib/composables/useSearch.svelte';
  import { rangeToKey } from '$lib/range';
  import { getLineNumber, getDiffKind } from '$lib/line-utils';
  import { highlightMatches, clearHighlights } from '$lib/search-highlight';
  import { invoke } from '@tauri-apps/api/core';
  import CopyButton from '$lib/components/CopyButton.svelte';
  import AnnotationSlot, { type AnnotationSlotProps } from '$lib/components/AnnotationSlot.svelte';
  import { BookmarkIcon } from '$lib/icons';

  interface DisplayLine {
    line: Line;
    displayIndex: number;
  }

  interface Props {
    lines: DisplayLine[];
    markdownMetadata: MarkdownMetadata | null;
    selection: Range | null;
    interactionRange: Range | null;
    interactionPhase: string;
    lastSelectedLine: number | null;

    // Search
    searchMatches: SearchMatch[];
    currentSearchMatch: SearchMatch | null;

    // Functions
    isSelected: (displayIdx: number) => boolean;
    isPreview: (displayIdx: number) => boolean;
    hasAnnotation: (displayIdx: number) => boolean;
    isLineBookmarked: (displayIdx: number) => boolean;
    getAnnotationAtLine: (displayIdx: number) => { key: string; content: JSONContent } | null;
    getMermaidBlockAt: (lineNum: number) => CodeBlockInfo | null;
    openMermaidWindow: (block: CodeBlockInfo) => void;

    // Interaction handlers
    onPointerDown: (displayIdx: number, e: PointerEvent) => void;
    onGutterClick: (displayIdx: number) => void;
    onLineEnter: (displayIdx: number) => void;
    onLineLeave: () => void;

    // Annotation slot props
    annotationSlotProps: Omit<AnnotationSlotProps, 'rangeKey'>;
  }

  let {
    lines,
    markdownMetadata,
    selection,
    interactionRange,
    interactionPhase,
    lastSelectedLine,
    searchMatches,
    currentSearchMatch,
    isSelected,
    isPreview,
    hasAnnotation,
    isLineBookmarked,
    getAnnotationAtLine,
    getMermaidBlockAt,
    openMermaidWindow,
    onPointerDown,
    onGutterClick,
    onLineEnter,
    onLineLeave,
    annotationSlotProps,
  }: Props = $props();

  // Map of display indices to code element refs for search highlighting
  let codeRefs: Map<number, HTMLElement> = new Map();

  // Svelte action to track code element refs
  function setCodeRef(el: HTMLElement, displayIndex: number) {
    codeRefs.set(displayIndex, el);
    return {
      destroy() {
        codeRefs.delete(displayIndex);
      },
    };
  }

  /**
   * Get section info for a line if it's a markdown heading.
   */
  function getSectionAt(lineNum: number): SectionInfo | null {
    if (!markdownMetadata?.sections) return null;
    return markdownMetadata.sections.find(s => s.source_line === lineNum) ?? null;
  }

  /**
   * Copy a section to clipboard.
   */
  async function copySection(section: SectionInfo) {
    await invoke('copy_section', {
      startLine: section.source_line,
      endLine: section.end_line,
    });
  }

  // Apply search highlights when matches change
  $effect(() => {
    // Clear all previous highlights first
    for (const el of codeRefs.values()) {
      clearHighlights(el);
    }

    // Apply new highlights
    for (const match of searchMatches) {
      const el = codeRefs.get(match.displayIndex);
      if (el) {
        const isCurrent = currentSearchMatch?.displayIndex === match.displayIndex;
        // Find the range index within this match that should be "current"
        const currentRangeIndex = isCurrent ? 0 : null;
        highlightMatches(el, match.ranges, currentRangeIndex);
      }
    }
  });
</script>

{#each lines as { line, displayIndex }}
  {@const sourceLineNum = getLineNumber(line)}
  {@const diffKind = getDiffKind(line)}
  {@const mermaidBlock = sourceLineNum !== null ? getMermaidBlockAt(sourceLineNum) : null}
  {@const sectionInfo = sourceLineNum !== null ? getSectionAt(sourceLineNum) : null}
  {@const bookmarked = isLineBookmarked(displayIndex)}
  <div
    class="line"
    class:selected={isSelected(displayIndex)}
    class:annotated={hasAnnotation(displayIndex)}
    class:preview={isPreview(displayIndex)}
    class:bookmarked
    class:diff-added={diffKind === 'added'}
    class:diff-deleted={diffKind === 'deleted'}
    class:diff-context={diffKind === 'context'}
    class:diff-header={diffKind === 'file_header' || diffKind === 'hunk_header'}
    data-display-idx={displayIndex}
    onmouseenter={() => onLineEnter(displayIndex)}
    onmouseleave={() => onLineLeave()}
    role="presentation"
  >
    <button
      class="add-btn"
      onpointerdown={(e) => onPointerDown(displayIndex, e)}
      aria-label="Add annotation"
    >+</button>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <span
      class="gutter"
      class:selected={isSelected(displayIndex)}
      onpointerdown={(e) => onPointerDown(displayIndex, e)}
      onclick={() => onGutterClick(displayIndex)}
      role="button"
      tabindex="-1"
    >
      {#if line.origin.type === 'diff'}
        <span class="diff-gutter-old">{line.origin.old_line ?? ''}</span>
        <span class="diff-gutter-new">{line.origin.new_line ?? ''}</span>
      {:else if sourceLineNum !== null}
        {sourceLineNum}
      {/if}
    </span>
    <span
      class="code"
      class:md={markdownMetadata}
      use:setCodeRef={displayIndex}
    >{#if line.html?.type === 'full'}{@html line.html.value}{:else}{line.content}{/if}</span>
    {#if mermaidBlock}
      <button
        class="mermaid-view-btn"
        onclick={() => openMermaidWindow(mermaidBlock)}
        title="View diagram"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" width="14" height="14">
          <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 7.125C2.25 6.504 2.754 6 3.375 6h6c.621 0 1.125.504 1.125 1.125v3.75c0 .621-.504 1.125-1.125 1.125h-6a1.125 1.125 0 0 1-1.125-1.125v-3.75ZM14.25 8.625c0-.621.504-1.125 1.125-1.125h5.25c.621 0 1.125.504 1.125 1.125v8.25c0 .621-.504 1.125-1.125 1.125h-5.25a1.125 1.125 0 0 1-1.125-1.125v-8.25ZM3.75 16.125c0-.621.504-1.125 1.125-1.125h5.25c.621 0 1.125.504 1.125 1.125v2.25c0 .621-.504 1.125-1.125 1.125h-5.25a1.125 1.125 0 0 1-1.125-1.125v-2.25Z" />
        </svg>
      </button>
    {/if}
    {#if sectionInfo}
      <CopyButton
        onCopy={() => copySection(sectionInfo)}
        title="Copy section"
        hoverOnly
        class="copy-section-btn"
      />
    {/if}
    {#if bookmarked}
      <span class="bookmark-indicator"><BookmarkIcon filled /></span>
    {/if}
  </div>
  {@const annotationAtLine = getAnnotationAtLine(displayIndex)}
  {@const isLastSelectedLine = displayIndex === lastSelectedLine && interactionRange && interactionPhase !== 'selecting'}
  {@const rangeKey = annotationAtLine?.key ?? (isLastSelectedLine && interactionRange ? rangeToKey(interactionRange) : null)}
  <AnnotationSlot {rangeKey} {...annotationSlotProps} />
{/each}

<style>
  .mermaid-view-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-left: 8px;
    padding: 2px 4px;
    background: var(--bg-window);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mermaid-view-btn:hover {
    background: var(--bg-panel);
    color: var(--text-primary);
    border-color: var(--border-strong);
  }

  .mermaid-view-btn:focus-visible {
    outline: none;
    border-color: var(--focus-ring);
  }

  .mermaid-view-btn svg {
    display: block;
  }

  :global(.copy-section-btn) {
    margin-left: 8px;
  }

  .line:hover :global(.copy-section-btn) {
    opacity: 1;
  }
</style>
