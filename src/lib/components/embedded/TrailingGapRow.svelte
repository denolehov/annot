<script lang="ts">
  /**
   * TrailingGapRow - standalone host for the fold after a file's last hunk.
   *
   * Every other gap inlines its UnfoldControls into the @@ hunk-header row
   * below it; the trailing gap has no header row below, so it keeps a thin
   * row of its own. Pure affordance: renders through
   * LineRow(interactive=false), so it carries no displayIndex and can't be
   * selected or annotated — never hand it one.
   */
  import LineRow from './LineRow.svelte';
  import UnfoldControls, { type ExpandRequest } from './UnfoldControls.svelte';

  interface Props {
    /** Folded line count — always positive when the row renders. */
    size: number;
    onExpand: (request: ExpandRequest) => Promise<void>;
  }

  let { size, onExpand }: Props = $props();
</script>

<!-- 'diff-header' borrows the @@ row's blue tint / hover / chevron pointer-events
     treatment from code-viewer.css — same fold affordance, no header text. -->
<LineRow interactive={false} additionalClasses={{ 'gap-line': true, 'diff-header': true }}>
  {#snippet gutter()}
    <!-- ▼ only: the hunk above grows downward into the fold. -->
    <UnfoldControls {size} showUp={false} showDown={true} {onExpand} />
  {/snippet}

  {#snippet code()}{/snippet}
</LineRow>

<style>
  :global(.line.gap-line) {
    color: var(--text-secondary);
  }
</style>
