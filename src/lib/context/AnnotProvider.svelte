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
  import type { Line, ContentMetadata, Tag, JSONContent, MarkdownMetadata } from '$lib/types';
  import type { Range } from '$lib/range';
  import { rangeToKey } from '$lib/range';
  import { setAnnotContext, type AnnotContext } from './annot-context.svelte';
  import type { useInteraction } from '$lib/composables/useInteraction.svelte';
  import type { useAnnotations } from '$lib/composables/useAnnotations.svelte';
  import type { useExitModes } from '$lib/composables/useExitModes.svelte';
  import type { useSearch } from '$lib/composables/useSearch.svelte';
  import type { useMermaid } from '$lib/composables/useMermaid.svelte';
  import type { useBookmarks } from '$lib/composables/useBookmarks.svelte';
  import type { useTerraformRegions } from '$lib/composables/useTerraformRegions.svelte';

  interface Props {
    // Reactive data
    lines: Line[];
    metadata: ContentMetadata;
    tags: Tag[];
    allowsImagePaste: boolean;
    contentZoom: number;

    // Composables (created by page)
    interaction: ReturnType<typeof useInteraction>;
    annotations: ReturnType<typeof useAnnotations>;
    exitModes: ReturnType<typeof useExitModes>;
    search: ReturnType<typeof useSearch>;
    mermaid: ReturnType<typeof useMermaid>;
    bookmarks: ReturnType<typeof useBookmarks>;
    terraform: ReturnType<typeof useTerraformRegions>;

    // Utilities
    showToast: (message: string, duration?: number) => void;
    isLineSelectable: (displayIdx: number) => boolean;
    getOriginalLinesForRange: (range: Range) => string;

    children: Snippet;
  }

  let {
    lines,
    metadata,
    tags,
    allowsImagePaste,
    contentZoom,
    interaction,
    annotations,
    exitModes,
    search,
    mermaid,
    bookmarks,
    terraform,
    showToast,
    isLineSelectable,
    getOriginalLinesForRange,
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

  const annotationsMap = $derived.by(() => {
    const map = new Map<string, JSONContent>();
    for (const [key, entry] of Object.entries(annotations.annotations)) {
      map.set(key, entry.content);
    }
    return map;
  });

  const lastSelectedLine = $derived.by(() => {
    const sel = interaction.range;
    if (!sel) return null;
    return Math.max(sel.start, sel.end);
  });

  /**
   * Get the range key for a line. Used by embedded components to connect
   * annotation slots to their content.
   *
   * Always returns existing annotation keys. Only returns new selection key
   * when not in pendingChoice mode (waiting for user to choose annotate/bookmark).
   */
  function getRangeKeyForLine(displayIndex: number): string | null {
    // Always show existing annotations
    const annotationAtLine = annotations.getAtLine(displayIndex);
    if (annotationAtLine) {
      return annotationAtLine.key;
    }

    // Don't show new editor during pendingChoice (user choosing annotate vs bookmark)
    if (interaction.pendingChoice) {
      return null;
    }

    // Don't show new editor during terraforming (terraform palette is open)
    if (interaction.phase === 'terraforming') {
      return null;
    }

    const isLast = displayIndex === lastSelectedLine && selection && !isDragging;
    if (isLast && selection) {
      return rangeToKey(selection);
    }

    return null;
  }

  // Set context with getters for reactive updates
  setAnnotContext({
    get interaction() { return interaction; },
    get annotations() { return annotations; },
    get exitModes() { return exitModes; },
    get search() { return search; },
    get mermaid() { return mermaid; },
    get bookmarks() { return bookmarks; },
    get terraform() { return terraform; },

    get selection() { return selection; },
    get isDragging() { return isDragging; },
    get hoveredIdx() { return hoveredIdx; },
    get annotationsMap() { return annotationsMap; },
    get lastSelectedLine() { return lastSelectedLine; },

    get lines() { return lines; },
    get metadata() { return metadata; },
    get tags() { return tags; },
    get allowsImagePaste() { return allowsImagePaste; },
    get markdownMetadata() { return markdownMetadata; },
    get contentZoom() { return contentZoom; },

    get showToast() { return showToast; },
    get isLineSelectable() { return isLineSelectable; },
    get getOriginalLinesForRange() { return getOriginalLinesForRange; },
    getRangeKeyForLine,
  });
</script>

{@render children()}
