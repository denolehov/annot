import { getContext, setContext } from 'svelte';
import type { Line, ContentMetadata, Tag, JSONContent, MarkdownMetadata } from '$lib/types';
import type { Range } from '$lib/range';
import type { useInteraction } from '$lib/composables/useInteraction.svelte';
import type { useAnnotations } from '$lib/composables/useAnnotations.svelte';
import type { useExitModes } from '$lib/composables/useExitModes.svelte';
import type { useSearch } from '$lib/composables/useSearch.svelte';
import type { useMermaid } from '$lib/composables/useMermaid.svelte';
import type { useBookmarks } from '$lib/composables/useBookmarks.svelte';

/**
 * AnnotContext - Shared state and utilities for annot components.
 *
 * Exposed via Svelte context to eliminate prop drilling across
 * Portal, CodeBlock, RegularLines, AnnotationSlot, Header, StatusBar, etc.
 */
export interface AnnotContext {
  // Composable instances (full API access)
  interaction: ReturnType<typeof useInteraction>;
  annotations: ReturnType<typeof useAnnotations>;
  exitModes: ReturnType<typeof useExitModes>;
  search: ReturnType<typeof useSearch>;
  mermaid: ReturnType<typeof useMermaid>;
  bookmarks: ReturnType<typeof useBookmarks>;

  // Derived values (computed once in provider)
  readonly selection: Range | null;
  readonly isDragging: boolean;
  readonly hoveredIdx: number | null;
  readonly annotationsMap: Map<string, JSONContent>;
  readonly lastSelectedLine: number | null;

  // Static/reactive data
  readonly lines: Line[];
  readonly metadata: ContentMetadata;
  readonly tags: Tag[];
  readonly allowsImagePaste: boolean;
  readonly markdownMetadata: MarkdownMetadata | null;

  // Utilities
  showToast: (message: string, duration?: number) => void;
  isLineSelectable: (displayIdx: number) => boolean;
  getOriginalLinesForRange: (range: Range) => string;
}

const ANNOT_CONTEXT = Symbol('annot');

export function setAnnotContext(ctx: AnnotContext): void {
  setContext(ANNOT_CONTEXT, ctx);
}

export function getAnnotContext(): AnnotContext {
  const ctx = getContext<AnnotContext>(ANNOT_CONTEXT);
  if (!ctx) {
    throw new Error('getAnnotContext must be called within AnnotProvider');
  }
  return ctx;
}
