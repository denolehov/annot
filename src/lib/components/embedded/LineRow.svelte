<script lang="ts">
  /**
   * LineRow - Shared line-rendering component for embedded content.
   *
   * Handles common concerns across Portal, CodeBlock, and RegularLines:
   * - Selection, annotation, and preview state
   * - Mouse/pointer event handlers
   * - data-display-idx attribute
   *
   * ⚠️ SYNC WARNING: Table.svelte uses <tr>/<td> structure instead of <div>/<span>,
   * so it cannot use this component. When modifying LineRow, check if Table.svelte
   * needs equivalent changes (especially for: selection state, event handlers,
   * new CSS classes).
   */
  import type { Snippet } from 'svelte';
  import type { Line } from '$lib/types';
  import type { Side } from '$lib/anchor';
  import { getAnnotContext } from '$lib/context';

  interface SharedProps {
    /** Unused by LineRow itself; optional so walk-driven diff rows can omit it. */
    line?: Line;
    additionalClasses?: Record<string, boolean>;
    gutterClass?: string;
    /**
     * Split-view column this row renders in, for side-aware selection and
     * annotation highlight. Null for unified/flat rows — and for context
     * cells, which are the same line in both columns and highlight in both.
     */
    side?: Side | null;
    gutter: Snippet<[]>;
    code: Snippet<[]>;
    trailing?: Snippet<[]>;
    /** Optional wrapper for the code span. When provided, consumer controls the element and can attach actions. */
    codeWrapper?: Snippet<[Snippet]>;
  }

  /**
   * interactive: true  — a real, selectable/annotatable line. Requires displayIndex.
   * interactive: false — presentational furniture (gap bars, etc). No displayIndex,
   * no add-btn, no gutter click wiring, no selection/annotation lookups.
   *
   * Kept as a discriminated union (not an optional displayIndex + boolean flag) so
   * a non-interactive row can never accidentally carry a displayIndex at the type
   * level — that mistake is exactly what would make a gap row selectable.
   */
  type Props =
    | (SharedProps & { interactive: true; displayIndex: number })
    | (SharedProps & { interactive: false; displayIndex?: never });

  // Held whole, not destructured: destructuring `$props()` here would type
  // `displayIndex` as `number | undefined` and drop the link to `interactive`,
  // defeating the union. Every other component in this codebase destructures
  // `$props()` directly — this is a deliberate, contained exception.
  let props: Props = $props();

  const ctx = getAnnotContext();

  // Unified state derivation from context
  const selected = $derived(props.interactive ? ctx.interaction.isCellHighlighted(props.displayIndex, props.side ?? null) : false);
  const annotated = $derived(props.interactive ? ctx.annotations.hasAnnotation(props.displayIndex, props.side ?? null) : false);
  const markdownMetadata = $derived(ctx.markdownMetadata);

  // Convert additionalClasses object to class string
  const extraClasses = $derived(
    Object.entries(props.additionalClasses ?? {})
      .filter(([_, v]) => v)
      .map(([k]) => k)
      .join(' ')
  );
</script>

<div
  class="line {extraClasses}"
  class:selected
  class:annotated
  data-display-idx={props.interactive ? props.displayIndex : undefined}
  onmouseenter={() => props.interactive && ctx.interaction.handleLineEnter(props.displayIndex)}
  onmouseleave={() => props.interactive && ctx.interaction.handleLineLeave()}
  role="presentation"
>
  {#if props.interactive}
    <button
      class="add-btn"
      onpointerdown={(e) => ctx.interaction.handlePointerDown(props.displayIndex, e)}
      aria-label="Add annotation"
    >+</button>
  {/if}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <span
    class="gutter {props.gutterClass ?? ''}"
    class:selected
    onpointerdown={props.interactive ? (e) => ctx.interaction.handlePointerDown(props.displayIndex, e) : undefined}
    onclick={props.interactive ? () => ctx.interaction.handleGutterClick(props.displayIndex) : undefined}
    role={props.interactive ? 'button' : 'presentation'}
    tabindex={props.interactive ? -1 : undefined}
  >
    {@render props.gutter()}
  </span>
  {#if props.codeWrapper}
    {@render props.codeWrapper(props.code)}
  {:else}
    <span class="code" class:md={markdownMetadata}>
      {@render props.code()}
    </span>
  {/if}
  {#if props.trailing}
    <span class="line-actions">
      {@render props.trailing()}
    </span>
  {/if}
</div>
