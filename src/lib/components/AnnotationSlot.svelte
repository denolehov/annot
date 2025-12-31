<script lang="ts" module>
  import type { Range } from '$lib/range';
  import type { JSONContent, Tag, Bookmark } from '$lib/types';

  interface AnnotationEntry {
    content: JSONContent;
    sealed: boolean;
  }

  /** Props for AnnotationSlot component (exported for use in other components) */
  export interface AnnotationSlotProps {
    rangeKey: string | null;
    annotationState: {
      getByKey(key: string): AnnotationEntry | undefined;
      isSealed(key: string): boolean;
      unseal(key: string): void;
    };
    interaction: {
      setSelection(range: Range): void;
    };
    tags: Tag[];
    bookmarks: Bookmark[];
    allowsImagePaste: boolean;
    pendingTagInsertion: {
      editorKey: string;
      from: number;
      to: number;
      tag: Tag;
    } | null;
    onUpdate: (content: JSONContent | null) => Promise<void>;
    onDismiss: () => void;
    onRequestCreateTag: (rangeKey: string, text: string, from: number, to: number) => void;
    onImagePasteBlocked: () => void;
    getOriginalLinesForRange: (range: Range) => string;
  }
</script>

<script lang="ts">
  /**
   * AnnotationSlot - Wrapper component for AnnotationEditor in embedded contexts.
   *
   * Handles the conditional rendering, keying, and prop threading for annotations
   * in Portal, CodeBlock, Table, and regular line contexts.
   */
  import AnnotationEditor from '$lib/AnnotationEditor.svelte';
  import { keyToRange } from '$lib/range';

  let {
    rangeKey,
    annotationState,
    interaction,
    tags,
    bookmarks,
    allowsImagePaste,
    pendingTagInsertion,
    onUpdate,
    onDismiss,
    onRequestCreateTag,
    onImagePasteBlocked,
    getOriginalLinesForRange,
  }: AnnotationSlotProps = $props();
</script>

{#if rangeKey}
  {#key rangeKey}
    <AnnotationEditor
      {rangeKey}
      content={annotationState.getByKey(rangeKey)?.content}
      sealed={annotationState.isSealed(rangeKey)}
      {onUpdate}
      onUnseal={() => {
        interaction.setSelection(keyToRange(rangeKey));
        annotationState.unseal(rangeKey);
      }}
      {onDismiss}
      {tags}
      {bookmarks}
      {allowsImagePaste}
      {onImagePasteBlocked}
      onRequestCreateTag={(text, from, to) => onRequestCreateTag(rangeKey, text, from, to)}
      pendingTagInsertion={pendingTagInsertion?.editorKey === rangeKey
        ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag }
        : null}
      getOriginalLines={() => getOriginalLinesForRange(keyToRange(rangeKey))}
    />
  {/key}
{/if}
