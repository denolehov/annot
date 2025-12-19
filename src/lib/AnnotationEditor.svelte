<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Editor, type JSONContent, type Range } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Placeholder from '@tiptap/extension-placeholder';
  import type { SuggestionProps, SuggestionKeyDownProps } from '@tiptap/suggestion';
  import { trimContent, isContentEmpty, TagChip } from './tiptap';
  import type { Tag } from './types';

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

  // Tag suggestion state
  let tags: Tag[] = $state([]);
  let suggestionCommand: ((item: Tag) => void) | null = $state(null);
  let suggestionState = $state<{
    active: boolean;
    items: Tag[];
    selectedIndex: number;
    clientRect: (() => DOMRect | null) | null;
  }>({
    active: false,
    items: [],
    selectedIndex: 0,
    clientRect: null,
  });

  // Load tags on mount
  onMount(async () => {
    tags = await invoke<Tag[]>('get_tags');
  });

  onMount(() => {
    editorState.editor = new Editor({
      element: element,
      extensions: [
        StarterKit,
        Placeholder.configure({
          placeholder: 'Type annotation…',
        }),
        TagChip.configure({
          suggestion: {
            char: '#',
            items: ({ query }: { query: string }) => {
              return tags
                .filter((tag) => tag.name.toLowerCase().includes(query.toLowerCase()))
                .slice(0, 5);
            },
            render: () => {
              return {
                onStart: (props: SuggestionProps<Tag>) => {
                  suggestionCommand = props.command;
                  suggestionState = {
                    active: true,
                    items: props.items,
                    selectedIndex: 0,
                    clientRect: props.clientRect ?? null,
                  };
                },
                onUpdate: (props: SuggestionProps<Tag>) => {
                  suggestionCommand = props.command;
                  suggestionState = {
                    ...suggestionState,
                    items: props.items,
                    clientRect: props.clientRect ?? null,
                  };
                },
                onKeyDown: (props: SuggestionKeyDownProps) => {
                  if (props.event.key === 'ArrowUp') {
                    suggestionState.selectedIndex =
                      (suggestionState.selectedIndex - 1 + suggestionState.items.length) %
                      suggestionState.items.length;
                    return true;
                  }
                  if (props.event.key === 'ArrowDown') {
                    suggestionState.selectedIndex =
                      (suggestionState.selectedIndex + 1) % suggestionState.items.length;
                    return true;
                  }
                  if (props.event.key === 'Enter') {
                    const item = suggestionState.items[suggestionState.selectedIndex];
                    if (item && suggestionCommand) {
                      suggestionCommand(item);
                    }
                    return true;
                  }
                  if (props.event.key === 'Escape') {
                    suggestionState.active = false;
                    return true;
                  }
                  return false;
                },
                onExit: () => {
                  suggestionState.active = false;
                  suggestionCommand = null;
                },
              };
            },
            command: ({ editor, range, props }: { editor: Editor; range: Range; props: Tag }) => {
              editor
                .chain()
                .focus()
                .insertContentAt(range, [
                  {
                    type: 'tagChip',
                    attrs: {
                      id: props.id,
                      name: props.name,
                      instruction: props.instruction,
                    },
                  },
                  { type: 'text', text: ' ' },
                ])
                .run();
            },
          },
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
        if (!sealed && !suggestionState.active) {
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
  {#if suggestionState.active && suggestionState.items.length > 0}
    <div
      class="tag-suggestions"
      style:left={suggestionState.clientRect?.()?.left ?? 0}
      style:top={(suggestionState.clientRect?.()?.bottom ?? 0) + 4}
    >
      {#each suggestionState.items as tag, i}
        <button
          type="button"
          class="tag-suggestion"
          class:selected={i === suggestionState.selectedIndex}
          onmousedown={(e) => {
            e.preventDefault();
            if (suggestionCommand) {
              suggestionCommand(tag);
            }
          }}
        >
          <span class="tag-name">{tag.name}</span>
          <span class="tag-instruction">{tag.instruction}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if !sealed}
    <div class="toolbar">
      <span class="kbd-hint"><kbd>#</kbd> tags</span>
      <span class="kbd-hint"><kbd>⌘↵</kbd> done</span>
      <span class="kbd-hint"><kbd>Esc</kbd> cancel</span>
    </div>
  {/if}
</div>

<style>
  /* Component styles - see src/styles/editor.css and src/styles/chips.css for shared styles */
</style>
