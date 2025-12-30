<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Line, MarkdownMetadata, PortalSemantics, JSONContent } from '$lib/types';
  import type { Range } from '$lib/range';
  import { getLineNumber } from '$lib/line-utils';
  import { rangeToKey, isLineInRange } from '$lib/range';

  interface Props {
    lines: Array<{ line: Line; displayIndex: number }>;
    selection: Range | null;
    isDragging: boolean;
    hoveredDisplayIdx: number | null;
    markdownMetadata: MarkdownMetadata | null;
    annotations: Map<string, JSONContent>;
    lastSelectedLine: number | null;

    onGutterMouseDown: (displayIdx: number, event: MouseEvent) => void;
    onGutterClick: (displayIdx: number) => void;
    onAddMouseDown: (displayIdx: number, event: MouseEvent) => void;
    onMouseEnter: (displayIdx: number) => void;
    onMouseLeave: () => void;

    annotationSlot: Snippet<[displayIndex: number, rangeKey: string | null]>;
  }

  let {
    lines,
    selection,
    isDragging,
    hoveredDisplayIdx,
    markdownMetadata,
    annotations,
    lastSelectedLine,
    onGutterMouseDown,
    onGutterClick,
    onAddMouseDown,
    onMouseEnter,
    onMouseLeave,
    annotationSlot,
  }: Props = $props();

  // Get portal semantics from a line
  function getPortalSemantics(line: Line): PortalSemantics | null {
    if (line.semantics.type === 'portal') {
      return line.semantics;
    }
    return null;
  }

  // Check if a display index is selected
  function isSelected(displayIdx: number): boolean {
    if (!selection) return false;
    return isLineInRange(displayIdx, selection);
  }

  // Check if a display index has an annotation
  function hasAnnotation(displayIdx: number): boolean {
    for (const key of annotations.keys()) {
      const [start, end] = key.split('-').map(Number);
      if (displayIdx >= start && displayIdx <= end) {
        return true;
      }
    }
    return false;
  }

  // Get annotation info for a specific display index (is it the last line of any annotation?)
  function getAnnotationAtLine(displayIdx: number): { key: string; content: JSONContent } | null {
    for (const [key, content] of annotations) {
      const [, end] = key.split('-').map(Number);
      if (end === displayIdx) {
        return { key, content };
      }
    }
    return null;
  }

  // Compute the range key for annotation rendering at this line
  function computeRangeKey(displayIndex: number): string | null {
    const annotationAtLine = getAnnotationAtLine(displayIndex);
    if (annotationAtLine) {
      return annotationAtLine.key;
    }

    const isLastSelectedLine = displayIndex === lastSelectedLine && selection && !isDragging;
    if (isLastSelectedLine && selection) {
      return rangeToKey(selection);
    }

    return null;
  }
</script>

<div class="portal-group">
  {#each lines as { line, displayIndex }}
    {@const sourceLineNum = getLineNumber(line)}
    {@const portalSemantics = getPortalSemantics(line)}
    {@const rangeKey = computeRangeKey(displayIndex)}
    {@const isPreview = hoveredDisplayIdx === displayIndex && !isDragging}
    <div
      class="line"
      class:portal-header={portalSemantics?.kind === 'header'}
      class:portal-content={portalSemantics?.kind === 'content'}
      class:portal-footer={portalSemantics?.kind === 'footer'}
      class:selected={isSelected(displayIndex)}
      class:annotated={hasAnnotation(displayIndex)}
      class:preview={isPreview}
      data-display-idx={displayIndex}
      onmouseenter={() => onMouseEnter(displayIndex)}
      onmouseleave={onMouseLeave}
      role="presentation"
    >
      <button
        class="add-btn"
        onmousedown={(e) => onAddMouseDown(displayIndex, e)}
        aria-label="Add annotation"
      >+</button>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <span
        class="gutter portal-gutter"
        class:selected={isSelected(displayIndex)}
        onmousedown={(e) => onGutterMouseDown(displayIndex, e)}
        onclick={() => onGutterClick(displayIndex)}
        role="button"
        tabindex="-1"
      >
        {#if portalSemantics?.kind === 'header'}
          <svg class="portal-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
          </svg>
        {:else if sourceLineNum !== null}
          {sourceLineNum}
        {/if}
      </span>
      <span class="code" class:md={markdownMetadata}>
        {#if portalSemantics?.kind === 'header'}
          <span class="portal-header-info">
            <span class="portal-label">{portalSemantics.label}</span>
            <span class="portal-path">{portalSemantics.path}#{portalSemantics.range}</span>
          </span>
        {:else if line.html?.type === 'full'}
          {@html line.html.value}
        {:else}
          {line.content}
        {/if}
      </span>
    </div>
    {@render annotationSlot(displayIndex, rangeKey)}
  {/each}
</div>

<style>
  /* ===========================================
     Portal Styles
     =========================================== */

  .portal-group {
    background:
      var(--portal-checker-bg),
      var(--bg-portal);
    background-size: var(--portal-checker-size), auto;
    border-top: 1px solid var(--border-portal);
    border-bottom: 1px solid var(--border-portal);
  }

  .line.portal-header {
    background: linear-gradient(to bottom, var(--bg-portal-glow), transparent 25%);
  }

  .line.portal-footer {
    height: 4px;
    min-height: 4px;
    background: linear-gradient(to top, var(--bg-portal-glow), transparent);
  }

  .line.portal-footer .gutter {
    visibility: hidden;
  }

  .line.portal-footer .code {
    display: none;
  }

  .gutter.portal-gutter {
    color: var(--text-muted);
  }

  /* Gutter highlight for selected/preview lines */
  .line.selected .gutter.portal-gutter {
    background: var(--selection-bg);
    color: var(--text-secondary);
  }

  .line.preview .gutter.portal-gutter {
    background: var(--selection-bg-preview);
    color: var(--text-secondary);
  }

  .line.portal-header .gutter.portal-gutter {
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .portal-header-info {
    display: flex;
    align-items: center;
    gap: 0.5em;
    font-size: 0.85em;
    color: var(--text-muted);
  }

  .portal-icon {
    color: var(--border-portal);
  }

  .portal-label {
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-ui);
  }

  .portal-path {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 0.9em;
    opacity: 0.8;
  }

  .portal-path::before {
    content: "—";
    margin-right: 0.5em;
    opacity: 0.5;
  }
</style>
