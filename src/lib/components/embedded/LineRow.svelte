<script lang="ts">
  /**
   * LineRow - Shared line-rendering component for embedded content.
   *
   * Handles common concerns across Portal, CodeBlock, and RegularLines:
   * - Selection, annotation, and preview state
   * - Mouse/pointer event handlers
   * - Bookmark indicator
   * - data-display-idx attribute
   *
   * ⚠️ SYNC WARNING: Table.svelte uses <tr>/<td> structure instead of <div>/<span>,
   * so it cannot use this component. When modifying LineRow, check if Table.svelte
   * needs equivalent changes (especially for: selection state, bookmark support,
   * event handlers, new CSS classes).
   */
  import type { Snippet } from 'svelte';
  import type { Line } from '$lib/types';
  import { getAnnotContext } from '$lib/context';
  import { BookmarkIcon } from '$lib/icons';

  interface Props {
    line: Line;
    displayIndex: number;
    isBookmarked?: boolean;
    additionalClasses?: Record<string, boolean>;
    gutterClass?: string;
    gutter: Snippet<[]>;
    code: Snippet<[]>;
    trailing?: Snippet<[]>;
    /** Optional wrapper for the code span. When provided, consumer controls the element and can attach actions. */
    codeWrapper?: Snippet<[Snippet]>;
  }

  let {
    line,
    displayIndex,
    isBookmarked = false,
    additionalClasses = {},
    gutterClass = '',
    gutter,
    code,
    trailing,
    codeWrapper,
  }: Props = $props();

  const ctx = getAnnotContext();

  // Unified state derivation from context
  const selected = $derived(ctx.interaction.isLineHighlighted(displayIndex));
  const annotated = $derived(ctx.annotations.hasAnnotation(displayIndex));
  const preview = $derived(ctx.interaction.isLinePreview(displayIndex));
  const markdownMetadata = $derived(ctx.markdownMetadata);
</script>

<div
  class="line"
  class:selected
  class:annotated
  class:preview
  class:bookmarked={isBookmarked}
  {...additionalClasses}
  data-display-idx={displayIndex}
  onmouseenter={() => ctx.interaction.handleLineEnter(displayIndex)}
  onmouseleave={() => ctx.interaction.handleLineLeave()}
  role="presentation"
>
  <button
    class="add-btn"
    onpointerdown={(e) => ctx.interaction.handlePointerDown(displayIndex, e)}
    aria-label="Add annotation"
  >+</button>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <span
    class="gutter {gutterClass}"
    class:selected
    onpointerdown={(e) => ctx.interaction.handlePointerDown(displayIndex, e)}
    onclick={() => ctx.interaction.handleGutterClick(displayIndex)}
    role="button"
    tabindex="-1"
  >
    {@render gutter()}
  </span>
  {#if codeWrapper}
    {@render codeWrapper(code)}
  {:else}
    <span class="code" class:md={markdownMetadata}>
      {@render code()}
    </span>
  {/if}
  {#if isBookmarked}
    <span class="bookmark-indicator"><BookmarkIcon filled /></span>
  {/if}
  {#if trailing}
    {@render trailing()}
  {/if}
</div>
