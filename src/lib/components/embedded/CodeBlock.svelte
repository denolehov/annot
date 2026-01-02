<script lang="ts">
  /**
   * CodeBlock - Renders fenced code blocks with syntax highlighting.
   * Uses LineRow for shared line-rendering logic and adds codeblock-specific styling.
   */
  import type { Snippet } from 'svelte';
  import type { Line } from '$lib/types';
  import { getLineNumber, isCodeBlockFence } from '$lib/line-utils';
  import { computePosition, offset, flip, shift } from '@floating-ui/dom';
  import Icon from '$lib/CommandPalette/Icon.svelte';
  import { getAnnotContext } from '$lib/context';
  import LineRow from './LineRow.svelte';

  interface Props {
    lines: Array<{ line: Line; displayIndex: number }>;
    language: string | null;
    color: string | null;
    isLineBookmarked?: (displayIdx: number) => boolean;
    onMermaidOpen?: () => void;
    onExcalidrawOpen?: () => void;
    excalidrawSupported?: boolean;
    mermaidError?: string | null;
    onReportMermaidError?: (error: string) => void;
    annotationSlot: Snippet<[displayIndex: number, rangeKey: string | null]>;
  }

  let {
    lines,
    language,
    color,
    isLineBookmarked,
    onMermaidOpen,
    onExcalidrawOpen,
    excalidrawSupported = true,
    mermaidError = null,
    onReportMermaidError,
    annotationSlot,
  }: Props = $props();

  const ctx = getAnnotContext();

  let isMermaid = $derived(language === 'mermaid');
  let copied = $state(false);

  // Mermaid error popover state
  let errorPopoverOpen = $state(false);
  let errorBtnEl: HTMLButtonElement | undefined = $state();
  let errorPopoverEl: HTMLDivElement | undefined = $state();

  // Position popover when it opens
  $effect(() => {
    if (!errorPopoverOpen || !errorBtnEl || !errorPopoverEl) return;

    async function updatePosition() {
      if (!errorBtnEl || !errorPopoverEl) return;
      const { x, y } = await computePosition(errorBtnEl, errorPopoverEl, {
        placement: 'bottom-start',
        middleware: [
          offset(4),
          flip({ padding: 8 }),
          shift({ padding: 8 }),
        ],
      });
      Object.assign(errorPopoverEl.style, {
        left: `${x}px`,
        top: `${y}px`,
      });
    }

    updatePosition();
  });

  function handleAddToFeedback() {
    if (mermaidError && onReportMermaidError) {
      onReportMermaidError(mermaidError);
    }
    errorPopoverOpen = false;
  }

  function handlePopoverKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      errorPopoverOpen = false;
    }
  }

  function handlePopoverClickOutside(e: MouseEvent) {
    if (!errorPopoverOpen) return;
    const target = e.target as HTMLElement;
    if (!target.closest('.mermaid-error-popover-container')) {
      errorPopoverOpen = false;
    }
  }

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

<svelte:window onkeydown={handlePopoverKeydown} onclick={handlePopoverClickOutside} />

<div class="codeblock-group">
  {#each lines as { line, displayIndex }}
    {@const sourceLineNum = getLineNumber(line)}
    {@const rangeKey = ctx.interaction.pendingChoice ? null : ctx.getRangeKeyForLine(displayIndex)}
    {@const fence = isFence(line)}
    {@const startFence = isStartFence(line)}
    {@const endFence = isEndFence(line)}
    <LineRow
      {line}
      {displayIndex}
      isBookmarked={isLineBookmarked?.(displayIndex)}
      additionalClasses={{
        'codeblock-header': startFence && !!language,
        'codeblock-fence': fence && !language,
        'codeblock-content': !fence,
        'codeblock-footer': endFence && !!language,
      }}
      gutterClass="codeblock-gutter"
    >
      {#snippet gutter()}
        {#if !endFence && sourceLineNum !== null}
          {sourceLineNum}
        {/if}
      {/snippet}

      {#snippet code()}
        {#if startFence && language}
          <span class="codeblock-header-info">
            <span class="lang-badge" style:--lang-color={color}>{language}</span>
            <span class="codeblock-actions">
              {#if isMermaid}
                {#if mermaidError}
                  <div class="mermaid-error-popover-container">
                    <button
                      bind:this={errorBtnEl}
                      class="codeblock-action-btn mermaid-error-btn"
                      onclick={() => (errorPopoverOpen = !errorPopoverOpen)}
                      title="Syntax error - click for details"
                    >
                      <Icon name="warning" />
                    </button>
                    {#if errorPopoverOpen}
                      <div bind:this={errorPopoverEl} class="mermaid-error-popover">
                        <pre class="error-text">{mermaidError}</pre>
                        <button class="feedback-btn" onclick={handleAddToFeedback}>
                          Add to feedback
                        </button>
                      </div>
                    {/if}
                  </div>
                {:else if onMermaidOpen}
                  <button
                    class="codeblock-action-btn"
                    onclick={onMermaidOpen}
                    title="View diagram"
                  >
                    <Icon name="view-finder" />
                  </button>
                {/if}
              {/if}
              {#if isMermaid}
                <button
                  class="codeblock-action-btn"
                  onclick={onExcalidrawOpen}
                  disabled={!excalidrawSupported || !!mermaidError}
                  title={mermaidError
                    ? "Fix syntax error before editing in Excalidraw"
                    : excalidrawSupported
                      ? "Edit in Excalidraw"
                      : "Only flowchart, sequence, and class diagrams can be edited in Excalidraw"}
                >
                  <Icon name="excalidraw" />
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
          {#if line.html?.type === 'full'}
            {@html wrapPipes(line.html.value)}
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
      {/snippet}
    </LineRow>
    {#if !fence}
      <div class="annotation-row">
        <span class="annotation-gutter"></span>
        {@render annotationSlot(displayIndex, rangeKey)}
      </div>
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

  /* Styles targeting LineRow-rendered elements need :global() */
  .codeblock-group :global(.line.codeblock-header .code) {
    border-bottom: 1px solid var(--border-subtle);
  }

  .codeblock-group :global(.line.codeblock-footer .code) {
    border-top: 1px solid var(--border-subtle);
  }

  /* Fence lines (header/footer with language, or any fence without): minimal height */
  .codeblock-group :global(.line.codeblock-fence),
  .codeblock-group :global(.line.codeblock-header),
  .codeblock-group :global(.line.codeblock-footer) {
    height: auto;
    min-height: 0;
  }

  .codeblock-group :global(.line.codeblock-fence .gutter),
  .codeblock-group :global(.line.codeblock-fence .code),
  .codeblock-group :global(.line.codeblock-footer .gutter),
  .codeblock-group :global(.line.codeblock-footer .code) {
    display: none;
  }

  /* Hide add button for fence lines */
  .codeblock-group :global(.line.codeblock-header .add-btn),
  .codeblock-group :global(.line.codeblock-footer .add-btn),
  .codeblock-group :global(.line.codeblock-fence .add-btn) {
    display: none !important;
  }

  .codeblock-group :global(.gutter.codeblock-gutter) {
    color: var(--text-muted);
    background: var(--bg-main);
  }

  /* Gutter highlight for selected/preview lines */
  .codeblock-group :global(.line.selected .gutter.codeblock-gutter),
  .codeblock-group :global(.line.annotated .gutter.codeblock-gutter) {
    background: var(--selection-bg);
    color: var(--text-secondary);
  }

  .codeblock-group :global(.line.preview .gutter.codeblock-gutter) {
    background: var(--selection-bg-preview);
    color: var(--text-secondary);
  }

  .codeblock-group :global(.line.codeblock-header .gutter.codeblock-gutter) {
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
    background: var(--lang-color, var(--accent));
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

  .codeblock-action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .codeblock-action-btn:disabled:hover {
    color: var(--text-muted);
    background: transparent;
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
  .codeblock-group :global(.line.codeblock-content .code) {
    position: relative;
  }

  /* Mermaid error popover styles */
  .mermaid-error-popover-container {
    position: relative;
    display: inline-flex;
  }

  .mermaid-error-btn {
    color: var(--warning, #f97316) !important;
  }

  .mermaid-error-btn:hover {
    color: var(--warning, #f97316) !important;
    background: color-mix(in srgb, var(--warning, #f97316) 15%, transparent) !important;
  }

  .mermaid-error-popover {
    position: fixed;
    top: 0;
    left: 0;
    background: var(--bg-window);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 12px;
    min-width: 280px;
    max-width: 400px;
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.08),
      0 1px 3px rgba(0, 0, 0, 0.06);
    z-index: 1000;
    animation: popover-enter 150ms ease;
  }

  @keyframes popover-enter {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .mermaid-error-popover .error-text {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-panel);
    border-radius: 4px;
    padding: 8px;
    margin: 0 0 8px 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 150px;
    overflow-y: auto;
  }

  .mermaid-error-popover .feedback-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: 8px 12px;
    background: var(--warning, #f97316);
    border: none;
    border-radius: 6px;
    color: white;
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    transition: opacity 0.15s ease;
  }

  .mermaid-error-popover .feedback-btn:hover {
    opacity: 0.9;
  }

  .mermaid-error-popover .feedback-btn:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  /* Annotation row - structural gutter for annotations inside code blocks */
  .annotation-row {
    display: flex;
  }

  .annotation-gutter {
    width: var(--gutter-width);
    flex-shrink: 0;
    background: var(--bg-main);
    border-right: 1px solid var(--border-subtle);
  }

  /* Override annotation editor margin when inside code block */
  .annotation-row :global(.annotation-editor) {
    flex: 1;
    margin-left: 8px;
  }
</style>
