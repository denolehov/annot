import { untrack } from 'svelte';
import { Editor, type JSONContent, type Range } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import {
  trimContent,
  isContentEmpty,
  TagChip,
  MediaChip,
  ImagePasteHandler,
  ExcalidrawChip,
  ExcalidrawPlaceholder,
  ReplaceBlock,
  ReplacePreview,
  ErrorChip,
  SlashCommands,
  createSlashSuggestion,
  EditorShortcuts,
  createSuggestionRender,
  type SlashCommand,
  type SuggestionState,
} from '../tiptap';
import type { Tag } from '../types';
import { fuzzySearch } from '../fuzzy';

export interface AnnotationEditorOptions {
  /** Returns the DOM element to mount the editor in */
  element: () => HTMLElement | undefined;
  /** Returns initial content (only used at editor creation time) */
  getContent: () => JSONContent | undefined;
  /** Returns whether the editor is sealed (reactive) */
  getSealed: () => boolean;
  /** Returns available tags for autocomplete (reactive) */
  getTags: () => Tag[];
  /** Returns whether image paste is allowed */
  getAllowsImagePaste: () => boolean;
  /** Returns the onUpdate callback (reactive) */
  getOnUpdate: () => (content: JSONContent | null) => void;
  /** Returns the onDismiss callback (reactive) */
  getOnDismiss: () => () => void;
  /** Returns the onImagePasteBlocked callback */
  getOnImagePasteBlocked: () => (() => void) | undefined;
  /** Returns the original lines content for /replace command */
  getOriginalLines?: () => string;
}

function createInitialSuggestionState<T>(): SuggestionState<T> {
  return {
    active: false,
    items: [],
    selectedIndex: 0,
    clientRect: null,
  };
}

/**
 * Composable for managing TipTap editor lifecycle, extensions, and suggestion state.
 * Centralizes editor creation/destruction across N+1 AnnotationEditor instances.
 */
export function useAnnotationEditor(options: AnnotationEditorOptions) {
  let editor: Editor | null = $state(null);
  let tagSuggestion = $state<SuggestionState<Tag>>(createInitialSuggestionState());
  let slashSuggestion = $state<SuggestionState<SlashCommand>>(createInitialSuggestionState());
  let tagCommand: ((item: Tag) => void) | null = null;
  let slashCommandFn: ((item: SlashCommand) => void) | null = null;

  // Track if Excalidraw modal is open (prevents blur dismiss)
  let excalidrawModalOpen = false;

  // Capture initial values OUTSIDE effect to avoid reactive dependencies
  // that would re-run the effect and recreate the editor
  const initialSealed = options.getSealed();
  const initialContent = options.getContent();
  const initialAllowsImagePaste = options.getAllowsImagePaste();
  const initialOnImagePasteBlocked = options.getOnImagePasteBlocked();

  // Create/destroy editor when element becomes available
  // IMPORTANT: Only track `element()` here. All other values are captured above
  // to prevent effect re-runs that would destroy/recreate the editor.
  $effect(() => {
    const el = options.element();
    if (!el) return;

    const { getSealed, getTags, getOnUpdate, getOnDismiss } = options;

    editor = new Editor({
      element: el,
      extensions: [
        StarterKit.configure({
          heading: false,
          blockquote: false,
          codeBlock: false,
          horizontalRule: false,
        }),
        Placeholder.configure({
          placeholder: 'Type annotation…',
        }),
        TagChip.configure({
          suggestion: {
            char: '#',
            items: ({ query }: { query: string }) => {
              return fuzzySearch(getTags(), query, [
                { name: 'name', weight: 2 },
                { name: 'instruction', weight: 1 },
              ]);
            },
            render: createSuggestionRender<Tag>(
              () => tagSuggestion,
              (state) => { tagSuggestion = state; },
              () => tagCommand,
              (cmd) => { tagCommand = cmd; }
            ),
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
        ReplaceBlock,
        ReplacePreview,
        ErrorChip,
        ImagePasteHandler.configure({
          allowsImagePaste: initialAllowsImagePaste,
          onPasteBlocked: initialOnImagePasteBlocked,
        }),
        SlashCommands.configure({
          suggestion: {
            ...createSlashSuggestion({
              getOriginalLines: options.getOriginalLines,
            }),
            render: createSuggestionRender<SlashCommand>(
              () => slashSuggestion,
              (state) => { slashSuggestion = state; },
              () => slashCommandFn,
              (cmd) => { slashCommandFn = cmd; }
            ),
          },
        }),
        EditorShortcuts.configure({
          onSubmit: () => {
            editor?.commands.blur();
          },
          onDismiss: () => {
            // Close suggestion menu first, then dismiss editor on second Escape
            if (tagSuggestion.active) {
              tagSuggestion = { ...tagSuggestion, active: false };
              return;
            }
            if (slashSuggestion.active) {
              slashSuggestion = { ...slashSuggestion, active: false };
              return;
            }
            editor?.commands.blur();
          },
        }),
      ],
      content: initialContent,
      editable: !initialSealed,
      autofocus: false, // Don't autofocus - we'll focus manually without scrolling
      onUpdate: ({ editor }) => {
        // Clear any replace validation errors when content changes
        editor.view.dom.classList.remove('has-replace-error');

        const json = trimContent(editor.getJSON());
        getOnUpdate()(isContentEmpty(json) ? null : json);
      },
      onBlur: ({ editor: blurEditor }) => {
        // Don't dismiss while Excalidraw modal is open or suggestion menus are active
        if (!getSealed() && !tagSuggestion.active && !excalidrawModalOpen) {
          // Find ReplaceBlock nodes and validate/transform them
          const replaceBlocks: Array<{ pos: number; node: import('@tiptap/pm/model').Node }> = [];
          blurEditor.state.doc.descendants((node, pos) => {
            if (node.type.name === 'replaceBlock') {
              replaceBlocks.push({ pos, node });
            }
          });

          // Validate each replace block
          for (const { node } of replaceBlocks) {
            const original = node.attrs.original || '';
            const replacement = node.textContent || '';

            if (original === replacement) {
              // Validation error - content unchanged
              const editorDom = blurEditor.view.dom as HTMLElement;
              editorDom.classList.add('has-replace-error', 'shake');
              setTimeout(() => editorDom.classList.remove('shake'), 400);
              blurEditor.commands.focus();
              return;
            }
          }

          // Clear any previous error state
          const editorDom = blurEditor.view.dom as HTMLElement;
          editorDom.classList.remove('has-replace-error');

          // Transform ReplaceBlock nodes to ReplacePreview nodes for sealed display
          if (replaceBlocks.length > 0 && blurEditor.schema.nodes.replacePreview) {
            const tr = blurEditor.state.tr;
            let offset = 0;

            for (const { pos, node } of replaceBlocks) {
              const original = node.attrs.original || '';
              const replacement = node.textContent || '';
              const preview = blurEditor.schema.nodes.replacePreview.create({
                blockId: node.attrs.blockId || crypto.randomUUID(),
                original,
                replacement,
              });

              const mappedPos = pos + offset;
              tr.replaceWith(mappedPos, mappedPos + node.nodeSize, preview);
              offset += preview.nodeSize - node.nodeSize;
            }

            if (tr.docChanged) {
              blurEditor.view.dispatch(tr);
            }
          }

          const trimmed = trimContent(blurEditor.getJSON());
          blurEditor.commands.setContent(trimmed);
          getOnUpdate()(isContentEmpty(trimmed) ? null : trimmed);
          getOnDismiss()();
        }
      },
    });

    return () => {
      editor?.destroy();
      editor = null;
    };
  });

  // Update editable state when sealed changes
  $effect(() => {
    const isSealed = options.getSealed();
    untrack(() => {
      if (editor) {
        editor.setEditable(!isSealed);
        if (!isSealed) {
          // When unsealing, transform ReplacePreviews back to ReplaceBlock for editing
          if (editor.state && editor.schema && editor.schema.nodes.replaceBlock) {
            const doc = editor.state.doc;
            const tr = editor.state.tr;
            let offset = 0;

            doc.descendants((node, pos) => {
              if (node.type.name !== 'replacePreview') return;

              const { blockId, original, replacement } = node.attrs;
              const replaceBlock = editor!.schema.nodes.replaceBlock.create(
                { blockId, original },
                replacement ? editor!.schema.text(replacement) : null
              );

              const mappedPos = pos + offset;
              tr.replaceWith(mappedPos, mappedPos + node.nodeSize, replaceBlock);
              offset += replaceBlock.nodeSize - node.nodeSize;
            });

            if (tr.docChanged) {
              editor.view.dispatch(tr);
            }
          }

          editor.commands.focus('end', { scrollIntoView: false });
        }
      }
    });
  });

  return {
    get editor() { return editor; },
    get tagSuggestion() { return tagSuggestion; },
    get slashSuggestion() { return slashSuggestion; },

    /** Execute selected tag item */
    selectTagItem(item: Tag) {
      tagCommand?.(item);
    },

    /** Execute selected slash command item */
    selectSlashItem(item: SlashCommand) {
      slashCommandFn?.(item);
    },

    /** Insert a tag chip at the specified position (for pending tag insertion) */
    insertPendingTag(tag: Tag, from: number, to: number) {
      if (!editor) return;
      editor
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
    },

    /** Focus the editor at the end */
    focus() {
      editor?.commands.focus('end');
    },

    /** Set Excalidraw modal state (prevents blur dismiss) */
    setExcalidrawModalOpen(open: boolean) {
      excalidrawModalOpen = open;
    },

    /** Check if Excalidraw modal is open */
    get isExcalidrawModalOpen() {
      return excalidrawModalOpen;
    },
  };
}
