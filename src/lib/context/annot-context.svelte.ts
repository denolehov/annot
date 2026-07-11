import { getContext, setContext } from 'svelte';
import type { Line, ContentMetadata, Tag, MarkdownMetadata } from '$lib/types';
import type { Range } from '$lib/range';
import type { SlotRef } from '$lib/anchor';
import type { useInteraction } from '$lib/composables/useInteraction.svelte';
import type { useAnnotations } from '$lib/composables/useAnnotations.svelte';
import type { useExitModes } from '$lib/composables/useExitModes.svelte';
import type { useSearch } from '$lib/composables/useSearch.svelte';
import type { useMermaid } from '$lib/composables/useMermaid.svelte';
import type { FileCollapse } from '$lib/composables/useFileCollapse.svelte';
import type { FileEntry } from '$lib/file-tree';
import type { DiffDisplay } from '$lib/display-rows';

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
  fileCollapse: FileCollapse;

  // Derived values (computed once in provider)
  readonly selection: Range | null;
  readonly isDragging: boolean;
  readonly hoveredIdx: number | null;

  // Static/reactive data
  readonly lines: Line[];
  readonly metadata: ContentMetadata;
  readonly tags: Tag[];
  readonly allowsImagePaste: boolean;
  readonly markdownMetadata: MarkdownMetadata | null;
  readonly contentZoom: number;
  /** The DisplayRow walk — display truth for diff mode; null otherwise. */
  readonly diffDisplay: DiffDisplay | null;
  /** Changed files in a diff; [] for non-diff content.
   *  Transitional FileEntry view of diffDisplay.docs — prefer the walk. */
  readonly fileEntries: FileEntry[];

  // Utilities
  showToast: (message: string, duration?: number) => void;
  isLineSelectable: (displayIdx: number) => boolean;
  getOriginalLinesForRange: (range: Range) => string;

  /**
   * The annotation slot a row hosts, used to connect annotation slots to
   * their content: the annotation whose span ends on this row, or the draft
   * slot for a committed selection ending here, or null.
   */
  slotForRow: (displayIndex: number) => SlotRef | null;
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
