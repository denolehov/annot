<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { JSONContent } from '@tiptap/core';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { useAnnotationEditor } from './composables';
  import { trimContent, isContentEmpty } from './tiptap';
  import type { Tag } from './types';

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
    getOriginalLines?: () => string; // Returns original lines content for /replace
  }

  let { content, onUpdate, sealed = false, onUnseal, onDismiss, tags = [], allowsImagePaste = false, onImagePasteBlocked, onRequestCreateTag, pendingTagInsertion, rangeKey = '', getOriginalLines }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let element: HTMLDivElement | undefined = $state();

  // Use the annotation editor composable for TipTap lifecycle
  const ann = useAnnotationEditor({
    element: () => element,
    getContent: () => content,
    getSealed: () => sealed,
    getTags: () => tags,
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

  // Force position recalculation on scroll
  let positionTick = $state(0);
  let suggestionsEl: HTMLDivElement | undefined = $state();
  let slashSuggestionsEl: HTMLDivElement | undefined = $state();

  // Sync Excalidraw window state with composable (prevents blur dismiss)
  $effect(() => {
    ann.setExcalidrawModalOpen(excalidrawWindowOpen);
  });

  // Scroll listener for popup repositioning
  $effect(() => {
    if (!ann.tagSuggestion.active && !ann.slashSuggestion.active) return;

    let rafId: number | null = null;
    const handleScroll = () => {
      if (rafId) return;
      rafId = requestAnimationFrame(() => {
        rafId = null;
        positionTick++;
      });
    };

    window.addEventListener('scroll', handleScroll, { passive: true, capture: true });
    return () => {
      window.removeEventListener('scroll', handleScroll, { capture: true });
      if (rafId) cancelAnimationFrame(rafId);
    };
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

  // Calculate optimal popup position (above or below cursor)
  function getSuggestionPosition(_tick: number, clientRect: (() => DOMRect | null) | null, menuEl?: HTMLDivElement): { left: number; top: number } {
    const rect = clientRect?.();
    if (!rect) return { left: 0, top: 0 };

    const menuHeight = menuEl?.offsetHeight ?? 60;
    const padding = 8;
    const gap = 4;

    const spaceBelow = window.innerHeight - rect.bottom - padding;
    const spaceAbove = rect.top - padding;

    let top: number;
    if (spaceBelow >= menuHeight) {
      top = rect.bottom + gap;
    } else if (spaceAbove >= menuHeight) {
      top = rect.top - menuHeight - gap;
    } else {
      top = spaceAbove > spaceBelow
        ? rect.top - menuHeight - gap
        : rect.bottom + gap;
    }

    if (top < padding) top = padding;

    return { left: rect.left, top };
  }

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
{#if ann.tagSuggestion.active && ann.tagSuggestion.items.length > 0}
  {@const pos = getSuggestionPosition(positionTick, ann.tagSuggestion.clientRect, suggestionsEl)}
  <div
    bind:this={suggestionsEl}
    use:portal
    class="tag-suggestions"
    style:left="{pos.left}px"
    style:top="{pos.top}px"
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

<!-- Portal slash command suggestions to body -->
{#if ann.slashSuggestion.active && ann.slashSuggestion.items.length > 0}
  {@const pos = getSuggestionPosition(positionTick, ann.slashSuggestion.clientRect, slashSuggestionsEl)}
  <div
    bind:this={slashSuggestionsEl}
    use:portal
    class="slash-suggestions"
    style:left="{pos.left}px"
    style:top="{pos.top}px"
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
