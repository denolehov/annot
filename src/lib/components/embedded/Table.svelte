<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Line, MarkdownMetadata, JSONContent } from '$lib/types';
  import type { Range } from '$lib/range';
  import { getLineNumber } from '$lib/line-utils';
  import { rangeToKey, isLineInRange } from '$lib/range';
  import {
    analyzeTable,
    splitTableRow,
  } from '$lib/utils/tableParser';

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

  // Analyze table structure
  let tableInfo = $derived(
    analyzeTable(lines.map((l) => ({ content: l.line.content, displayIndex: l.displayIndex })))
  );

  // Check if this is the header row
  function isHeaderRow(lineIndex: number): boolean {
    return tableInfo !== null && lineIndex === tableInfo.headerRow;
  }

  // Check if this is the separator row
  function isSeparator(lineIndex: number): boolean {
    return tableInfo !== null && lineIndex === tableInfo.separatorRow;
  }

  // Filter out separator row
  let visibleLines = $derived(
    lines
      .map((item, idx) => ({ ...item, lineIndex: idx }))
      .filter(({ lineIndex }) => !isSeparator(lineIndex))
  );

  // Calculate column count (for colspan)
  let columnCount = $derived(
    visibleLines[0] ? splitTableRow(visibleLines[0].line.content).length + 1 : 2
  );

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

  // Get annotation info for a specific display index
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

  // Get alignment style for a column
  function getAlignStyle(colIndex: number): string {
    if (!tableInfo) return 'left';
    const align = tableInfo.alignments[colIndex] ?? 'left';
    return align;
  }

  // Track first and last for border styling
  let firstDisplayIndex = $derived(visibleLines[0]?.displayIndex ?? -1);
  let lastDisplayIndex = $derived(visibleLines[visibleLines.length - 1]?.displayIndex ?? -1);
</script>

<div class="table-container">
  <table class="content-table">
    <tbody>
      {#each visibleLines as { line, displayIndex, lineIndex }, rowIdx}
        {@const sourceLineNum = getLineNumber(line)}
        {@const rangeKey = computeRangeKey(displayIndex)}
        {@const cells = splitTableRow(line.content)}
        {@const isHeader = isHeaderRow(lineIndex)}
        {@const isFirst = displayIndex === firstDisplayIndex}
        {@const isLast = displayIndex === lastDisplayIndex}

        <tr
          class="content-row"
          class:selected={isSelected(displayIndex)}
          class:annotated={hasAnnotation(displayIndex)}
          class:table-header-row={isHeader}
          class:table-first-row={isFirst}
          class:table-last-row={isLast}
          data-display-idx={displayIndex}
          onmouseenter={() => onMouseEnter(displayIndex)}
          onmouseleave={onMouseLeave}
        >
          <td class="gutter-cell">
            <button
              class="add-btn"
              onmousedown={(e) => onAddMouseDown(displayIndex, e)}
              aria-label="Add annotation"
            >+</button>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <span
              class="gutter"
              class:selected={isSelected(displayIndex)}
              onmousedown={(e) => onGutterMouseDown(displayIndex, e)}
              onclick={() => onGutterClick(displayIndex)}
              role="button"
              tabindex="-1"
            >
              {#if sourceLineNum !== null}
                {sourceLineNum}
              {/if}
            </span>
          </td>
          {#each cells as cell, colIndex}
            {#if isHeader}
              <th class="table-cell" style:text-align={getAlignStyle(colIndex)}>{cell}</th>
            {:else}
              <td class="table-cell" style:text-align={getAlignStyle(colIndex)}>{cell}</td>
            {/if}
          {/each}
        </tr>

        {#if rangeKey}
          <tr class="annotation-row">
            <td colspan={columnCount} class="annotation-cell">
              {@render annotationSlot(displayIndex, rangeKey)}
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
</div>

<style>
  .table-container {
    overflow-x: auto;
    position: relative;
  }

  .content-table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 22px;
    background:
      var(--chip-pattern-bg),
      var(--bg-code-block);
    background-size: var(--chip-pattern-size), auto;
  }

  .content-row {
    height: 22px;
  }

  .content-row.selected,
  .content-row.annotated {
    background: var(--selection-bg);
  }

  /* Sticky gutter cell */
  .gutter-cell {
    position: sticky;
    left: 0;
    z-index: 1;
    width: var(--gutter-width);
    min-width: var(--gutter-width);
    padding: 0;
    background: var(--bg-main);
    border-right: 1px solid var(--border-subtle);
    vertical-align: top;
  }

  .content-row.selected .gutter-cell,
  .content-row.annotated .gutter-cell {
    background: var(--selection-bg);
  }

  .gutter-cell :global(.gutter) {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    width: 100%;
    height: 22px;
    border-right: none;
    background: inherit;
  }

  /* Content cells */
  .table-cell {
    height: 22px;
    padding: 0 12px;
    white-space: pre;
    vertical-align: middle;
  }

  /* First row top border */
  .table-first-row .table-cell {
    border-top: 1px solid var(--border-code);
  }

  /* Last row bottom border */
  .table-last-row .table-cell {
    border-bottom: 1px solid var(--border-code);
  }

  /* Header row styling */
  .table-header-row .table-cell {
    font-weight: 600;
    background:
      url("data:image/svg+xml,%3Csvg width='4' height='4' xmlns='http://www.w3.org/2000/svg'%3E%3Ccircle cx='1' cy='1' r='0.75' fill='rgba(140,120,80,0.35)'/%3E%3Ccircle cx='3' cy='3' r='0.75' fill='rgba(140,120,80,0.35)'/%3E%3C/svg%3E"),
      var(--bg-code-block);
    background-size: 4px 4px, auto;
    border-bottom: 1px solid var(--border-strong);
  }

  /* Annotation row */
  .annotation-row {
    background:
      var(--chip-pattern-bg),
      var(--bg-code-block);
    background-size: var(--chip-pattern-size), auto;
  }

  .annotation-cell {
    padding: 0;
  }

  /* Add button */
  .add-btn {
    position: absolute;
    top: 50%;
    right: -9px;
    transform: translateY(-50%);
    width: 18px;
    height: 18px;
    background: var(--selection-border);
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 16px;
    font-weight: 400;
    cursor: pointer;
    display: none;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    padding: 0;
    padding-bottom: 2px;
    line-height: 0;
    -webkit-user-select: none;
    user-select: none;
    z-index: 2;
  }

  .add-btn:hover {
    transform: translateY(-50%) scale(1.1);
    box-shadow: 0 3px 6px rgba(0,0,0,0.15);
  }

  .content-row:not(.selected):hover .add-btn {
    display: flex;
  }
</style>
