<script lang="ts">
  /**
   * SessionEditor - File-level/global comment editor.
   * Uses context for: tags, bookmarks, allowsImagePaste
   */
  import type { JSONContent, Tag } from '$lib/types';
  import AnnotationEditor from '$lib/AnnotationEditor.svelte';
  import { getAnnotContext } from '$lib/context';

  interface Props {
    content: JSONContent | undefined;
    isOpen: boolean;
    pendingTagInsertion: { from: number; to: number; tag: Tag } | null;
    onUpdate: (content: JSONContent | null) => void;
    onOpen: () => void;
    onClose: () => void;
    onRequestCreateTag: (text: string, from: number, to: number) => void;
    onImagePasteBlocked: () => void;
  }

  let {
    content,
    isOpen,
    pendingTagInsertion,
    onUpdate,
    onOpen,
    onClose,
    onRequestCreateTag,
    onImagePasteBlocked
  }: Props = $props();

  const ctx = getAnnotContext();
</script>

{#if isOpen || content}
  <div class="session-slot">
    <AnnotationEditor
      {content}
      sealed={!isOpen}
      onUpdate={onUpdate}
      onUnseal={onOpen}
      onDismiss={onClose}
      tags={ctx.tags}
      bookmarks={ctx.bookmarks.all}
      annotationEntries={ctx.annotations.allEntries()}
      allowsImagePaste={ctx.allowsImagePaste}
      {onImagePasteBlocked}
      {onRequestCreateTag}
      {pendingTagInsertion}
    />
  </div>
{/if}
