<script lang="ts">
  /**
   * Portal - Renders embedded file snippets (portals).
   * Uses LineRow for shared line-rendering logic and adds portal-specific styling.
   */
  import type { Snippet } from 'svelte';
  import type { Line, PortalSemantics } from '$lib/types';
  import { getLineNumber } from '$lib/line-utils';
  import { getAnnotContext } from '$lib/context';
  import LineRow from './LineRow.svelte';

  interface Props {
    lines: Array<{ line: Line; displayIndex: number }>;
    isLineBookmarked?: (displayIdx: number) => boolean;
    annotationSlot: Snippet<[displayIndex: number, rangeKey: string | null]>;
  }

  let { lines, isLineBookmarked, annotationSlot }: Props = $props();

  const ctx = getAnnotContext();

  // Get portal semantics from a line
  function getPortalSemantics(line: Line): PortalSemantics | null {
    if (line.semantics.type === 'portal') {
      return line.semantics;
    }
    return null;
  }
</script>

<div class="portal-group">
  {#each lines as { line, displayIndex }}
    {@const sourceLineNum = getLineNumber(line)}
    {@const portalSemantics = getPortalSemantics(line)}
    {@const rangeKey = ctx.interaction.pendingChoice ? null : ctx.getRangeKeyForLine(displayIndex)}
    <LineRow
      {line}
      {displayIndex}
      isBookmarked={isLineBookmarked?.(displayIndex)}
      additionalClasses={{
        'portal-header': portalSemantics?.kind === 'header',
        'portal-content': portalSemantics?.kind === 'content',
        'portal-footer': portalSemantics?.kind === 'footer',
      }}
      gutterClass="portal-gutter"
    >
      {#snippet gutter()}
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
      {/snippet}

      {#snippet code()}
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
      {/snippet}
    </LineRow>
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

  /* Styles targeting LineRow-rendered elements need :global() */
  .portal-group :global(.line.portal-header) {
    background: linear-gradient(to bottom, var(--bg-portal-glow), transparent 25%);
  }

  .portal-group :global(.line.portal-footer) {
    height: 4px;
    min-height: 4px;
    background: linear-gradient(to top, var(--bg-portal-glow), transparent);
  }

  .portal-group :global(.line.portal-footer .gutter) {
    visibility: hidden;
  }

  .portal-group :global(.line.portal-footer .code) {
    display: none;
  }

  .portal-group :global(.gutter.portal-gutter) {
    color: var(--text-muted);
  }

  /* Gutter highlight for selected/preview lines */
  .portal-group :global(.line.selected .gutter.portal-gutter),
  .portal-group :global(.line.annotated .gutter.portal-gutter) {
    background: var(--selection-bg);
    color: var(--text-secondary);
  }

  .portal-group :global(.line.preview .gutter.portal-gutter) {
    background: var(--selection-bg-preview);
    color: var(--text-secondary);
  }

  .portal-group :global(.line.portal-header .gutter.portal-gutter) {
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
