<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { Editor, type JSONContent } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Placeholder from '@tiptap/extension-placeholder';
  import { trimContent, isContentEmpty } from './tiptap';

  interface Props {
    content?: JSONContent;
    onUpdate: (content: JSONContent | null) => void;
    sealed?: boolean;
    onUnseal?: () => void;
    onDismiss?: () => void;
  }

  let { content, onUpdate, sealed = false, onUnseal, onDismiss }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let element: HTMLDivElement | undefined = $state();
  let editorState: { editor: Editor | null } = $state({ editor: null });

  onMount(() => {
    editorState.editor = new Editor({
      element: element,
      extensions: [
        StarterKit,
        Placeholder.configure({
          placeholder: 'Type annotation…',
        }),
      ],
      content: content, // TipTap accepts JSONContent directly
      editable: !sealed,
      autofocus: sealed ? false : 'end',
      onUpdate: ({ editor }) => {
        const json = trimContent(editor.getJSON());
        onUpdate(isContentEmpty(json) ? null : json);
      },
      onBlur: ({ editor }) => {
        if (!sealed) {
          // Trim trailing empty paragraphs before sealing
          const trimmed = trimContent(editor.getJSON());
          editor.commands.setContent(trimmed);
          onUpdate(isContentEmpty(trimmed) ? null : trimmed);
          onDismiss?.();
        }
      },
    });

    // Handle Escape and Cmd+Enter to dismiss editor
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!editorState.editor?.isFocused) return;

      if (e.key === 'Escape' || (e.key === 'Enter' && (e.metaKey || e.ctrlKey))) {
        e.preventDefault();
        e.stopPropagation();
        editorState.editor.commands.blur();
      }
    };
    element?.addEventListener('keydown', handleKeyDown);

    // Scroll entire editor (including toolbar) into view after layout completes
    // Use setTimeout to run after TipTap's autofocus scroll
    setTimeout(() => {
      if (!container) return;
      const scrollParent = container.closest('.content');
      if (!scrollParent) return;

      const containerRect = container.getBoundingClientRect();
      const parentRect = scrollParent.getBoundingClientRect();

      // Check if bottom of container is below visible area
      const overflow = containerRect.bottom - parentRect.bottom;
      if (overflow > 0) {
        scrollParent.scrollBy({ top: overflow + 16, behavior: 'smooth' });
      }
    }, 50);

    return () => element?.removeEventListener('keydown', handleKeyDown);
  });

  onDestroy(() => {
    editorState.editor?.destroy();
  });

  // Update editable state when sealed changes
  $effect(() => {
    const isSealed = sealed; // track sealed
    untrack(() => {
      if (editorState.editor) {
        editorState.editor.setEditable(!isSealed);
        if (!isSealed) {
          editorState.editor.commands.focus('end');
        }
      }
    });
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div bind:this={container} class="annotation-editor" class:sealed onclick={() => sealed && onUnseal?.()}>
  <div bind:this={element} class="editor-content"></div>
  {#if !sealed}
    <div class="toolbar">
      <span class="kbd-hint"><kbd>/</kbd> tags</span>
      <span class="kbd-hint"><kbd>⌘↵</kbd> done</span>
      <span class="kbd-hint"><kbd>Esc</kbd> cancel</span>
    </div>
  {/if}
</div>

<style>
  /* Popover-style editor matching hl-editor appearance */
  .annotation-editor {
    display: block;
    position: relative;
    background: #fafaf9;
    border: 1px dashed #e4e4e7;
    border-radius: 8px;
    box-shadow:
      0 1px 1px -1px rgba(0, 0, 0, 0.12),
      0 1px 1px -0.8px rgba(0, 0, 0, 0.1),
      0 0 2px -0.5px rgba(0, 0, 0, 0.08),
      0 0 2px -1px rgba(0, 0, 0, 0.12),
      0 10px 8px -5px rgba(0, 0, 0, 0.06);
    z-index: 2;
    max-width: 600px;
    margin: 8px 8px 8px 60px;
    padding: 10px 14px;
    min-height: 44px;
  }

  /* Arrow pointing up toward selected lines */
  .annotation-editor::after {
    content: '';
    position: absolute;
    left: 24px;
    top: -6px;
    width: 10px;
    height: 10px;
    background: #fafaf9;
    border-left: 1px dashed #e4e4e7;
    border-top: 1px dashed #e4e4e7;
    transform: rotate(45deg);
  }

  /* Sealed state - solid border, clickable to edit */
  .annotation-editor.sealed {
    border-style: solid;
    cursor: pointer;
  }

  .annotation-editor.sealed::after {
    border-left-style: solid;
    border-top-style: solid;
  }

  .editor-content :global(.tiptap) {
    outline: none;
    font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 14px;
    line-height: 22px;
    color: #3f3f46;
  }

  .editor-content :global(.tiptap p) {
    margin: 0;
  }

  .editor-content :global(.tiptap p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    color: #a1a1aa;
    pointer-events: none;
    float: left;
    height: 0;
  }

  /* Compact list styling */
  .editor-content :global(.tiptap ul),
  .editor-content :global(.tiptap ol) {
    margin: 0;
    padding-left: 20px;
  }

  .editor-content :global(.tiptap li) {
    margin: 0;
  }

  .editor-content :global(.tiptap li p) {
    margin: 0;
  }

  /* Toolbar with keyboard hints */
  .toolbar {
    display: flex;
    gap: 12px;
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid var(--border-subtle, #e4e4e7);
  }
</style>
