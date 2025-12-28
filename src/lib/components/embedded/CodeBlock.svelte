<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Line, MarkdownMetadata, JSONContent } from '$lib/types';
  import type { Range } from '$lib/range';
  import { getLineNumber, isCodeBlockFence } from '$lib/line-utils';
  import { rangeToKey, isLineInRange } from '$lib/range';

  interface Props {
    lines: Array<{ line: Line; displayIndex: number }>;
    language: string | null;
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
    onMermaidOpen?: () => void;

    annotationSlot: Snippet<[displayIndex: number, rangeKey: string | null]>;
  }

  let {
    lines,
    language,
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
    onMermaidOpen,
    annotationSlot,
  }: Props = $props();

  let isMermaid = $derived(language === 'mermaid');

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

  // Check if line is a fence (start or end)
  function isFence(line: Line): boolean {
    return isCodeBlockFence(line);
  }

  // Check if line is the start fence
  function isStartFence(line: Line): boolean {
    return line.semantics.type === 'markdown' && line.semantics.kind === 'code_block_start';
  }

  // Check if line is the end fence
  function isEndFence(line: Line): boolean {
    return line.semantics.type === 'markdown' && line.semantics.kind === 'code_block_end';
  }

  // Wrap box-drawing characters in a span for CSS scaling
  // Covers: | │ ├ ┤ ┬ ┴ ┼ ┌ ┐ └ ┘ and dashed variants ┄ ┆ ┊
  function wrapPipes(text: string): string {
    return text.replace(/[|│├┤┬┴┼┌┐└┘┄┆┊]/g, '<span class="pipe">$&</span>');
  }

  // Escape HTML entities for safe rendering
  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }
</script>

<div class="codeblock-group">
  {#each lines as { line, displayIndex }}
    {@const sourceLineNum = getLineNumber(line)}
    {@const rangeKey = computeRangeKey(displayIndex)}
    {@const fence = isFence(line)}
    {@const startFence = isStartFence(line)}
    {@const endFence = isEndFence(line)}
    <div
      class="line"
      class:codeblock-header={startFence && language}
      class:codeblock-fence={fence && !language}
      class:codeblock-content={!fence}
      class:codeblock-footer={endFence && language}
      class:selected={isSelected(displayIndex)}
      class:annotated={hasAnnotation(displayIndex)}
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
        class="gutter codeblock-gutter"
        class:selected={isSelected(displayIndex)}
        onmousedown={(e) => onGutterMouseDown(displayIndex, e)}
        onclick={() => onGutterClick(displayIndex)}
        role="button"
        tabindex="-1"
      >
        {#if !endFence && sourceLineNum !== null}
          {sourceLineNum}
        {/if}
      </span>
      <span class="code" class:md={markdownMetadata}>
        {#if startFence && language}
          <span class="codeblock-header-info">
            <span class="lang-badge">{language}</span>
            {#if isMermaid && onMermaidOpen}
              <button
                class="mermaid-view-btn"
                onclick={onMermaidOpen}
                title="View diagram"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" width="18" height="18">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M7.5 3.75H6A2.25 2.25 0 0 0 3.75 6v1.5M16.5 3.75H18A2.25 2.25 0 0 1 20.25 6v1.5m0 9V18A2.25 2.25 0 0 1 18 20.25h-1.5m-9 0H6A2.25 2.25 0 0 1 3.75 18v-1.5M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
                </svg>
              </button>
            {/if}
          </span>
        {:else if startFence || endFence}
          <span class="codeblock-footer-info"></span>
        {:else}
          {#if line.html}
            {@html wrapPipes(line.html)}
          {:else}
            {@html wrapPipes(escapeHtml(line.content))}
          {/if}
        {/if}
      </span>
    </div>
    {#if !fence}
      {@render annotationSlot(displayIndex, rangeKey)}
    {/if}
  {/each}
</div>

<style>
  /* ===========================================
     Code Block Styles
     =========================================== */

  .codeblock-group {
    background:
      var(--codeblock-pattern-bg),
      var(--bg-code-block);
    background-size: var(--codeblock-pattern-size), auto;
    border-top: 1px solid var(--border-code);
    border-bottom: 1px solid var(--border-code);
  }

  /* Make pipe characters taller so they connect across lines */
  .codeblock-group :global(.pipe) {
    display: inline-block;
    transform: scaleY(1.5);
  }

  .line.codeblock-header {
    border-bottom: 1px solid var(--border-subtle);
  }

  .line.codeblock-footer {
    border-top: 1px solid var(--border-subtle);
  }

  /* Fence lines (header/footer with language, or any fence without): minimal height */
  .line.codeblock-fence,
  .line.codeblock-header,
  .line.codeblock-footer {
    height: auto;
    min-height: 0;
  }

  .line.codeblock-fence .gutter,
  .line.codeblock-fence .code,
  .line.codeblock-footer .gutter,
  .line.codeblock-footer .code {
    display: none;
  }

  /* Hide add button for fence lines */
  .line.codeblock-header .add-btn,
  .line.codeblock-footer .add-btn,
  .line.codeblock-fence .add-btn {
    display: none !important;
  }

  .gutter.codeblock-gutter {
    color: var(--text-muted);
  }

  .line.codeblock-header .gutter.codeblock-gutter {
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .codeblock-header-info {
    display: flex;
    align-items: center;
    gap: 0.5em;
    font-size: 0.85em;
    width: 100%;
  }

  .lang-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .lang-badge::before {
    content: "";
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .mermaid-view-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    margin-left: auto;
    margin-right: 4px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .mermaid-view-btn:hover {
    color: var(--accent);
  }

  .mermaid-view-btn:focus-visible {
    outline: 1px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .mermaid-view-btn svg {
    display: block;
  }
</style>
