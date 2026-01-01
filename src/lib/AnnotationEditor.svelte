<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { JSONContent } from '@tiptap/core';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { computePosition, offset, flip, shift, type Placement } from '@floating-ui/dom';
  import { useAnnotationEditor } from './composables';
  import { trimContent, isContentEmpty } from './tiptap';
  import type { Tag, Bookmark } from './types';
  import Icon from './CommandPalette/Icon.svelte';

  interface NodeRef {
    type: 'Chip' | 'Placeholder';
    id: string;
  }

  type ExcalidrawOutcome =
    | { type: 'Saved'; elements: string; png: string }
    | { type: 'Cancelled' };

  interface ExcalidrawResult {
    range_key: string;
    node_ref: NodeRef;
    outcome: ExcalidrawOutcome;
  }

  // Portal action: moves element to body so it's not clipped by scroll containers
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }

  // Floating UI action: positions element relative to a virtual reference (cursor rect)
  function floating(node: HTMLElement, opts: { getRect: () => DOMRect | null; placement?: Placement }) {
    let cleanup: (() => void) | null = null;

    async function update() {
      const rect = opts.getRect();
      if (!rect) return;

      // Create virtual element for Floating UI
      const virtualEl = {
        getBoundingClientRect: () => rect,
      };

      const { x, y, placement } = await computePosition(virtualEl, node, {
        placement: opts.placement ?? 'bottom-start',
        middleware: [
          offset(4),
          flip({ padding: 8 }),
          shift({ padding: 8 }),
        ],
      });

      // When flipped above cursor, anchor from bottom so menu shrinks downward
      if (placement.startsWith('top')) {
        Object.assign(node.style, {
          left: `${x}px`,
          top: 'auto',
          bottom: `${window.innerHeight - y - node.offsetHeight}px`,
        });
      } else {
        Object.assign(node.style, {
          left: `${x}px`,
          top: `${y}px`,
          bottom: 'auto',
        });
      }
    }

    // Initial position
    update();

    // Reposition on scroll
    const handleScroll = () => {
      requestAnimationFrame(update);
    };
    window.addEventListener('scroll', handleScroll, { passive: true, capture: true });
    cleanup = () => {
      window.removeEventListener('scroll', handleScroll, { capture: true });
    };

    return {
      update(newOpts: { getRect: () => DOMRect | null; placement?: Placement }) {
        opts = newOpts;
        update();
      },
      destroy() {
        cleanup?.();
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
    bookmarks?: Bookmark[];
    allowsImagePaste?: boolean;
    onImagePasteBlocked?: () => void;
    onRequestCreateTag?: (text: string, from: number, to: number) => void;
    pendingTagInsertion?: { from: number; to: number; tag: Tag } | null;
    rangeKey?: string; // Annotation line range key like "45-52"
    getOriginalLines?: () => string; // Returns original lines content for /replace
  }

  let { content, onUpdate, sealed = false, onUnseal, onDismiss, tags = [], bookmarks = [], allowsImagePaste = false, onImagePasteBlocked, onRequestCreateTag, pendingTagInsertion, rangeKey = '', getOriginalLines }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let element: HTMLDivElement | undefined = $state();

  // Detect if content is ONLY a replace block (for compact styling)
  const isReplaceOnly = $derived(
    content?.content?.length === 1 && content?.content?.[0]?.type === 'replacePreview'
  );

  // Use the annotation editor composable for TipTap lifecycle
  const ann = useAnnotationEditor({
    element: () => element,
    getContent: () => content,
    getSealed: () => sealed,
    getTags: () => tags,
    getBookmarks: () => bookmarks,
    getAllowsImagePaste: () => allowsImagePaste,
    getOnUpdate: () => onUpdate,
    getOnDismiss: () => () => onDismiss?.(),
    getOnImagePasteBlocked: () => onImagePasteBlocked,
    getOriginalLines: () => getOriginalLines?.() ?? '',
  });

  // Excalidraw window state (tracks if window is open to prevent blur dismiss)
  let excalidrawWindowOpen = $state(false);
  let excalidrawResultUnlisten: UnlistenFn | null = null;
  let mermaidOpenUnlisten: UnlistenFn | null = null;

  // Selection popover state (for "Create Tag from Selection")
  let selectionPopover = $state<{
    text: string;
    from: number;
    to: number;
    rect: DOMRect;
  } | null>(null);
  let selectionPopoverEl: HTMLDivElement | undefined = $state();
  let selectionDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  let suggestionsEl: HTMLDivElement | undefined = $state();
  let slashSuggestionsEl: HTMLDivElement | undefined = $state();
  let bookmarkSuggestionsEl: HTMLDivElement | undefined = $state();

  // Sync Excalidraw window state with composable (prevents blur dismiss)
  $effect(() => {
    ann.setExcalidrawModalOpen(excalidrawWindowOpen);
  });

  // Scroll selected tag suggestion into view on keyboard navigation
  $effect(() => {
    if (!ann.tagSuggestion.active) return;
    const _idx = ann.tagSuggestion.selectedIndex; // Track changes
    requestAnimationFrame(() => {
      const selected = suggestionsEl?.querySelector('.tag-suggestion.selected') as HTMLElement | null;
      selected?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    });
  });

  // Scroll selected slash suggestion into view on keyboard navigation
  $effect(() => {
    if (!ann.slashSuggestion.active) return;
    const _idx = ann.slashSuggestion.selectedIndex; // Track changes
    requestAnimationFrame(() => {
      const selected = slashSuggestionsEl?.querySelector('.slash-suggestion.selected') as HTMLElement | null;
      selected?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    });
  });

  // Scroll selected bookmark suggestion into view on keyboard navigation
  $effect(() => {
    if (!ann.bookmarkSuggestion.active) return;
    const _idx = ann.bookmarkSuggestion.selectedIndex; // Track changes
    requestAnimationFrame(() => {
      const selected = bookmarkSuggestionsEl?.querySelector('.bookmark-suggestion.selected') as HTMLElement | null;
      selected?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    });
  });

  // Helper to find a TipTap node by attribute value
  function findNodeByAttr(attrName: string, attrValue: string): { pos: number; node: import('@tiptap/pm/model').Node } | null {
    if (!ann.editor) return null;
    let found: { pos: number; node: import('@tiptap/pm/model').Node } | null = null;
    ann.editor.state.doc.descendants((node, pos) => {
      if (node.attrs[attrName] === attrValue) {
        found = { pos, node };
        return false; // Stop iteration
      }
    });
    return found;
  }

  // Open Excalidraw window for creating new diagram
  async function openExcalidrawCreate(placeholderId: string) {
    excalidrawWindowOpen = true;
    try {
      await invoke('open_excalidraw_window', {
        elements: '[]',
        rangeKey: rangeKey,
        nodeRef: { type: 'Placeholder', id: placeholderId },
      });
    } catch (e) {
      console.error('Failed to open Excalidraw window:', e);
      excalidrawWindowOpen = false;
    }
  }

  // Open Excalidraw window for editing existing diagram
  async function openExcalidrawEdit(nodeId: string, elements: string) {
    excalidrawWindowOpen = true;
    try {
      await invoke('open_excalidraw_window', {
        elements: elements || '[]',
        rangeKey: rangeKey,
        nodeRef: { type: 'Chip', id: nodeId },
      });
    } catch (e) {
      console.error('Failed to open Excalidraw window:', e);
      excalidrawWindowOpen = false;
    }
  }

  // Handle Excalidraw result from window
  function handleExcalidrawResult(result: ExcalidrawResult) {
    // Only handle results for our annotation
    if (result.range_key !== rangeKey) return;

    excalidrawWindowOpen = false;

    if (result.outcome.type === 'Cancelled') {
      // Handle cancel
      if (result.node_ref.type === 'Placeholder') {
        const found = findNodeByAttr('placeholderId', result.node_ref.id);
        if (found && ann.editor) {
          const { pos, node } = found;
          let to = pos + node.nodeSize;
          // Also delete trailing space
          const afterPlaceholder = ann.editor.state.doc.textBetween(to, Math.min(to + 1, ann.editor.state.doc.content.size), '', '');
          if (afterPlaceholder === ' ') {
            to += 1;
          }
          ann.editor.chain().deleteRange({ from: pos, to }).run();

          const contentJson = trimContent(ann.editor.getJSON());
          if (isContentEmpty(contentJson)) {
            onUpdate(null);
            onDismiss?.();
          } else {
            ann.editor.commands.focus();
          }
        }
      } else {
        // Editing existing chip - just refocus editor
        ann.editor?.commands.focus();
      }
    } else {
      // Handle save
      const { elements, png } = result.outcome;
      if (result.node_ref.type === 'Placeholder') {
        const found = findNodeByAttr('placeholderId', result.node_ref.id);
        if (found && ann.editor) {
          ann.editor
            .chain()
            .focus()
            .deleteRange({ from: found.pos, to: found.pos + found.node.nodeSize })
            .insertContentAt(found.pos, [
              { type: 'excalidrawChip', attrs: { elements, image: png } },
              { type: 'text', text: ' ' },
            ])
            .run();
        }
      } else {
        const found = findNodeByAttr('nodeId', result.node_ref.id);
        if (found && ann.editor) {
          ann.editor
            .chain()
            .focus()
            .deleteRange({ from: found.pos, to: found.pos + found.node.nodeSize })
            .insertContentAt(found.pos, [
              { type: 'excalidrawChip', attrs: { elements, image: png } },
            ])
            .run();
        }
      }
    }
  }

  // Handle Excalidraw events and scroll into view
  onMount(() => {
    const handleExcalidrawCreate = (e: Event) => {
      const detail = (e as CustomEvent).detail as { pos: number; placeholderId: string };
      openExcalidrawCreate(detail.placeholderId);
    };

    const handleExcalidrawEdit = (e: Event) => {
      const detail = (e as CustomEvent).detail as { pos: number; nodeId: string; elements: string };
      openExcalidrawEdit(detail.nodeId, detail.elements);
    };

    element?.addEventListener('excalidraw-create', handleExcalidrawCreate);
    element?.addEventListener('excalidraw-edit', handleExcalidrawEdit);

    // Listen for Excalidraw results from the window
    listen<ExcalidrawResult>('excalidraw-result', (event) => {
      handleExcalidrawResult(event.payload);
    }).then((unlisten) => {
      excalidrawResultUnlisten = unlisten;
    });

    // Listen for mermaid button requests to open excalidraw
    // This allows mermaid to tap into TipTap's fresh state instead of stale annotationState
    listen<{ rangeKey: string }>('mermaid-open-excalidraw', (event) => {
      if (event.payload.rangeKey !== rangeKey) return;

      // Find excalidraw chip in TipTap's current state
      if (!ann.editor) return;
      let chipFound = false;
      ann.editor.state.doc.descendants((node, pos) => {
        if (chipFound) return false;
        if (node.type.name === 'excalidrawChip' && node.attrs.nodeId) {
          chipFound = true;
          openExcalidrawEdit(node.attrs.nodeId, node.attrs.elements);
          return false;
        }
      });
    }).then((unlisten) => {
      mermaidOpenUnlisten = unlisten;
    });

    // Scroll entire editor into view after layout completes
    setTimeout(() => {
      if (!container) return;
      const scrollParent = container.closest('.content');
      if (!scrollParent) return;

      const containerRect = container.getBoundingClientRect();
      const parentRect = scrollParent.getBoundingClientRect();

      const overflow = containerRect.bottom - parentRect.bottom;
      if (overflow > 0) {
        scrollParent.scrollBy({ top: overflow + 16, behavior: 'smooth' });
      }
    }, 50);

    return () => {
      element?.removeEventListener('excalidraw-create', handleExcalidrawCreate);
      element?.removeEventListener('excalidraw-edit', handleExcalidrawEdit);
    };
  });

  onDestroy(() => {
    if (selectionDebounceTimer) {
      clearTimeout(selectionDebounceTimer);
    }
    excalidrawResultUnlisten?.();
    mermaidOpenUnlisten?.();
  });

  // Handle pending tag insertion (after tag is created via CommandPalette)
  $effect(() => {
    if (!pendingTagInsertion) return;
    const { from, to, tag } = pendingTagInsertion;
    ann.insertPendingTag(tag, from, to);
  });

  // Selection update handling (for "Create Tag from Selection")
  $effect(() => {
    const editor = ann.editor;
    if (!editor || !onRequestCreateTag) return;

    const handleSelectionUpdate = () => {
      if (selectionDebounceTimer) {
        clearTimeout(selectionDebounceTimer);
        selectionDebounceTimer = null;
      }

      const { from, to, empty } = editor.state.selection;

      if (empty || to - from < 2) {
        selectionPopover = null;
        return;
      }

      selectionDebounceTimer = setTimeout(() => {
        const text = editor.state.doc.textBetween(from, to, ' ');
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
    };

    editor.on('selectionUpdate', handleSelectionUpdate);
    return () => {
      editor.off('selectionUpdate', handleSelectionUpdate);
    };
  });

  // Handle Enter key to create tag from selection
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
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_tabindex -->
<div
  bind:this={container}
  class="annotation-editor"
  class:sealed
  class:replace-only={isReplaceOnly}
  role={sealed ? "button" : undefined}
  tabindex={sealed ? 0 : undefined}
  onmousedown={(e) => {
    if (sealed) {
      e.preventDefault();
      e.stopPropagation();
      onUnseal?.();
    }
  }}
  onkeydown={(e) => {
    if (sealed && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      onUnseal?.();
    }
  }}
>
  <div bind:this={element} class="editor-content"></div>
  {#if !sealed}
    <div class="toolbar">
      <span class="kbd-hint"><kbd>#</kbd> tags</span>
      <span class="kbd-hint"><kbd>@</kbd> bookmarks</span>
      <span class="kbd-hint"><kbd>/</kbd> commands</span>
      <span class="kbd-hint"><kbd>⌘↵</kbd> done</span>
      <span class="kbd-hint"><kbd>Esc</kbd> cancel</span>
    </div>
  {/if}
</div>

<!-- Portal tag suggestions to body, positioned with Floating UI -->
{#if ann.tagSuggestion.active && ann.tagSuggestion.items.length > 0}
  <div
    bind:this={suggestionsEl}
    use:portal
    use:floating={{ getRect: () => ann.tagSuggestion.clientRect?.() ?? null }}
    class="tag-suggestions"
  >
    {#each ann.tagSuggestion.items as tag, i}
      <button
        type="button"
        class="tag-suggestion"
        class:selected={i === ann.tagSuggestion.selectedIndex}
        onmousedown={(e) => {
          e.preventDefault();
          ann.selectTagItem(tag);
        }}
      >
        <span class="tag-name">{tag.name}</span>
        <span class="tag-instruction">{tag.instruction}</span>
      </button>
    {/each}
  </div>
{/if}

<!-- Portal slash command suggestions to body, positioned with Floating UI -->
{#if ann.slashSuggestion.active && ann.slashSuggestion.items.length > 0}
  <div
    bind:this={slashSuggestionsEl}
    use:portal
    use:floating={{ getRect: () => ann.slashSuggestion.clientRect?.() ?? null }}
    class="slash-suggestions"
  >
    {#each ann.slashSuggestion.items as cmd, i}
      <button
        type="button"
        class="slash-suggestion"
        class:selected={i === ann.slashSuggestion.selectedIndex}
        onmousedown={(e) => {
          e.preventDefault();
          ann.selectSlashItem(cmd);
        }}
      >
        <span class="slash-icon"><Icon name={cmd.icon} /></span>
        <div class="slash-info">
          <span class="slash-name">{cmd.name}</span>
          <span class="slash-description">{cmd.description}</span>
        </div>
      </button>
    {/each}
  </div>
{/if}

<!-- Portal bookmark suggestions to body, positioned with Floating UI -->
{#if ann.bookmarkSuggestion.active && ann.bookmarkSuggestion.items.length > 0}
  <div
    bind:this={bookmarkSuggestionsEl}
    use:portal
    use:floating={{ getRect: () => ann.bookmarkSuggestion.clientRect?.() ?? null }}
    class="bookmark-suggestions"
  >
    {#each ann.bookmarkSuggestion.items as bookmark, i}
      {@const displayLabel = bookmark.label ?? bookmark.snapshot.source_title}
      {@const dateStr = new Date(bookmark.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
      <button
        type="button"
        class="bookmark-suggestion"
        class:selected={i === ann.bookmarkSuggestion.selectedIndex}
        onmousedown={(e) => {
          e.preventDefault();
          ann.selectBookmarkItem(bookmark);
        }}
      >
        <span class="bookmark-id">{bookmark.id.slice(0, 3)}</span>
        <div class="bookmark-info">
          <span class="bookmark-label">{displayLabel}</span>
          <span class="bookmark-meta">{bookmark.snapshot.source_title} · {dateStr}</span>
        </div>
      </button>
    {/each}
  </div>
{/if}

<!-- Selection popover for "Create Tag from Selection", positioned with Floating UI -->
{#if selectionPopover && onRequestCreateTag}
  <div
    bind:this={selectionPopoverEl}
    use:portal
    use:floating={{ getRect: () => selectionPopover?.rect ?? null }}
    class="selection-popover"
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
