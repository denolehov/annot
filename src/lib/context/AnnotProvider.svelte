<script lang="ts">
  /**
   * AnnotProvider - Context provider for annot components.
   *
   * Accepts composables from the page and exposes them via Svelte context,
   * eliminating prop drilling across Portal, CodeBlock, RegularLines, etc.
   *
   * The page creates composables (for keyboard/modal coordination access),
   * then passes them here to be set as context for child components.
   */
  import type { Snippet } from 'svelte';
  import type { Line, ContentMetadata, Tag, MarkdownMetadata } from '$lib/types';
  import type { Range } from '$lib/range';
  import type { SlotRef } from '$lib/anchor';
  import { setAnnotContext, type AnnotContext } from './annot-context.svelte';
  import type { useInteraction } from '$lib/composables/useInteraction.svelte';
  import type { useAnnotations } from '$lib/composables/useAnnotations.svelte';
  import type { useExitModes } from '$lib/composables/useExitModes.svelte';
  import type { useSearch } from '$lib/composables/useSearch.svelte';
  import type { useMermaid } from '$lib/composables/useMermaid.svelte';
  import type { FileCollapse } from '$lib/composables/useFileCollapse.svelte';
  import type { DiffDisplay } from '$lib/display-rows';

  interface Props {
    // Reactive data
    lines: Line[];
    metadata: ContentMetadata;
    tags: Tag[];
    allowsImagePaste: boolean;
    contentZoom: number;
    diffDisplay: DiffDisplay | null;

    // Composables (created by page)
    interaction: ReturnType<typeof useInteraction>;
    annotations: ReturnType<typeof useAnnotations>;
    /** Draft slot for a new annotation (id minted, no content yet). */
    draft: SlotRef | null;
    exitModes: ReturnType<typeof useExitModes>;
    search: ReturnType<typeof useSearch>;
    mermaid: ReturnType<typeof useMermaid>;
    fileCollapse: FileCollapse;

    // Utilities
    showToast: (message: string, duration?: number) => void;
    isLineSelectable: (displayIdx: number) => boolean;
    getOriginalLinesForRange: (range: Range) => string;
    expandContext: AnnotContext['expandContext'];

    children: Snippet;
  }

  let {
    lines,
    metadata,
    tags,
    allowsImagePaste,
    contentZoom,
    diffDisplay,
    interaction,
    annotations,
    draft,
    exitModes,
    search,
    mermaid,
    fileCollapse,
    showToast,
    isLineSelectable,
    getOriginalLinesForRange,
    expandContext,
    children,
  }: Props = $props();

  // Derived metadata helper
  const markdownMetadata = $derived(
    metadata.type === 'markdown' ? metadata as MarkdownMetadata & { type: 'markdown' } : null
  );

  // Derived values for consumers
  const selection = $derived(interaction.range);
  const isDragging = $derived(interaction.phase === 'selecting');
  const hoveredIdx = $derived(interaction.hoverLine);

  // Draft's resolved span, computed once per render rather than once per row
  // (slotForRow below is called for every rendered line).
  const draftSpan = $derived.by(() => {
    if (!draft || !selection || isDragging) return null;
    return annotations.spanOfAnchor(draft.anchor);
  });

  /**
   * The annotation slot hosted by a row, if any. Used by embedded components
   * to connect annotation slots to their content.
   *
   * An existing annotation whose resolved span ends on this row always claims
   * it; otherwise the draft slot renders here once its anchor resolves to
   * this row and the selection is committed (never mid-drag).
   */
  function slotForRow(displayIndex: number): SlotRef | null {
    // Entries and the draft are identity-stable objects (content is mutated
    // in place), so returning them directly keeps the slot prop referentially
    // stable — rows don't re-render just because this re-evaluates.
    const existing = annotations.atEndRow(displayIndex);
    if (existing) return existing;

    if (draftSpan?.end === displayIndex) return draft;

    return null;
  }

  // Set context with getters for reactive updates
  setAnnotContext({
    get interaction() { return interaction; },
    get annotations() { return annotations; },
    get exitModes() { return exitModes; },
    get search() { return search; },
    get mermaid() { return mermaid; },
    get fileCollapse() { return fileCollapse; },

    get selection() { return selection; },
    get isDragging() { return isDragging; },
    get hoveredIdx() { return hoveredIdx; },

    get lines() { return lines; },
    get metadata() { return metadata; },
    get tags() { return tags; },
    get allowsImagePaste() { return allowsImagePaste; },
    get markdownMetadata() { return markdownMetadata; },
    get contentZoom() { return contentZoom; },
    get diffDisplay() { return diffDisplay; },

    get showToast() { return showToast; },
    get isLineSelectable() { return isLineSelectable; },
    get getOriginalLinesForRange() { return getOriginalLinesForRange; },
    get expandContext() { return expandContext; },
    slotForRow,
  });
</script>

{@render children()}
