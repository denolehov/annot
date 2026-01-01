<script lang="ts" module>
  import type { JSONContent, Tag } from '$lib/types';

  /** Props for AnnotationSlot component (exported for use in other components) */
  export interface AnnotationSlotProps {
    rangeKey: string | null;
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
  }
</script>

<script lang="ts">
  /**
   * AnnotationSlot - Wrapper component for AnnotationEditor in embedded contexts.
   *
   * Handles the conditional rendering, keying, and prop threading for annotations
   * in Portal, CodeBlock, Table, and regular line contexts.
   *
   * Uses context for: annotations, interaction, tags, bookmarks, allowsImagePaste, getOriginalLinesForRange
   */
  import AnnotationEditor from '$lib/AnnotationEditor.svelte';
  import { keyToRange } from '$lib/range';
  import { getAnnotContext } from '$lib/context';

  let {
    rangeKey,
    pendingTagInsertion,
    onUpdate,
    onDismiss,
    onRequestCreateTag,
    onImagePasteBlocked,
  }: AnnotationSlotProps = $props();

  const ctx = getAnnotContext();
</script>

{#if rangeKey}
  {#key rangeKey}
    <AnnotationEditor
      {rangeKey}
      content={ctx.annotations.getByKey(rangeKey)?.content}
      sealed={ctx.annotations.isSealed(rangeKey)}
      {onUpdate}
      onUnseal={() => {
        ctx.interaction.setSelection(keyToRange(rangeKey));
        ctx.annotations.unseal(rangeKey);
      }}
      {onDismiss}
      tags={ctx.tags}
      bookmarks={ctx.bookmarks.all}
      allowsImagePaste={ctx.allowsImagePaste}
      {onImagePasteBlocked}
      onRequestCreateTag={(text, from, to) => onRequestCreateTag(rangeKey, text, from, to)}
      pendingTagInsertion={pendingTagInsertion?.editorKey === rangeKey
        ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag }
        : null}
      getOriginalLines={() => ctx.getOriginalLinesForRange(keyToRange(rangeKey))}
    />
  {/key}
{/if}
