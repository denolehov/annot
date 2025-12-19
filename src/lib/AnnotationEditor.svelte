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
  /* Popover-style editor matching hl-editor appearance */
  .annotation-editor {
    display: block;
    position: relative;
    background: var(--bg-portal, #fdfcfa);
    border: 1px solid var(--border-subtle, #e4e4e7);
    border-radius: 8px;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.04),
      0 2px 4px rgba(0, 0, 0, 0.02);
    z-index: 2;
    max-width: 600px;
    margin: 8px 8px 8px 60px;
    padding: 10px 14px;
    min-height: 44px;
    font-family: var(--font-ui, "Inter", sans-serif);
  }

  /* Active state - dashed border while user can type */
  .annotation-editor:not(.sealed) {
    border-style: dashed;
  }

  .annotation-editor:not(.sealed)::after {
    border-left-style: dashed;
    border-top-style: dashed;
  }

  /* Arrow pointing up toward selected lines */
  .annotation-editor::after {
    content: '';
    position: absolute;
    left: 24px;
    top: -6px;
    width: 10px;
    height: 10px;
    background: var(--bg-portal, #fdfcfa);
    border-left: 1px solid var(--border-subtle, #e4e4e7);
    border-top: 1px solid var(--border-subtle, #e4e4e7);
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
    font-family: var(--font-ui, "Inter", sans-serif);
    font-size: 14px;
    line-height: 22px;
    color: var(--text-secondary, #52525b);
  }

  .editor-content :global(.tiptap p) {
    margin: 0;
  }

  .editor-content :global(.tiptap p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    color: var(--text-muted, #71717a);
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
    padding-top: 10px;
    border-top: 1px solid var(--border-subtle, #e4e4e7);
  }

  /* Tag chip styling (zinc family - matching hl) */
  .editor-content :global(.tag-chip) {
    display: inline-flex;
    align-items: center;
    height: var(--chip-height, 20px);
    border-radius: var(--chip-radius, 4px);
    font-size: var(--chip-font, 10px);
    padding: var(--chip-padding, 0 6px);
    user-select: none;
    border: 1px solid transparent;
    font-family: var(--font-ui, "Inter", sans-serif);
    vertical-align: middle;
    margin: 0 2px;
    position: relative;
  }

  /* Color variant for tags */
  .editor-content :global(.tag-chip.tag-tag) {
    background: var(--tag-bg, #fafafa);
    border-color: var(--tag-border, #d4d4d8);
    color: var(--tag-text, #52525b);
  }

  .editor-content :global(.tag-chip .tag-icon) {
    font-weight: 700;
    font-size: var(--chip-font, 10px);
    opacity: 0.9;
  }

  .editor-content :global(.tag-chip .tag-content) {
    font-weight: 500;
    white-space: nowrap;
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-left: 4px;
  }

  /* Selected chip (via backspace or click) */
  .editor-content :global(.tag-chip.ProseMirror-selectednode) {
    outline: 2px solid var(--selection-border, #3b82f6);
    outline-offset: 1px;
    background: var(--selection-bg, #eff6ff);
  }

  /* Hover tooltip for tag instruction */
  .editor-content :global(.tag-chip .chip-tooltip) {
    position: fixed;
    background: var(--text-primary, #18181b);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 11px;
    color: white;
    max-width: 300px;
    word-wrap: break-word;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.1s ease, visibility 0.1s ease;
    z-index: 9999;
    pointer-events: none;
    left: var(--tooltip-x, 0);
    top: var(--tooltip-y, 0);
    transform: translateX(-25%) translateY(-100%) translateY(-8px);
  }

  .editor-content :global(.tag-chip .chip-tooltip::after) {
    content: "";
    position: absolute;
    top: 100%;
    left: 25%;
    transform: translateX(-50%);
    border: 4px solid transparent;
    border-top-color: var(--text-primary, #18181b);
  }

  .editor-content :global(.tag-chip:hover .chip-tooltip) {
    opacity: 1;
    visibility: visible;
  }

  /* Tag suggestions popup */
  .tag-suggestions {
    position: fixed;
    background: var(--bg-window, #fefefe);
    border: 1px solid var(--border-strong, #d4d4d8);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 100;
    min-width: 160px;
    max-width: 300px;
    overflow: hidden;
    padding: 4px;
    font-family: var(--font-ui, "Inter", sans-serif);
  }

  .tag-suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    border-radius: 4px;
    background: none;
    cursor: pointer;
    text-align: left;
  }

  .tag-suggestion:hover,
  .tag-suggestion.selected {
    background: var(--bg-panel, #fafaf8);
  }

  .tag-name {
    font-family: var(--font-ui, "Inter", sans-serif);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary, #18181b);
  }

  .tag-instruction {
    font-size: 11px;
    color: var(--text-muted, #71717a);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    margin-left: auto;
  }
</style>
