<script lang="ts">
  /**
   * UnfoldControls - the unfold chevron cluster (S3 unfold), GitHub-style.
   *
   * Hosted inside a gutter: the @@ hunk-header row's gutter for every gap
   * above a hunk, and TrailingGapRow for the fold after the last hunk. Like
   * GitHub, the arrows stack vertically in one cell, each nearest the hunk
   * it grows: ▼ on top continues the hunk above downward (reveals the top
   * of the fold), ▲ below grows the hunk beneath upward (reveals the
   * bottom). A gap of at most one step collapses to a single expand-all
   * button.
   *
   * The gap itself is derived range arithmetic (HunkView.gapAbove /
   * DocView.trailingGap); clicking asks the backend to grow the adjacent
   * hunk and the whole document re-renders.
   *
   * The hunk-header host is an interactive row whose gutter is wired for
   * selection (LineRow's pointerdown/click handlers), so every button stops
   * propagation on both events — chevron click means unfold, never select.
   * Errors are button-local: the cluster tints and the tooltip swaps; there
   * is no inline error text.
   */
  import { tick } from 'svelte';
  import { EXPAND_STEP } from '$lib/display-rows';
  import { FoldUpIcon, FoldDownIcon, UnfoldIcon } from '$lib/icons';

  export type ExpandRequest = { direction: 'up' | 'down'; amount: 'step' | 'all' };

  interface Props {
    /** Folded line count — always positive when the cluster renders. */
    size: number;
    /** A hunk below exists: ▲ reveals lines at the bottom of the gap. */
    showUp: boolean;
    /** A hunk above exists: ▼ reveals lines at the top of the gap. */
    showDown: boolean;
    onExpand: (request: ExpandRequest) => Promise<void>;
  }

  let { size, showUp, showDown, onExpand }: Props = $props();

  let phase = $state<'idle' | 'loading' | 'error'>('idle');
  let root: HTMLSpanElement | null = $state(null);

  const single = $derived(size <= EXPAND_STEP);
  const countLabel = $derived(`${size} unchanged ${size === 1 ? 'line' : 'lines'}`);
  const title = (action: string) =>
    phase === 'error' ? "couldn't expand — try again" : `${action} — ${countLabel}`;

  async function expand(request: ExpandRequest) {
    if (phase === 'loading') return;
    phase = 'loading';
    // Keep the host row visually anchored: content spliced in above the
    // viewport would otherwise shove everything down.
    const scroller = root?.closest('.content');
    const before = root?.getBoundingClientRect().top ?? 0;
    try {
      await onExpand(request);
      phase = 'idle';
    } catch {
      phase = 'error';
    }
    await tick();
    if (root?.isConnected && scroller) {
      const delta = root.getBoundingClientRect().top - before;
      if (delta !== 0) scroller.scrollBy(0, delta);
    }
  }

  // The host row's gutter selects on pointerdown/click; chevrons must not.
  function onclick(e: MouseEvent, request: ExpandRequest) {
    e.stopPropagation();
    expand(request);
  }
  const onpointerdown = (e: PointerEvent) => e.stopPropagation();

  const expandAll = $derived<ExpandRequest>(
    showDown ? { direction: 'down', amount: 'all' } : { direction: 'up', amount: 'all' },
  );
</script>

<span class="unfold-controls" class:loading={phase === 'loading'} class:error={phase === 'error'} bind:this={root}>
  {#if single}
    <button class="unfold-btn" title={title('Expand all')} {onpointerdown} onclick={(e) => onclick(e, expandAll)}>
      <UnfoldIcon />
    </button>
  {:else}
    {#if showDown}
      <button
        class="unfold-btn"
        title={title('Expand down')}
        {onpointerdown}
        onclick={(e) => onclick(e, { direction: 'down', amount: 'step' })}
      >
        <FoldDownIcon />
      </button>
    {/if}
    {#if showUp}
      <button
        class="unfold-btn"
        title={title('Expand up')}
        {onpointerdown}
        onclick={(e) => onclick(e, { direction: 'up', amount: 'step' })}
      >
        <FoldUpIcon />
      </button>
    {/if}
  {/if}
</span>

<style>
  .unfold-controls {
    /* Fill the gutter (a flex row in diff mode) with a GitHub-style cell:
       ▼ over ▲, each button a full-width band one row tall — hovering one
       highlights its whole half of the cell. The negative margin bleeds the
       bands under .gutter's 12px right padding, up to the gutter border. */
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    margin-right: -12px;
  }

  .unfold-controls.loading {
    opacity: 0.6;
  }

  /* diff-header rows disable gutter pointer events wholesale
     (code-viewer.css) — the chevrons opt back in. */
  .unfold-controls,
  .unfold-btn {
    pointer-events: auto;
  }

  .unfold-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    /* One "row" per button: with both arrows the host row is two rows tall
       (the @@ text centers via div.line.diff-header's align-items). */
    min-height: 22px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--accent-blue-hover);
    /* <button> doesn't inherit font-size from ancestors (browser default
       stylesheet gives form controls their own font) — set explicitly to
       match .gutter/.code's 12px, since icons size themselves in `em`
       (tokens.css .cp-icon). */
    font-size: 12px;
    cursor: pointer;
    user-select: none;
  }

  .unfold-btn:hover {
    background: var(--diff-header-bg-hover);
    color: var(--accent-blue);
  }

  .unfold-btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 1px var(--focus-ring);
  }

  .unfold-controls.error .unfold-btn {
    color: var(--error-text);
  }
</style>
