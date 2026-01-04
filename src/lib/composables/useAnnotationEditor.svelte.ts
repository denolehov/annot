import { untrack } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { Editor, type JSONContent, type Range } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import {
  trimContent,
  isContentEmpty,
  ImagePasteHandler,
  TextPasteHandler,
  SlashCommands,
  createSlashSuggestion,
  EditorShortcuts,
  createSuggestionRender,
  parseFenceFromJson,
  transformReplaceFenceToPreview,
  transformReplacePreviewToFence,
  extractContentNodes,
  type SlashCommand,
  type SuggestionState,
} from '../tiptap';
import {
  ErrorChip,
  TagChip,
  PasteChip,
  MediaChip,
  RefChip,
  type RefSuggestionItem,
  ReplacePreview,
  ExcalidrawChip,
  ExcalidrawPlaceholder,
} from '../tiptap/extensions';
import type { Tag, Bookmark, RefSnapshot, AnnotationRefSnapshot, ContentNode, SectionInfo } from '../types';
import type { AnnotationEntry } from './useAnnotations.svelte';
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
  /** Returns available bookmarks for @ autocomplete (reactive) */
  getBookmarks: () => Bookmark[];
  /** Returns all annotation entries for @ autocomplete (reactive) */
  getAnnotationEntries: () => Record<string, AnnotationEntry>;
  /** Returns the current annotation's range key (to exclude from suggestions) */
  getCurrentRangeKey: () => string;
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
  /** Returns markdown sections for @ autocomplete (reactive, null if not markdown mode) */
  getSections?: () => SectionInfo[] | null;
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
/** Extract a preview string from ContentNode array (first ~50 chars of text) */
function extractPreviewFromContent(nodes: ContentNode[]): string {
  const textParts: string[] = [];
  for (const node of nodes) {
    if (node.type === 'text') {
      textParts.push(node.text);
    } else if (node.type === 'tag') {
      textParts.push(`#${node.name}`);
    }
    // Stop after ~50 chars
    if (textParts.join('').length > 50) break;
  }
  const full = textParts.join('').trim();
  return full.length > 50 ? full.slice(0, 47) + '...' : full;
}

export function useAnnotationEditor(options: AnnotationEditorOptions) {
  let editor: Editor | null = $state(null);
  let tagSuggestion = $state<SuggestionState<Tag>>(createInitialSuggestionState());
  let slashSuggestion = $state<SuggestionState<SlashCommand>>(createInitialSuggestionState());
  let refSuggestion = $state<SuggestionState<RefSuggestionItem>>(createInitialSuggestionState());
  let tagCommand: ((item: Tag) => void) | null = null;
  let slashCommandFn: ((item: SlashCommand) => void) | null = null;
  let refCommand: ((item: RefSuggestionItem) => void) | null = null;

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

    const { getSealed, getTags, getBookmarks, getAnnotationEntries, getCurrentRangeKey, getOnUpdate, getOnDismiss, getSections } = options;

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
              return fuzzySearch(getTags(), query, [{ name: 'name', weight: 1 }]);
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
        // Unified RefChip with @ trigger for annotations, bookmarks, and files
        RefChip.configure({
          suggestion: {
            char: '@',
            items: async ({ query }: { query: string }): Promise<RefSuggestionItem[]> => {
              const currentKey = getCurrentRangeKey();
              const annotations = getAnnotationEntries();
              const bookmarks = getBookmarks();
              const sections = getSections?.() ?? null;

              // Build annotation items (exclude current annotation)
              const annotationItems: RefSuggestionItem[] = Object.entries(annotations)
                .filter(([key, entry]) => key !== currentKey && entry.content)
                .map(([key, entry]) => {
                  const nodes = extractContentNodes(entry.content);
                  const preview = extractPreviewFromContent(nodes);
                  return {
                    type: 'annotation' as const,
                    key,
                    preview,
                    content: nodes,
                  };
                });

              // Build bookmark items
              const bookmarkItems: RefSuggestionItem[] = bookmarks.map((b) => ({
                type: 'bookmark' as const,
                bookmark: b,
              }));

              // Build heading items (only in markdown mode)
              const headingItems: RefSuggestionItem[] = sections
                ? sections.map((s) => ({
                    type: 'heading' as const,
                    section: s,
                  }))
                : [];

              // Fetch file items (only if query >= 2 chars for performance)
              let fileItems: RefSuggestionItem[] = [];
              if (query.length >= 2) {
                try {
                  const files = await invoke<string[]>('list_project_files', { query, limit: 20 });
                  fileItems = files.map((path) => ({
                    type: 'file' as const,
                    path,
                  }));
                } catch {
                  // Ignore errors - file search is best-effort
                }
              }

              // Filter by query
              const q = query.toLowerCase();
              const filteredAnnotations = q ? annotationItems.filter((item) => {
                if (item.type !== 'annotation') return false;
                return item.key.includes(q) || item.preview.toLowerCase().includes(q);
              }) : annotationItems;

              const filteredBookmarks = q ? bookmarkItems.filter((item) => {
                if (item.type !== 'bookmark') return false;
                const label = item.bookmark.label || item.bookmark.snapshot.source_title || '';
                return item.bookmark.id.toLowerCase().includes(q) || label.toLowerCase().includes(q);
              }) : bookmarkItems;

              const filteredHeadings = q ? headingItems.filter((item) => {
                if (item.type !== 'heading') return false;
                return item.section.title.toLowerCase().includes(q);
              }) : headingItems;

              // Build sectioned results
              const result: RefSuggestionItem[] = [];

              if (filteredAnnotations.length > 0) {
                result.push({ type: 'menu-header', label: 'Annotations' });
                result.push(...filteredAnnotations);
              }

              if (filteredBookmarks.length > 0) {
                result.push({ type: 'menu-header', label: 'Bookmarks' });
                result.push(...filteredBookmarks);
              }

              if (filteredHeadings.length > 0) {
                result.push({ type: 'menu-header', label: 'Sections' });
                result.push(...filteredHeadings);
              }

              if (fileItems.length > 0) {
                result.push({ type: 'menu-header', label: 'Files' });
                result.push(...fileItems);
              }

              return result;
            },
            render: createSuggestionRender<RefSuggestionItem>(
              () => refSuggestion,
              (state) => { refSuggestion = state; },
              () => refCommand,
              (cmd) => { refCommand = cmd; },
              (item) => item.type !== 'menu-header' // Skip menu headers in navigation
            ),
            command: ({ editor, range, props }: { editor: Editor; range: Range; props: RefSuggestionItem }) => {
              // Skip menu headers - they're not selectable
              if (props.type === 'menu-header') return;

              if (props.type === 'file') {
                // File reference - no snapshot, just path
                editor
                  .chain()
                  .focus()
                  .insertContentAt(range, [
                    {
                      type: 'refChip',
                      attrs: { refType: 'file', path: props.path },
                    },
                    { type: 'text', text: ' ' },
                  ])
                  .run();
                return;
              }

              if (props.type === 'heading') {
                // Heading section reference
                editor
                  .chain()
                  .focus()
                  .insertContentAt(range, [
                    {
                      type: 'refChip',
                      attrs: {
                        refType: 'heading',
                        sectionLine: props.section.source_line,
                        sectionLevel: props.section.level,
                        sectionTitle: props.section.title,
                      },
                    },
                    { type: 'text', text: ' ' },
                  ])
                  .run();
                return;
              }

              let snapshot: RefSnapshot;
              let refType: 'annotation' | 'bookmark';

              if (props.type === 'annotation') {
                refType = 'annotation';
                snapshot = {
                  type: 'annotation',
                  source_key: props.key,
                  source_file: null, // Same file
                  preview: props.preview,
                  content: props.content,
                } as AnnotationRefSnapshot;
              } else {
                refType = 'bookmark';
                snapshot = {
                  type: 'bookmark',
                  bookmark: props.bookmark,
                };
              }

              editor
                .chain()
                .focus()
                .insertContentAt(range, [
                  {
                    type: 'refChip',
                    attrs: { refType, snapshot },
                  },
                  { type: 'text', text: ' ' },
                ])
                .run();
            },
          },
        }),
        MediaChip,
        PasteChip,
        ExcalidrawChip,
        ExcalidrawPlaceholder,
        ReplacePreview,
        ErrorChip,
        ImagePasteHandler.configure({
          allowsImagePaste: initialAllowsImagePaste,
          onPasteBlocked: initialOnImagePasteBlocked,
        }),
        TextPasteHandler,
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
            if (refSuggestion.active) {
              refSuggestion = { ...refSuggestion, active: false };
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
        // Don't dismiss while Excalidraw modal is open
        if (!getSealed() && !excalidrawModalOpen) {
          // Close any active suggestion menus on blur
          // (blur means focus left both editor AND popup, since popup buttons preventDefault)
          if (tagSuggestion.active) {
            tagSuggestion = { ...tagSuggestion, active: false };
          }
          if (refSuggestion.active) {
            refSuggestion = { ...refSuggestion, active: false };
          }
          if (slashSuggestion.active) {
            slashSuggestion = { ...slashSuggestion, active: false };
          }
          const editorDom = blurEditor.view.dom as HTMLElement;
          const json = blurEditor.getJSON();

          // Use centralized parser to find isolated fence
          const parsed = parseFenceFromJson(json);

          if (parsed) {
            const original = options.getOriginalLines?.() ?? '';

            // Validate: replacement must differ from original
            if (parsed.replacement === original) {
              editorDom.classList.add('has-replace-error', 'shake');
              setTimeout(() => editorDom.classList.remove('shake'), 400);
              blurEditor.commands.focus();
              return;
            }

            // Clear any previous error state
            editorDom.classList.remove('has-replace-error');

            // Transform the fence text to ReplacePreview node
            const transformedJson = transformReplaceFenceToPreview(json, original, parsed.replacement);
            const trimmed = trimContent(transformedJson);
            blurEditor.commands.setContent(trimmed);
            getOnUpdate()(isContentEmpty(trimmed) ? null : trimmed);
            getOnDismiss()();
          } else {
            // No valid isolated fence found, clear error state
            editorDom.classList.remove('has-replace-error');

            const trimmed = trimContent(json);
            blurEditor.commands.setContent(trimmed);
            getOnUpdate()(isContentEmpty(trimmed) ? null : trimmed);
            getOnDismiss()();
          }
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
          // When unsealing, transform ReplacePreviews back to fence text for editing
          const json = editor.getJSON();
          const transformedJson = transformReplacePreviewToFence(json);
          editor.commands.setContent(transformedJson);

          // Focus at end after content is set
          editor.commands.focus('end', { scrollIntoView: false });
        }
      }
    });
  });

  return {
    get editor() { return editor; },
    get tagSuggestion() { return tagSuggestion; },
    get slashSuggestion() { return slashSuggestion; },
    get refSuggestion() { return refSuggestion; },

    /** Execute selected tag item */
    selectTagItem(item: Tag) {
      tagCommand?.(item);
    },

    /** Execute selected slash command item */
    selectSlashItem(item: SlashCommand) {
      slashCommandFn?.(item);
    },

    /** Execute selected ref item */
    selectRefItem(item: RefSuggestionItem) {
      refCommand?.(item);
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
