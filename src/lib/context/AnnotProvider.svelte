<script lang="ts">
  /**
   * AnnotProvider - Context provider for annot components.
   *
   * Instantiates composables and exposes them via Svelte context,
   * eliminating prop drilling across Portal, CodeBlock, RegularLines, etc.
   */
  import type { Snippet } from 'svelte';
  import type { Line, ContentMetadata, Tag, Bookmark, JSONContent, MarkdownMetadata, DiffMetadata } from '$lib/types';
  import type { Range } from '$lib/range';
  import { ContentTracker, type HunkPayload } from '$lib/content-tracker';
  import { setAnnotContext, type AnnotContext } from './annot-context.svelte';
  import { useInteraction } from '$lib/composables/useInteraction.svelte';
  import { useAnnotations } from '$lib/composables/useAnnotations.svelte';
  import { useExitModes } from '$lib/composables/useExitModes.svelte';
  import { useSearch } from '$lib/composables/useSearch.svelte';
  import { useMermaid } from '$lib/composables/useMermaid.svelte';
  import { useSelectionBounds } from '$lib/composables/useSelectionBounds.svelte';
  import { isSelectable } from '$lib/line-utils';

  interface Props {
    lines: Line[];
    metadata: ContentMetadata;
    tags: Tag[];
    bookmarks: Bookmark[];
    allowsImagePaste: boolean;
    label: string;
    showToast: (message: string, duration?: number) => void;
    scrollToDisplayIndex: (displayIndex: number) => void;
    hunkTracker: ContentTracker<HunkPayload> | null;
    children: Snippet;
  }

  let {
    lines,
    metadata,
    tags,
    bookmarks,
    allowsImagePaste,
    label,
    showToast,
    scrollToDisplayIndex,
    hunkTracker,
    children,
  }: Props = $props();

  // Derived metadata helpers
  const diffMetadata = $derived(metadata.type === 'diff' ? metadata as DiffMetadata & { type: 'diff' } : null);
  const markdownMetadata = $derived(metadata.type === 'markdown' ? metadata as MarkdownMetadata & { type: 'markdown' } : null);

  // Selection bounds (for constraining selections to hunks/portals)
  const selectionBounds = useSelectionBounds({
    getLines: () => lines,
    getDiffMetadata: () => diffMetadata,
    getHunkTracker: () => hunkTracker,
  });

  // Check if a line is selectable
  function isLineSelectable(displayIdx: number): boolean {
    const line = lines[displayIdx - 1];
    return line ? isSelectable(line) : false;
  }

  // Composables
  const interaction = useInteraction({
    isLineSelectable,
    constrainToBounds: selectionBounds.constrainToSelectionBounds,
  });

  const annotations = useAnnotations({
    getLines: () => lines,
  });

  const exitModes = useExitModes();

  const search = useSearch(() => lines, scrollToDisplayIndex);

  const mermaid = useMermaid({
    getLines: () => lines,
    getLabel: () => label,
    getMarkdownMetadata: () => markdownMetadata,
  });

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

  // Get original lines content for a range
  function getOriginalLinesForRange(range: Range): string {
    const start = Math.min(range.start, range.end);
    const end = Math.max(range.start, range.end);
    const rangeLines: string[] = [];
    for (let i = start; i <= end; i++) {
      const line = lines[i - 1];
      if (line) rangeLines.push(line.content);
    }
    return rangeLines.join('\n');
  }

  // Set context with getters for reactive updates
  setAnnotContext({
    interaction,
    annotations,
    exitModes,
    search,
    mermaid,

    get selection() { return selection; },
    get isDragging() { return isDragging; },
    get hoveredIdx() { return hoveredIdx; },
    get annotationsMap() { return annotationsMap; },
    get lastSelectedLine() { return lastSelectedLine; },

    get lines() { return lines; },
    get metadata() { return metadata; },
    get tags() { return tags; },
    get bookmarks() { return bookmarks; },
    get allowsImagePaste() { return allowsImagePaste; },
    get markdownMetadata() { return markdownMetadata; },

    showToast,
    isLineSelectable,
    getOriginalLinesForRange,
  });
</script>

{@render children()}
