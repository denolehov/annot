<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { Editor, type JSONContent, type Range } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Placeholder from '@tiptap/extension-placeholder';
  import type { SuggestionProps, SuggestionKeyDownProps } from '@tiptap/suggestion';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    trimContent,
    isContentEmpty,
    TagChip,
    MediaChip,
    ImagePasteHandler,
    ExcalidrawChip,
    ExcalidrawPlaceholder,
    SlashCommands,
    createSlashSuggestion,
    EditorShortcuts,
    type SlashCommand,
  } from './tiptap';
  import type { Tag } from './types';

  // Portal action: moves element to body so it's not clipped by scroll containers
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }

  interface Props {
    content?: JSONContent;
    onUpdate: (content: JSONContent | null) => void;
    sealed?: boolean;
    onUnseal?: () => void;
    onDismiss?: () => void;
    tags?: Tag[];
    allowsImagePaste?: boolean;
    onImagePasteBlocked?: () => void;
    onRequestCreateTag?: (text: string, from: number, to: number) => void;
    pendingTagInsertion?: { from: number; to: number; tag: Tag } | null;
    rangeKey?: string; // Annotation line range key like "45-52"
  }

  let { content, onUpdate, sealed = false, onUnseal, onDismiss, tags = [], allowsImagePaste = false, onImagePasteBlocked, onRequestCreateTag, pendingTagInsertion, rangeKey = '' }: Props = $props();

  // Types for Excalidraw window communication
  interface NodeRef {
    type: 'Chip' | 'Placeholder';
    id: string;
  }

  interface ExcalidrawOutcome {
    type: 'Saved' | 'Cancelled';
    elements?: string;
    png?: string;
  }

  interface ExcalidrawResult {
    range_key: string;
    node_ref: NodeRef;
    outcome: ExcalidrawOutcome;
  }

  let container: HTMLDivElement | undefined = $state();
  let element: HTMLDivElement | undefined = $state();
  let editorState: { editor: Editor | null } = $state({ editor: null });

  // Tag suggestion state
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

  // Slash command suggestion state
  let slashCommand: ((item: SlashCommand) => void) | null = $state(null);
  let slashState = $state<{
    active: boolean;
    items: SlashCommand[];
    selectedIndex: number;
    clientRect: (() => DOMRect | null) | null;
  }>({
    active: false,
    items: [],
    selectedIndex: 0,
    clientRect: null,
  });

  // Excalidraw window state
  let excalidrawWindowOpen = $state(false);
  let unlistenExcalidraw: UnlistenFn | null = null;

  // Selection popover state (for "Create Tag from Selection")
  let selectionPopover = $state<{
    text: string;
    from: number;
    to: number;
    rect: DOMRect;
  } | null>(null);
  let selectionPopoverEl: HTMLDivElement | undefined = $state();
  let selectionDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Force position recalculation on scroll (clientRect is a function, but Svelte needs a state change to re-render)
  let positionTick = $state(0);
  let suggestionsEl: HTMLDivElement | undefined = $state();
  let slashSuggestionsEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!suggestionState.active && !slashState.active) return;

    let rafId: number | null = null;
    const handleScroll = () => {
      if (rafId) return;
      rafId = requestAnimationFrame(() => {
        rafId = null;
        positionTick++; // Trigger re-render to recalculate position
      });
    };

    window.addEventListener('scroll', handleScroll, { passive: true, capture: true });
    return () => {
      window.removeEventListener('scroll', handleScroll, { capture: true });
      if (rafId) cancelAnimationFrame(rafId);
    };
  });

  // Calculate optimal popup position (above or below cursor)
  // _tick parameter creates reactive dependency for scroll updates
  function getSuggestionPosition(_tick: number, clientRect: (() => DOMRect | null) | null, menuEl?: HTMLDivElement): { left: number; top: number } {
    const rect = clientRect?.();
    if (!rect) return { left: 0, top: 0 };

    const menuHeight = menuEl?.offsetHeight ?? 60; // Small default for single-item menus
    const padding = 8;
    const gap = 4;

    const spaceBelow = window.innerHeight - rect.bottom - padding;
    const spaceAbove = rect.top - padding;

    let top: number;
    if (spaceBelow >= menuHeight) {
      // Fits below
      top = rect.bottom + gap;
    } else if (spaceAbove >= menuHeight) {
      // Fits above
      top = rect.top - menuHeight - gap;
    } else {
      // Neither fits fully - pick the larger space
      top = spaceAbove > spaceBelow
        ? rect.top - menuHeight - gap
        : rect.bottom + gap;
    }

    // Clamp to viewport
    if (top < padding) top = padding;

    return { left: rect.left, top };
  }

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
        MediaChip,
        ExcalidrawChip,
        ExcalidrawPlaceholder,
        ImagePasteHandler.configure({
          allowsImagePaste,
          onPasteBlocked: onImagePasteBlocked,
        }),
        SlashCommands.configure({
          suggestion: {
            ...createSlashSuggestion(),
            render: () => {
              return {
                onStart: (props: SuggestionProps<SlashCommand>) => {
                  slashCommand = props.command;
                  slashState = {
                    active: true,
                    items: props.items,
                    selectedIndex: 0,
                    clientRect: props.clientRect ?? null,
                  };
                },
                onUpdate: (props: SuggestionProps<SlashCommand>) => {
                  slashCommand = props.command;
                  slashState = {
                    ...slashState,
                    items: props.items,
                    clientRect: props.clientRect ?? null,
                  };
                },
                onKeyDown: (props: SuggestionKeyDownProps) => {
                  if (props.event.key === 'ArrowUp') {
                    slashState.selectedIndex =
                      (slashState.selectedIndex - 1 + slashState.items.length) %
                      slashState.items.length;
                    return true;
                  }
                  if (props.event.key === 'ArrowDown') {
                    slashState.selectedIndex =
                      (slashState.selectedIndex + 1) % slashState.items.length;
                    return true;
                  }
                  if (props.event.key === 'Enter') {
                    const item = slashState.items[slashState.selectedIndex];
                    if (item && slashCommand) {
                      slashCommand(item);
                    }
                    return true;
                  }
                  if (props.event.key === 'Escape') {
                    slashState.active = false;
                    return true;
                  }
                  return false;
                },
                onExit: () => {
                  slashState.active = false;
                  slashCommand = null;
                },
              };
            },
          },
        }),
        EditorShortcuts.configure({
          onSubmit: () => {
            editorState.editor?.commands.blur();
          },
          onDismiss: () => {
            // Only dismiss if no suggestion menu is active
            if (!suggestionState.active && !slashState.active) {
              editorState.editor?.commands.blur();
            }
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
        // Don't dismiss while Excalidraw window is open - it will handle cleanup on close
        if (!sealed && !suggestionState.active && !excalidrawWindowOpen) {
          // Trim trailing empty paragraphs before sealing
          const trimmed = trimContent(editor.getJSON());
          editor.commands.setContent(trimmed);
          onUpdate(isContentEmpty(trimmed) ? null : trimmed);
          onDismiss?.();
        }
      },
      onSelectionUpdate: ({ editor }) => {
        // Clear any pending debounce
        if (selectionDebounceTimer) {
          clearTimeout(selectionDebounceTimer);
          selectionDebounceTimer = null;
        }

        const { from, to, empty } = editor.state.selection;

        // Hide popover if selection is empty or too short
        if (empty || to - from < 2) {
          selectionPopover = null;
          return;
        }

        // Only show if onRequestCreateTag is provided
        if (!onRequestCreateTag) return;

        // Debounce: show popover after 150ms of stable selection
        selectionDebounceTimer = setTimeout(() => {
          const text = editor.state.doc.textBetween(from, to, ' ');
          // Get the selection coordinates for positioning
          const coords = editor.view.coordsAtPos(from);
          const endCoords = editor.view.coordsAtPos(to);

          selectionPopover = {
            text,
            from,
            to,
            rect: new DOMRect(
              coords.left,
              coords.top,
              endCoords.right - coords.left,
              endCoords.bottom - coords.top
            ),
          };
        }, 150);
      },
    });

    // Handle Excalidraw create/edit events
    const handleExcalidrawCreate = async (e: Event) => {
      const detail = (e as CustomEvent).detail as { pos: number; placeholderId: string };
      excalidrawWindowOpen = true;
      try {
        await invoke('open_excalidraw_window', {
          elements: '[]',
          rangeKey: rangeKey,
          nodeRef: { type: 'Placeholder', id: detail.placeholderId },
        });
      } catch (err) {
        console.error('Failed to open Excalidraw window:', err);
        excalidrawWindowOpen = false;
      }
    };

    const handleExcalidrawEdit = async (e: Event) => {
      const detail = (e as CustomEvent).detail as { pos: number; nodeId: string; elements: string };
      excalidrawWindowOpen = true;
      try {
        await invoke('open_excalidraw_window', {
          elements: detail.elements || '[]',
          rangeKey: rangeKey,
          nodeRef: { type: 'Chip', id: detail.nodeId },
        });
      } catch (err) {
        console.error('Failed to open Excalidraw window:', err);
        excalidrawWindowOpen = false;
      }
    };

    element?.addEventListener('excalidraw-create', handleExcalidrawCreate);
    element?.addEventListener('excalidraw-edit', handleExcalidrawEdit);

    // Listen for Excalidraw results from child window
    listen<ExcalidrawResult>('excalidraw-result', (event) => {
      handleExcalidrawResult(event.payload);
    }).then((unlisten) => {
      unlistenExcalidraw = unlisten;
    });

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

    return () => {
      element?.removeEventListener('excalidraw-create', handleExcalidrawCreate);
      element?.removeEventListener('excalidraw-edit', handleExcalidrawEdit);
      unlistenExcalidraw?.();
    };
  });

  onDestroy(() => {
    editorState.editor?.destroy();
    if (selectionDebounceTimer) {
      clearTimeout(selectionDebounceTimer);
    }
    unlistenExcalidraw?.();
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

  // Handle pending tag insertion (after tag is created via CommandPalette)
  $effect(() => {
    if (!pendingTagInsertion || !editorState.editor) return;

    const { from, to, tag } = pendingTagInsertion;

    editorState.editor
      .chain()
      .focus()
      .deleteRange({ from, to })
      .insertContent([
        {
          type: 'tagChip',
          attrs: {
            id: tag.id,
            name: tag.name,
            instruction: tag.instruction,
          },
        },
        { type: 'text', text: ' ' },
      ])
      .run();
  });

  // Handle Enter key to create tag from selection (capture phase to intercept before TipTap)
  $effect(() => {
    if (!selectionPopover || !onRequestCreateTag) return;

    const currentPopover = selectionPopover;
    const createTag = onRequestCreateTag;

    function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Enter') {
        e.preventDefault();
        e.stopPropagation();
        createTag(currentPopover.text, currentPopover.from, currentPopover.to);
        selectionPopover = null;
      }
    }

    document.addEventListener('keydown', handleKeydown, true);
    return () => document.removeEventListener('keydown', handleKeydown, true);
  });

  // Helper to find a node by attribute
  function findNodeByAttr(attrName: string, attrValue: string): { pos: number; node: import('@tiptap/pm/model').Node } | null {
    if (!editorState.editor) return null;
    let found: { pos: number; node: import('@tiptap/pm/model').Node } | null = null;
    editorState.editor.state.doc.descendants((node, pos) => {
      if (node.attrs[attrName] === attrValue) {
        found = { pos, node };
        return false;
      }
    });
    return found;
  }

  // Handle result from Excalidraw window
  function handleExcalidrawResult(result: ExcalidrawResult) {
    // Only handle results for our annotation
    if (result.range_key !== rangeKey) return;

    excalidrawWindowOpen = false;

    if (result.outcome.type === 'Cancelled') {
      // Handle cancel
      if (result.node_ref.type === 'Placeholder') {
        const found = findNodeByAttr('placeholderId', result.node_ref.id);
        if (found && editorState.editor) {
          const { pos, node } = found;
          let to = pos + node.nodeSize;
          // Also delete trailing space
          const afterPlaceholder = editorState.editor.state.doc.textBetween(to, Math.min(to + 1, editorState.editor.state.doc.content.size), '', '');
          if (afterPlaceholder === ' ') {
            to += 1;
          }
          editorState.editor.chain().deleteRange({ from: pos, to }).run();

          const contentJson = trimContent(editorState.editor.getJSON());
          if (isContentEmpty(contentJson)) {
            onUpdate(null);
            onDismiss?.();
          } else {
            editorState.editor.commands.focus();
          }
        }
      } else {
        // Editing existing chip - just refocus editor
        editorState.editor?.commands.focus();
      }
    } else {
      // Handle save
      const { elements, png } = result.outcome;
      if (result.node_ref.type === 'Placeholder') {
        const found = findNodeByAttr('placeholderId', result.node_ref.id);
        if (found && editorState.editor) {
          editorState.editor
            .chain()
            .focus()
            .deleteRange({ from: found.pos, to: found.pos + found.node.nodeSize })
            .insertContentAt(found.pos, [
              { type: 'excalidrawChip', attrs: { nodeId: crypto.randomUUID(), elements, image: png } },
              { type: 'text', text: ' ' },
            ])
            .run();
        }
      } else {
        const found = findNodeByAttr('nodeId', result.node_ref.id);
        if (found && editorState.editor) {
          editorState.editor
            .chain()
            .focus()
            .deleteRange({ from: found.pos, to: found.pos + found.node.nodeSize })
            .insertContentAt(found.pos, [
              { type: 'excalidrawChip', attrs: { nodeId: crypto.randomUUID(), elements, image: png } },
            ])
            .run();
        }
      }
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div bind:this={container} class="annotation-editor" class:sealed onclick={() => sealed && onUnseal?.()}>
  <div bind:this={element} class="editor-content"></div>
  {#if !sealed}
    <div class="toolbar">
      <span class="kbd-hint"><kbd>#</kbd> tags</span>
      <span class="kbd-hint"><kbd>/</kbd> commands</span>
      <span class="kbd-hint"><kbd>⌘↵</kbd> done</span>
      <span class="kbd-hint"><kbd>Esc</kbd> cancel</span>
    </div>
  {/if}
</div>

<!-- Portal tag suggestions to body so they're not clipped by scroll containers -->
{#if suggestionState.active && suggestionState.items.length > 0}
  {@const pos = getSuggestionPosition(positionTick, suggestionState.clientRect, suggestionsEl)}
  <div
    bind:this={suggestionsEl}
    use:portal
    class="tag-suggestions"
    style:left="{pos.left}px"
    style:top="{pos.top}px"
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

<!-- Portal slash command suggestions to body -->
{#if slashState.active && slashState.items.length > 0}
  {@const pos = getSuggestionPosition(positionTick, slashState.clientRect, slashSuggestionsEl)}
  <div
    bind:this={slashSuggestionsEl}
    use:portal
    class="slash-suggestions"
    style:left="{pos.left}px"
    style:top="{pos.top}px"
  >
    {#each slashState.items as cmd, i}
      <button
        type="button"
        class="slash-suggestion"
        class:selected={i === slashState.selectedIndex}
        onmousedown={(e) => {
          e.preventDefault();
          if (slashCommand) {
            slashCommand(cmd);
          }
        }}
      >
        <span class="slash-icon">{cmd.icon}</span>
        <div class="slash-info">
          <span class="slash-name">{cmd.name}</span>
          <span class="slash-description">{cmd.description}</span>
        </div>
      </button>
    {/each}
  </div>
{/if}

<!-- Selection popover for "Create Tag from Selection" -->
{#if selectionPopover && onRequestCreateTag}
  {@const pos = getSuggestionPosition(positionTick, () => selectionPopover?.rect ?? null, selectionPopoverEl)}
  <div
    bind:this={selectionPopoverEl}
    use:portal
    class="selection-popover"
    style:left="{pos.left}px"
    style:top="{pos.top}px"
  >
    <button
      type="button"
      class="create-tag-btn"
      onmousedown={(e) => {
        e.preventDefault();
        if (selectionPopover && onRequestCreateTag) {
          onRequestCreateTag(selectionPopover.text, selectionPopover.from, selectionPopover.to);
          selectionPopover = null;
        }
      }}
    >
      <span class="create-tag-icon">#</span>
      <span class="create-tag-label">Create Tag</span>
    </button>
  </div>
{/if}

<style>
  /* Component styles - see src/styles/editor.css and src/styles/chips.css for shared styles */
</style>
