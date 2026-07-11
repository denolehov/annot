<script lang="ts">
  /**
   * RegularLines - Renders non-special line segments (not portal/codeblock/table).
   *
   * Handles regular markdown lines, diff lines, and their annotations.
   * Uses LineRow for shared line-rendering logic and adds search highlighting via codeWrapper.
   */
  import type { SectionInfo } from '$lib/types';
  import { getLineNumber } from '$lib/line-utils';
  import { highlightMatches, clearHighlights } from '$lib/search-highlight';
  import { injectColorSwatches, clearColorSwatches } from '$lib/color-preview';
  import { invoke } from '@tauri-apps/api/core';
  import CopyButton from '$lib/components/CopyButton.svelte';
  import AnnotationSlot, { type AnnotationSlotProps } from '$lib/components/AnnotationSlot.svelte';
  import LineRow from './LineRow.svelte';
  import FileHeaderRow from './FileHeaderRow.svelte';
  import { getAnnotContext } from '$lib/context';
  import { hunkHeaderText, type DisplayRow } from '$lib/display-rows';
  import type { DisplayLine } from '$lib/composables/useLineSegments.svelte';

  interface Props {
    /** Flat-mode segment lines; unused when the diff walk drives rendering. */
    lines?: DisplayLine[];
    annotationSlotProps: Omit<AnnotationSlotProps, 'slotRef'>;
  }

  let {
    lines = [],
    annotationSlotProps,
  }: Props = $props();

  const ctx = getAnnotContext();

  // Convenience derived values
  const markdownMetadata = $derived(ctx.markdownMetadata);
  const searchMatches = $derived(ctx.search.matches);

  // Diff mode: the DisplayRow walk drives per-file sections for collapse +
  // sticky headers. Null for non-diff content — the flat render path below
  // stays untouched.
  const display = $derived(ctx.diffDisplay);

  // Body entries (hunk headers + rows) per document; file headers render
  // structurally via FileHeaderRow.
  const docBodies = $derived.by(() => {
    const map = new Map<number, DisplayRow[]>();
    if (!display) return map;
    for (const entry of display.rows) {
      if (entry.kind === 'file-header') continue;
      const body = map.get(entry.docIdx);
      if (body) body.push(entry);
      else map.set(entry.docIdx, [entry]);
    }
    return map;
  });

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

  // Inject color swatches for HEX values
  $effect(() => {
    // Track the rendered content source to re-run when it changes
    void lines;
    void display;
    // Use microtask to ensure DOM is updated after render
    queueMicrotask(() => {
      for (const el of codeRefs.values()) {
        clearColorSwatches(el);
        injectColorSwatches(el);
      }
    });
  });

  // Apply search highlights when matches change
  $effect(() => {
    // Clear all previous highlights first
    for (const el of codeRefs.values()) {
      clearHighlights(el);
    }

    // Apply new highlights
    const currentSearchMatch = ctx.search.getCurrentMatch();
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

{#snippet row({ line, displayIndex }: DisplayLine)}
  {@const sourceLineNum = getLineNumber(line)}
  {@const mermaidBlock = sourceLineNum !== null ? ctx.mermaid.getMermaidBlockAt(sourceLineNum) : null}
  {@const sectionInfo = sourceLineNum !== null ? getSectionAt(sourceLineNum) : null}
  <LineRow
    {line}
    {displayIndex}
  >
    {#snippet gutter()}
      {#if sourceLineNum !== null}
        {sourceLineNum}
      {/if}
    {/snippet}

    {#snippet codeWrapper(innerContent)}
      <span class="code" class:md={markdownMetadata} use:setCodeRef={displayIndex}>
        {@render innerContent()}
      </span>
    {/snippet}

    {#snippet code()}
      {#if line.html?.type === 'full'}{@html line.html.value}{:else}{line.content}{/if}
    {/snippet}

    {#snippet trailing()}
      {#if mermaidBlock}
        <button
          class="line-action mermaid-view-btn"
          onclick={() => ctx.mermaid.openMermaidWindow(mermaidBlock)}
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
          class="line-action copy-section-btn"
        />
      {/if}
    {/snippet}
  </LineRow>
  {@const slot = ctx.slotForRow(displayIndex)}
  <AnnotationSlot slotRef={slot} {...annotationSlotProps} />
{/snippet}

{#snippet walkEntry(entry: DisplayRow)}
  {#if entry.kind === 'hunk-header'}
    {@const hunk = display!.docs[entry.docIdx].doc.hunks[entry.hunkIdx]}
    <LineRow displayIndex={entry.displayIndex} additionalClasses={{ 'diff-header': true }}>
      {#snippet gutter()}
        <span class="diff-gutter-old"></span>
        <span class="diff-gutter-new"></span>
        <span class="diff-sign"></span>
      {/snippet}

      {#snippet codeWrapper(innerContent)}
        <span class="code" use:setCodeRef={entry.displayIndex}>
          {@render innerContent()}
        </span>
      {/snippet}

      {#snippet code()}{hunkHeaderText(hunk)}{/snippet}
    </LineRow>
  {:else if entry.kind === 'row'}
    <LineRow
      displayIndex={entry.displayIndex}
      additionalClasses={{
        'diff-added': entry.rowKind === 'added',
        'diff-deleted': entry.rowKind === 'deleted',
        'diff-context': entry.rowKind === 'context',
      }}
    >
      {#snippet gutter()}
        <span class="diff-gutter-old">{entry.row.old_line ?? ''}</span>
        <span class="diff-gutter-new">{entry.row.new_line ?? ''}</span>
        <span class="diff-sign">{entry.rowKind === 'added' ? '+' : entry.rowKind === 'deleted' ? '-' : ''}</span>
      {/snippet}

      {#snippet codeWrapper(innerContent)}
        <span class="code" use:setCodeRef={entry.displayIndex}>
          {@render innerContent()}
        </span>
      {/snippet}

      {#snippet code()}
        {#if entry.row.html?.type === 'full'}{@html entry.row.html.value}{:else}{entry.row.content}{/if}
      {/snippet}
    </LineRow>
  {/if}
  {@const slot = ctx.slotForRow(entry.displayIndex)}
  <AnnotationSlot slotRef={slot} {...annotationSlotProps} />
{/snippet}

{#if display}
  {#each display.docs as dv (dv.index)}
    {@const collapsed = ctx.fileCollapse.isCollapsed(dv.index)}
    <section class="file-section">
      <FileHeaderRow {dv} {collapsed} onToggle={() => ctx.fileCollapse.toggle(dv.index)} />
      {#if !collapsed}
        {#each docBodies.get(dv.index) ?? [] as entry (entry.displayIndex)}
          {@render walkEntry(entry)}
        {/each}
      {/if}
    </section>
  {/each}
{:else}
  {#each lines as dl (dl.displayIndex)}
    {@render row(dl)}
  {/each}
{/if}

<style>
  /* Mermaid button - extends .line-action */
  .mermaid-view-btn {
    padding: 2px 4px;
    background: var(--bg-window);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
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

  :global(.line:hover .copy-section-btn) {
    opacity: 1;
  }
</style>
