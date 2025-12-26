<script lang="ts">
  import type { JSONContent, Tag } from '$lib/types';
  import AnnotationEditor from '$lib/AnnotationEditor.svelte';

  interface Props {
    content: JSONContent | undefined;
    isOpen: boolean;
    tags: Tag[];
    allowsImagePaste: boolean;
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
    tags,
    allowsImagePaste,
    pendingTagInsertion,
    onUpdate,
    onOpen,
    onClose,
    onRequestCreateTag,
    onImagePasteBlocked
  }: Props = $props();
</script>

{#if isOpen || content}
  <div class="session-slot">
    <AnnotationEditor
      {content}
      sealed={!isOpen}
      onUpdate={onUpdate}
      onUnseal={onOpen}
      onDismiss={onClose}
      {tags}
      {allowsImagePaste}
      {onImagePasteBlocked}
      {onRequestCreateTag}
      {pendingTagInsertion}
    />
  </div>
{/if}
