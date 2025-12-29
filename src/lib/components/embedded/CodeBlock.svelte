<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Line, MarkdownMetadata, JSONContent } from '$lib/types';
  import type { Range } from '$lib/range';
  import { getLineNumber, isCodeBlockFence } from '$lib/line-utils';
  import { rangeToKey, isLineInRange } from '$lib/range';
  import Icon from '$lib/CommandPalette/Icon.svelte';

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
  let copied = $state(false);

  // Extract code content (excluding fence lines) for copying
  function getCodeContent(): string {
    return lines
      .filter(({ line }) => !isFence(line))
      .map(({ line }) => line.content)
      .join('\n');
  }

  // Copy code block content to clipboard
  async function copyCodeBlock() {
    const content = getCodeContent();
    try {
      await navigator.clipboard.writeText(content);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }

  // Check if this is the first content line (for no-language blocks)
  function isFirstContentLine(displayIndex: number): boolean {
    const contentLines = lines.filter(({ line }) => !isFence(line));
    return contentLines.length > 0 && contentLines[0].displayIndex === displayIndex;
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
    {@const isPreview = hoveredDisplayIdx === displayIndex && !isDragging}
    <div
      class="line"
      class:codeblock-header={startFence && language}
      class:codeblock-fence={fence && !language}
      class:codeblock-content={!fence}
      class:codeblock-footer={endFence && language}
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
            <span class="codeblock-actions">
              {#if isMermaid && onMermaidOpen}
                <button
                  class="codeblock-action-btn"
                  onclick={onMermaidOpen}
                  title="View diagram"
                >
                  <Icon name="view-finder" />
                </button>
              {/if}
              <button
                class="codeblock-action-btn"
                class:copied
                onclick={copyCodeBlock}
                title={copied ? 'Copied!' : 'Copy code'}
              >
                <Icon name={copied ? 'check' : 'copy-code'} />
              </button>
            </span>
          </span>
        {:else if startFence || endFence}
          <span class="codeblock-footer-info"></span>
        {:else}
          {#if line.html}
            {@html wrapPipes(line.html)}
          {:else}
            {@html wrapPipes(escapeHtml(line.content))}
          {/if}
          {#if !language && isFirstContentLine(displayIndex)}
            <span class="codeblock-inline-actions">
              <button
                class="codeblock-action-btn"
                class:copied
                onclick={copyCodeBlock}
                title={copied ? 'Copied!' : 'Copy code'}
              >
                <Icon name={copied ? 'check' : 'copy-code'} />
              </button>
            </span>
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
    position: relative;
    background:
      var(--codeblock-pattern-bg),
      var(--bg-code-block);
    background-size: var(--codeblock-pattern-size), auto;
  }

  /* Borders only on code area, not gutter */
  .codeblock-group::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--gutter-width);
    right: 0;
    border-top: 1px solid var(--border-code);
    border-bottom: 1px solid var(--border-code);
    pointer-events: none;
  }

  /* Make pipe characters taller so they connect across lines */
  .codeblock-group :global(.pipe) {
    display: inline-block;
    transform: scaleY(1.5);
  }

  .line.codeblock-header .code {
    border-bottom: 1px solid var(--border-subtle);
  }

  .line.codeblock-footer .code {
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
    background: var(--bg-main);
  }

  /* Gutter highlight for selected/preview lines */
  .line.selected .gutter.codeblock-gutter {
    background: var(--selection-bg);
    color: var(--text-secondary);
  }

  .line.preview .gutter.codeblock-gutter {
    background: var(--selection-bg-preview);
    color: var(--text-secondary);
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

  .codeblock-actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    margin-right: 4px;
  }

  .codeblock-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 4px;
    font-size: 16px;
    transition: color 0.15s ease, background 0.15s ease;
  }

  .codeblock-action-btn:hover {
    color: var(--text-secondary);
    background: var(--bg-hover);
  }

  .codeblock-action-btn.copied {
    color: var(--success, #22c55e);
  }

  .codeblock-action-btn:focus-visible {
    outline: 1px solid var(--focus-ring);
    outline-offset: 2px;
  }

  /* Positioning wrapper for inline actions (code blocks without language) */
  .codeblock-inline-actions {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  /* Ensure content lines have relative positioning for inline actions */
  .line.codeblock-content .code {
    position: relative;
  }
</style>
