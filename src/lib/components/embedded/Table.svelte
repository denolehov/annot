<script lang="ts">
  /**
   * Table - Renders markdown tables with proper styling.
   * Uses context for: interaction, annotations
   *
   * ⚠️ SYNC WARNING: This component uses <tr>/<td> structure instead of <div>/<span>,
   * so it cannot use LineRow.svelte. When LineRow is modified (especially for:
   * selection state, bookmark support, event handlers, new CSS classes), check if
   * equivalent changes are needed here.
   */
  import type { Snippet } from 'svelte';
  import type { Line } from '$lib/types';
  import { getLineNumber } from '$lib/line-utils';
  import {
    analyzeTable,
    splitTableRow,
  } from '$lib/utils/tableParser';
  import { getAnnotContext } from '$lib/context';
  import { highlightMatches, clearHighlights } from '$lib/search-highlight';
  import ChoiceButtons from '$lib/components/ChoiceButtons.svelte';

  interface Props {
    lines: Array<{ line: Line; displayIndex: number }>;
    isLineBookmarked?: (displayIdx: number) => boolean;
    isFirstLineOfBookmark?: (displayIdx: number) => boolean;
    deleteBookmarkAtLine?: (displayIdx: number) => void;
    annotationSlot: Snippet<[displayIndex: number, rangeKey: string | null]>;
  }

  let { lines, isLineBookmarked, isFirstLineOfBookmark, deleteBookmarkAtLine, annotationSlot }: Props = $props();

  const ctx = getAnnotContext();

  // Search highlighting state - track refs per cell to avoid cross-cell DOM manipulation
  const searchMatches = $derived(ctx.search.matches);
  let cellRefs: Map<string, HTMLElement> = new Map();

  // Svelte action to track cell element refs for search highlighting
  // Key format: "displayIndex-colIndex"
  function setCellRef(el: HTMLElement, key: { displayIndex: number; colIndex: number }) {
    const refKey = `${key.displayIndex}-${key.colIndex}`;
    cellRefs.set(refKey, el);
    return {
      destroy() {
        cellRefs.delete(refKey);
      },
    };
  }

  // Apply search highlights when matches change
  $effect(() => {
    // Clear all previous highlights first
    for (const el of cellRefs.values()) {
      clearHighlights(el);
    }

    // Apply highlights per cell
    const currentSearchMatch = ctx.search.getCurrentMatch();
    for (const match of searchMatches) {
      // Find the line data for this match
      const lineData = lines.find(l => l.displayIndex === match.displayIndex);
      if (!lineData) continue;

      const cells = splitTableRow(lineData.line.content);
      const isCurrent = currentSearchMatch?.displayIndex === match.displayIndex;

      // Calculate cell boundaries (cumulative offsets)
      const cellBoundaries: Array<{ start: number; end: number }> = [];
      let offset = 0;
      for (const cell of cells) {
        const cellText = lineData.line.html?.type === 'cells'
          ? (() => { const div = document.createElement('div'); div.innerHTML = lineData.line.html.value[cellBoundaries.length]; return div.textContent ?? cell; })()
          : cell;
        cellBoundaries.push({ start: offset, end: offset + cellText.length });
        offset += cellText.length;
      }

      // For each match range, split it across cells
      for (let rangeIdx = 0; rangeIdx < match.ranges.length; rangeIdx++) {
        const range = match.ranges[rangeIdx];
        const isCurrentRange = isCurrent && rangeIdx === 0;

        for (let colIndex = 0; colIndex < cellBoundaries.length; colIndex++) {
          const cell = cellBoundaries[colIndex];

          // Check if this range overlaps with this cell
          if (range.end <= cell.start || range.start >= cell.end) continue;

          // Calculate the intersection
          const cellRangeStart = Math.max(0, range.start - cell.start);
          const cellRangeEnd = Math.min(cell.end - cell.start, range.end - cell.start);

          const el = cellRefs.get(`${match.displayIndex}-${colIndex}`);
          if (el) {
            highlightMatches(el, [{ start: cellRangeStart, end: cellRangeEnd }], isCurrentRange ? 0 : null);
          }
        }
      }
    }
  });

  // Scroll state for edge shadows
  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);

  function setupScrollIndicators(node: HTMLDivElement) {
    const updateScrollState = () => {
      const { scrollLeft, scrollWidth, clientWidth } = node;
      canScrollLeft = scrollLeft > 0;
      canScrollRight = scrollLeft + clientWidth < scrollWidth - 1;
    };

    updateScrollState();
    node.addEventListener('scroll', updateScrollState, { passive: true });

    const ro = new ResizeObserver(updateScrollState);
    ro.observe(node);

    return {
      destroy() {
        node.removeEventListener('scroll', updateScrollState);
        ro.disconnect();
      }
    };
  }

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
    return ctx.interaction.isLineHighlighted(displayIdx);
  }

  // Check if a display index has an annotation
  function hasAnnotation(displayIdx: number): boolean {
    return ctx.annotations.hasAnnotation(displayIdx);
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

  // Check if choice buttons should show for a given display index
  function shouldShowChoiceButtons(displayIdx: number): boolean {
    if (!ctx.interaction.pendingChoice || ctx.interaction.range === null) return false;
    const rangeEnd = Math.max(ctx.interaction.range.start, ctx.interaction.range.end);
    return displayIdx === rangeEnd;
  }

  function handleChooseAnnotate() {
    ctx.interaction.confirmChoice('annotate');
  }

  function handleChooseBookmark() {
    ctx.interaction.confirmChoice('bookmark');
  }
</script>

<div class="table-wrapper" class:can-scroll-left={canScrollLeft} class:can-scroll-right={canScrollRight} class:is-dragging={ctx.isDragging}>
  <div class="table-scroller" use:setupScrollIndicators>
    <table class="content-table">
      <tbody>
        {#each visibleLines as { line, displayIndex, lineIndex }, rowIdx}
          {@const sourceLineNum = getLineNumber(line)}
          {@const rangeKey = ctx.getRangeKeyForLine(displayIndex)}
          {@const cells = splitTableRow(line.content)}
          {@const isHeader = isHeaderRow(lineIndex)}
          {@const isFirst = displayIndex === firstDisplayIndex}
          {@const isLast = displayIndex === lastDisplayIndex}
          {@const bookmarked = isLineBookmarked?.(displayIndex)}

          {@const isPreview = ctx.interaction.isLinePreview(displayIndex)}
          <tr
            class="content-row"
            class:selected={isSelected(displayIndex)}
            class:annotated={hasAnnotation(displayIndex)}
            class:preview={isPreview}
            class:bookmarked
            class:table-header-row={isHeader}
            class:table-first-row={isFirst}
            class:table-last-row={isLast}
            data-display-idx={displayIndex}
            onmouseenter={() => ctx.interaction.handleLineEnter(displayIndex)}
            onmouseleave={() => ctx.interaction.handleLineLeave()}
          >
            <td class="gutter-cell">
              <button
                class="add-btn"
                onpointerdown={(e) => ctx.interaction.handlePointerDown(displayIndex, e)}
                aria-label="Add annotation"
              >+</button>
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <span
                class="gutter"
                class:selected={isSelected(displayIndex)}
                onpointerdown={(e) => ctx.interaction.handlePointerDown(displayIndex, e)}
                onclick={() => ctx.interaction.handleGutterClick(displayIndex)}
                role="button"
                tabindex="-1"
              >
                {#if sourceLineNum !== null}
                  {sourceLineNum}
                {/if}
              </span>
            </td>
            {#each cells as cell, colIndex}
              {@const cellHtml = line.html?.type === 'cells' ? line.html.value[colIndex] : null}
              {#if isHeader}
                <th class="table-cell" style:text-align={getAlignStyle(colIndex)}>
                  <span class="cell-content" use:setCellRef={{ displayIndex, colIndex }}>
                    {#if cellHtml}{@html cellHtml}{:else}{cell}{/if}
                  </span>
                </th>
              {:else}
                <td class="table-cell" style:text-align={getAlignStyle(colIndex)}>
                  <span class="cell-content" use:setCellRef={{ displayIndex, colIndex }}>
                    {#if cellHtml}{@html cellHtml}{:else}{cell}{/if}
                  </span>
                </td>
              {/if}
            {/each}
          </tr>

          {#if rangeKey}
            <tr class="annotation-row">
              <td class="gutter-cell annotation-gutter"></td>
              <td colspan={columnCount - 1} class="annotation-cell">
                {@render annotationSlot(displayIndex, rangeKey)}
              </td>
            </tr>
          {/if}

          {#if shouldShowChoiceButtons(displayIndex)}
            <tr class="choice-buttons-row">
              <td class="gutter-cell"></td>
              <td colspan={columnCount - 1} class="choice-buttons-cell">
                <ChoiceButtons onAnnotate={handleChooseAnnotate} onBookmark={handleChooseBookmark} />
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .table-wrapper {
    position: relative;
  }

  .table-scroller {
    overflow-x: auto;
    overscroll-behavior-x: none;

    /* Hide native scrollbar */
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .table-scroller::-webkit-scrollbar {
    display: none;
  }

  /* Fade effects using pseudo-elements instead of mask-image
     This keeps the add button visible above the fade */
  .table-wrapper::before,
  .table-wrapper::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    width: 40px;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s ease;
    z-index: 3;
  }

  .table-wrapper::before {
    left: var(--gutter-width);
    background: linear-gradient(to right, var(--bg-code-block), transparent);
  }

  .table-wrapper::after {
    right: 0;
    background: linear-gradient(to left, var(--bg-code-block), transparent);
  }

  .table-wrapper.can-scroll-left::before {
    opacity: 1;
  }

  .table-wrapper.can-scroll-right::after {
    opacity: 1;
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

  /* Preview highlight (hover state - lighter than selection) */
  .content-row.preview {
    background-color: var(--selection-bg-preview);
  }

  .content-row.selected,
  .content-row.annotated {
    background: var(--selection-bg);
  }

  /* Sticky gutter cell - z-index: 4 to stay above scroll fade (z-index: 3) */
  .gutter-cell {
    position: sticky;
    left: 0;
    z-index: 4;
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

  /* Cell content wrapper for search highlighting */
  .cell-content {
    display: inline;
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

  .annotation-gutter {
    vertical-align: top;
  }

  .annotation-cell {
    position: sticky;
    left: var(--gutter-width);
    width: calc(100vw - var(--gutter-width) - 24px);
    max-width: calc(100vw - var(--gutter-width) - 24px);
    padding: 0;
    background: inherit;
  }

  .annotation-cell :global(.annotation-editor) {
    margin-left: 8px;
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

  /* Show add button on preview rows (hover state) */
  .content-row.preview .add-btn {
    display: flex;
  }

  /* Bookmarked rows - border + background tint (no icon, tables are dense enough) */
  .content-row.bookmarked .gutter-cell::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--bookmark-color, #ef4444);
    z-index: 5;
  }

  .content-row.bookmarked .gutter-cell,
  .content-row.bookmarked .table-cell {
    background-color: color-mix(in srgb, var(--bookmark-color, #ef4444) 8%, transparent);
  }

  /* Choice buttons row */
  .choice-buttons-row {
    background:
      var(--chip-pattern-bg),
      var(--bg-code-block);
    background-size: var(--chip-pattern-size), auto;
  }

  .choice-buttons-cell {
    padding: 4px 0 4px 8px;
  }
</style>
