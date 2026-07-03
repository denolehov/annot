<script lang="ts" module>
  import type { JSONContent, Tag } from '$lib/types';
  import type { SlotRef } from '$lib/anchor';

  /** Props for AnnotationSlot component (exported for use in other components) */
  export interface AnnotationSlotProps {
    slotRef: SlotRef | null;
    pendingTagInsertion: {
      editorKey: string;
      from: number;
      to: number;
      tag: Tag;
    } | null;
    /** Called when annotation content changes. id identifies which annotation. */
    onUpdate: (id: string, content: JSONContent | null) => Promise<void>;
    /** Called when a sealed annotation is clicked to open its editor. */
    onUnseal: (slot: SlotRef) => void;
    onDismiss: () => void;
    onRequestCreateTag: (id: string, text: string, from: number, to: number) => void;
    onImagePasteBlocked: () => void;
    onFileRefCopied?: (path: string) => void;
  }
</script>

<script lang="ts">
  /**
   * AnnotationSlot - Wrapper component for AnnotationEditor in embedded contexts.
   *
   * Handles the conditional rendering, keying, and prop threading for annotations
   * in Portal, CodeBlock, Table, and regular line contexts. Keyed by annotation
   * id, which is stable across the draft→saved transition — the editor must not
   * remount on the first keystroke.
   *
   * Uses context for: annotations, interaction, tags, allowsImagePaste, getOriginalLinesForRange
   */
  import AnnotationEditor from '$lib/AnnotationEditor.svelte';
  import type { Anchor } from '$lib/anchor';
  import { getAnnotContext } from '$lib/context';

  let {
    slotRef,
    pendingTagInsertion,
    onUpdate,
    onUnseal,
    onDismiss,
    onRequestCreateTag,
    onImagePasteBlocked,
    onFileRefCopied,
  }: AnnotationSlotProps = $props();

  const ctx = getAnnotContext();

  function originalLines(anchor: Anchor): string {
    const span = ctx.annotations.spanOfAnchor(anchor);
    return span ? ctx.getOriginalLinesForRange(span) : '';
  }
</script>

{#if slotRef}
  {@const s = slotRef}
  {#key s.id}
    <AnnotationEditor
      annotationId={s.id}
      content={ctx.annotations.getById(s.id)?.content}
      sealed={ctx.interaction.isAnnotationSealed(s.id)}
      onUpdate={(content) => onUpdate(s.id, content)}
      onUnseal={() => onUnseal(s)}
      {onDismiss}
      tags={ctx.tags}
      annotationEntries={ctx.annotations.allEntries()}
      allowsImagePaste={ctx.allowsImagePaste}
      {onImagePasteBlocked}
      {onFileRefCopied}
      onRequestCreateTag={(text, from, to) => onRequestCreateTag(s.id, text, from, to)}
      pendingTagInsertion={pendingTagInsertion?.editorKey === s.id
        ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag }
        : null}
      getOriginalLines={() => originalLines(s.anchor)}
    />
  {/key}
{/if}
