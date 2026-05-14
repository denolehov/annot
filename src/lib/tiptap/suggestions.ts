import type { JSONContent } from '@tiptap/core';
import { TextSelection } from '@tiptap/pm/state';
import type { SuggestionOptions, SuggestionProps, SuggestionKeyDownProps } from '@tiptap/suggestion';
import type { Tag } from '../types';
import { fuzzySearch } from '../fuzzy';
import type { SlashCommand } from './extensions/SlashCommands';
import { parseFenceFromJson } from './replace-fence';

/**
 * Generic suggestion state for autocomplete menus.
 * Used by TagChip (#) and SlashCommands (/).
 */
export interface SuggestionState<T> {
  active: boolean;
  items: T[];
  selectedIndex: number;
  clientRect: (() => DOMRect | null) | null;
}

/**
 * Factory to create suggestion render callbacks for TipTap suggestion plugins.
 * Deduplicates the identical render logic between TagChip and SlashCommands.
 *
 * @param isSelectable Optional predicate to determine if an item can be selected.
 *                     Used to skip section headers in navigation.
 */
export function createSuggestionRender<T>(
  getState: () => SuggestionState<T>,
  setState: (state: SuggestionState<T>) => void,
  getCommand: () => ((item: T) => void) | null,
  setCommand: (cmd: ((item: T) => void) | null) => void,
  isSelectable?: (item: T) => boolean
) {
  // Find first selectable index, skipping non-selectable items (like section headers)
  const findFirstSelectable = (items: T[]): number => {
    if (!isSelectable) return 0;
    const idx = items.findIndex(isSelectable);
    return idx >= 0 ? idx : 0;
  };

  // Find next selectable index in given direction, wrapping around
  const findNextSelectable = (items: T[], currentIndex: number, direction: 1 | -1): number => {
    if (!isSelectable) {
      return (currentIndex + direction + items.length) % items.length;
    }
    let idx = currentIndex;
    for (let i = 0; i < items.length; i++) {
      idx = (idx + direction + items.length) % items.length;
      if (isSelectable(items[idx])) return idx;
    }
    return currentIndex; // No selectable item found, stay put
  };

  return () => ({
    onStart: (props: SuggestionProps<T>) => {
      setCommand(props.command);
      setState({
        active: true,
        items: props.items,
        selectedIndex: findFirstSelectable(props.items),
        clientRect: props.clientRect ?? null,
      });
    },
    onUpdate: (props: SuggestionProps<T>) => {
      setCommand(props.command);
      const currentState = getState();
      // Preserve selection if still valid, otherwise find first selectable
      let newIndex = currentState.selectedIndex;
      if (newIndex >= props.items.length || (isSelectable && !isSelectable(props.items[newIndex]))) {
        newIndex = findFirstSelectable(props.items);
      }
      setState({
        ...currentState,
        items: props.items,
        selectedIndex: newIndex,
        clientRect: props.clientRect ?? null,
      });
    },
    onKeyDown: (props: SuggestionKeyDownProps) => {
      const state = getState();
      const command = getCommand();
      if (props.event.key === 'ArrowUp') {
        setState({
          ...state,
          selectedIndex: findNextSelectable(state.items, state.selectedIndex, -1),
        });
        return true;
      }
      if (props.event.key === 'ArrowDown') {
        setState({
          ...state,
          selectedIndex: findNextSelectable(state.items, state.selectedIndex, 1),
        });
        return true;
      }
      if (props.event.key === 'Enter') {
        const item = state.items[state.selectedIndex];
        if (item && command) {
          command(item);
        }
        return true;
      }
      if (props.event.key === 'Escape') {
        setState({ ...state, active: false });
        return true;
      }
      return false;
    },
    onExit: () => {
      setState({ ...getState(), active: false });
      setCommand(null);
    },
  });
}

/**
 * Create the suggestion configuration for tag autocomplete.
 * Call this with your tags array and callbacks.
 */
export function createTagSuggestion(
  tags: Tag[],
  onSelect: (tag: Tag) => void
): Omit<SuggestionOptions<Tag>, 'editor'> {
  return {
    char: '#',
    items: ({ query }) => {
      return fuzzySearch(tags, query, [{ name: 'name', weight: 1 }], 5);
    },
    command: ({ editor, range, props }) => {
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
          { type: 'text', text: ' ' }, // Space after tag
        ])
        .run();
      onSelect(props);
    },
  };
}

/**
 * Options for creating slash command suggestions.
 */
export interface SlashSuggestionOptions {
  /** Callback to get the original lines content for /replace command */
  getOriginalLines?: () => string;
}

/**
 * Create the suggestion configuration for slash commands.
 */
export function createSlashSuggestion(
  options: SlashSuggestionOptions = {}
): Omit<SuggestionOptions<SlashCommand>, 'editor'> {
  const { getOriginalLines } = options;

  const commands: SlashCommand[] = [
    {
      id: 'excalidraw',
      name: 'excalidraw',
      description: 'Draw a diagram',
      icon: 'excalidraw',
      action: (editor, range) => {
        editor
          .chain()
          .focus()
          .insertContentAt(range, [
            {
              type: 'excalidrawPlaceholder',
              attrs: { placeholderId: crypto.randomUUID() },
            },
            { type: 'text', text: ' ' },
          ])
          .run();
      },
    },
    {
      id: 'replace',
      name: 'replace',
      description: 'Propose a replacement',
      icon: 'edit',
      action: (editor, range) => {
        // Check if there's already a replace block (limit to one per annotation)
        // Either a sealed replacePreview node or an isolated fence in editing
        let hasReplaceBlock = false;
        editor.state.doc.descendants((node) => {
          if (node.type.name === 'replacePreview') {
            hasReplaceBlock = true;
            return false;
          }
        });
        // Check for existing isolated fence using the centralized parser
        if (!hasReplaceBlock) {
          const json = editor.getJSON();
          hasReplaceBlock = parseFenceFromJson(json) !== null;
        }
        if (hasReplaceBlock) {
          editor.chain().focus().deleteRange(range).run();
          return;
        }

        const original = getOriginalLines?.() ?? '';
        if (!original) {
          editor.chain().focus().deleteRange(range).run();
          return;
        }

        // Insert fence as separate paragraphs for clean isolation
        // This ensures the fence can be transformed without data loss
        const originalLines = original.split('\n');
        const contentNodes: JSONContent[] = [
          { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
          ...originalLines.map((line) => ({
            type: 'paragraph',
            content: line ? [{ type: 'text', text: line }] : undefined,
          })),
          { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
        ];

        editor
          .chain()
          .focus()
          .deleteRange(range)
          .insertContent(contentNodes)
          .command(({ tr, dispatch }) => {
            // Position cursor at end of last content line (before closing fence)
            const doc = tr.doc;
            const lastChild = doc.lastChild;
            if (lastChild && lastChild.type.name === 'paragraph') {
              // Move cursor to end of the paragraph before the closing fence
              const endOfContent = doc.content.size - lastChild.nodeSize - 1;
              tr.setSelection(TextSelection.create(doc, endOfContent));
            }
            if (dispatch) dispatch(tr);
            return true;
          })
          .run();
      },
    },
    {
      id: 'remove',
      name: 'remove',
      description: 'Propose removal (empty replacement)',
      icon: 'edit',
      action: (editor, range) => {
        // Same duplicate check as /replace
        let hasReplaceBlock = false;
        editor.state.doc.descendants((node) => {
          if (node.type.name === 'replacePreview') {
            hasReplaceBlock = true;
            return false;
          }
        });
        if (!hasReplaceBlock) {
          const json = editor.getJSON();
          hasReplaceBlock = parseFenceFromJson(json) !== null;
        }
        if (hasReplaceBlock) {
          editor.chain().focus().deleteRange(range).run();
          return;
        }

        // Insert empty replace fence with trailing paragraph for cursor
        const contentNodes: JSONContent[] = [
          { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
          { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
          { type: 'paragraph' }, // Empty line for cursor
        ];

        editor.chain().focus().deleteRange(range).insertContent(contentNodes).run();
      },
    },
  ];

  return {
    char: '/',
    items: ({ query }) => {
      return fuzzySearch(commands, query, [{ name: 'name', weight: 1 }]);
    },
    command: ({ editor, range, props }) => {
      props.action(editor, range);
    },
  };
}
