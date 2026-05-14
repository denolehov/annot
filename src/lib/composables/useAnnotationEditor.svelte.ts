import { untrack } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { Editor, type JSONContent, type Range } from '@tiptap/core';
// Explicit extension list — no StarterKit grab-bag. Every extension here is a
// deliberate choice; nothing rides along silently (see docs/tiptap-rebuild-spec.md).
// Notably absent: TrailingNode (appended a phantom empty paragraph after any
// non-paragraph block, and raced with input-rule undo).
import { Document } from '@tiptap/extension-document';
import { Paragraph } from '@tiptap/extension-paragraph';
import { Text } from '@tiptap/extension-text';
import { Bold } from '@tiptap/extension-bold';
import { Italic } from '@tiptap/extension-italic';
import { Strike } from '@tiptap/extension-strike';
import { Code } from '@tiptap/extension-code';
import { HardBreak } from '@tiptap/extension-hard-break';
import { OrderedList, ListItem, ListKeymap } from '@tiptap/extension-list';
import { Gapcursor, UndoRedo } from '@tiptap/extensions';
import Placeholder from '@tiptap/extension-placeholder';
import {
  trimContent,
  isContentEmpty,
  AnnotBulletList,
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

type SuggestionKind = 'tag' | 'slash' | 'ref';

/**
 * The one suggestion menu currently in play. `#tag`, `@ref`, and `/slash` are
 * mutually exclusive — only one can be open — so they share a single
 * discriminated-union state rather than three parallel fields + command refs.
 */
type ActiveSuggestion =
  | { kind: 'tag'; state: SuggestionState<Tag>; command: ((item: Tag) => void) | null }
  | { kind: 'slash'; state: SuggestionState<SlashCommand>; command: ((item: SlashCommand) => void) | null }
  | { kind: 'ref'; state: SuggestionState<RefSuggestionItem>; command: ((item: RefSuggestionItem) => void) | null };

/**
 * Stable inactive state the per-kind getters fall back to. Never mutated — all
 * write paths produce fresh objects — so it is safe to share.
 */
const EMPTY_SUGGESTION: SuggestionState<never> = createInitialSuggestionState();

/** Mark the active suggestion's menu closed, keeping its kind and command. */
function closeSuggestion(s: ActiveSuggestion): ActiveSuggestion {
  return { ...s, state: { ...s.state, active: false } } as ActiveSuggestion;
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

  // #tag, @ref, and /slash menus are mutually exclusive — one shared state
  // instead of three parallel $state fields plus three command refs.
  let activeSuggestion = $state<ActiveSuggestion | null>(null);

  /**
   * Bridge createSuggestionRender's four typed get/set callbacks to the shared
   * activeSuggestion union for one `kind`, as a tuple ready to spread in.
   */
  function suggestionBridge<T>(
    kind: SuggestionKind
  ): [
    () => SuggestionState<T>,
    (state: SuggestionState<T>) => void,
    () => ((item: T) => void) | null,
    (command: ((item: T) => void) | null) => void,
  ] {
    return [
      () =>
        (activeSuggestion?.kind === kind
          ? activeSuggestion.state
          : EMPTY_SUGGESTION) as SuggestionState<T>,
      (state) => {
        activeSuggestion = {
          kind,
          state,
          command: activeSuggestion?.kind === kind ? activeSuggestion.command : null,
        } as ActiveSuggestion;
      },
      () =>
        (activeSuggestion?.kind === kind
          ? activeSuggestion.command
          : null) as ((item: T) => void) | null,
      (command) => {
        activeSuggestion = {
          kind,
          state:
            activeSuggestion?.kind === kind
              ? activeSuggestion.state
              : createInitialSuggestionState(),
          command,
        } as ActiveSuggestion;
      },
    ];
  }

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
        // Core schema
        Document,
        Paragraph,
        Text,
        // Marks — authorable by typing markdown (`**x**`, `*x*`, `~~x~~`, `` `x` ``).
        // No Underline (no markdown syntax, emits <u> HTML) or Link (no authoring path).
        Bold,
        Italic,
        Strike,
        Code,
        // Nodes & list behavior
        HardBreak,
        AnnotBulletList, // `- ` only — see tiptap.ts; stock also matches `+`/`*`
        OrderedList,
        ListItem,
        ListKeymap,
        // Editing affordances — no Dropcursor (annot has no drag-and-drop)
        Gapcursor,
        UndoRedo,
        Placeholder.configure({
          placeholder: 'Type annotation…',
        }),
        TagChip.configure({
          suggestion: {
            char: '#',
            items: ({ query }: { query: string }) => {
              return fuzzySearch(getTags(), query, [{ name: 'name', weight: 1 }]);
            },
            render: createSuggestionRender<Tag>(...suggestionBridge<Tag>('tag')),
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

              // Build unified list with searchText for fuzzy matching
              const allItems = [
                ...annotationItems.map((item) => ({
                  item,
                  searchText: item.type === 'annotation' ? `${item.key} ${item.preview}` : '',
                })),
                ...bookmarkItems.map((item) => ({
                  item,
                  searchText: item.type === 'bookmark'
                    ? `${item.bookmark.id} ${item.bookmark.label || item.bookmark.snapshot.source_title || ''}`
                    : '',
                })),
                ...headingItems.map((item) => ({
                  item,
                  searchText: item.type === 'heading' ? item.section.title : '',
                })),
                ...fileItems.map((item) => ({
                  item,
                  searchText: item.type === 'file' ? item.path : '',
                })),
              ];

              // Single fuzzy search across all items
              const filtered = fuzzySearch(allItems, query, [{ name: 'searchText', weight: 1 }]);
              const items = filtered.map((f) => f.item);

              // Re-sort by priority: current doc (headings, annotations) → bookmarks → files
              // This ensures contextually relevant items appear first
              const currentDocItems = items.filter((i) => i.type === 'heading' || i.type === 'annotation');
              const bookmarkResults = items.filter((i) => i.type === 'bookmark');
              let fileResults = items.filter((i) => i.type === 'file');

              // Soft limit files to 5 for short queries (< 4 chars) to reduce noise
              const FILE_SOFT_LIMIT = 5;
              const FILE_SOFT_LIMIT_THRESHOLD = 4;
              if (query.length < FILE_SOFT_LIMIT_THRESHOLD) {
                fileResults = fileResults.slice(0, FILE_SOFT_LIMIT);
              }

              return [...currentDocItems, ...bookmarkResults, ...fileResults];
            },
            render: createSuggestionRender<RefSuggestionItem>(...suggestionBridge<RefSuggestionItem>('ref')),
            command: ({ editor, range, props }: { editor: Editor; range: Range; props: RefSuggestionItem }) => {
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
            render: createSuggestionRender<SlashCommand>(...suggestionBridge<SlashCommand>('slash')),
          },
        }),
        EditorShortcuts.configure({
          onSubmit: () => {
            editor?.commands.blur();
          },
          onDismiss: () => {
            // First Escape closes an open suggestion menu; second blurs the editor.
            if (activeSuggestion?.state.active) {
              activeSuggestion = closeSuggestion(activeSuggestion);
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
          // Close any active suggestion menu on blur
          // (blur means focus left both editor AND popup, since popup buttons preventDefault)
          if (activeSuggestion?.state.active) {
            activeSuggestion = closeSuggestion(activeSuggestion);
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
    // Per-kind views onto the shared activeSuggestion — the consumer renders a
    // distinct popup per kind, so it still asks per kind.
    get tagSuggestion(): SuggestionState<Tag> {
      return activeSuggestion?.kind === 'tag' ? activeSuggestion.state : EMPTY_SUGGESTION;
    },
    get slashSuggestion(): SuggestionState<SlashCommand> {
      return activeSuggestion?.kind === 'slash' ? activeSuggestion.state : EMPTY_SUGGESTION;
    },
    get refSuggestion(): SuggestionState<RefSuggestionItem> {
      return activeSuggestion?.kind === 'ref' ? activeSuggestion.state : EMPTY_SUGGESTION;
    },

    /** Execute selected tag item */
    selectTagItem(item: Tag) {
      if (activeSuggestion?.kind === 'tag') activeSuggestion.command?.(item);
    },

    /** Execute selected slash command item */
    selectSlashItem(item: SlashCommand) {
      if (activeSuggestion?.kind === 'slash') activeSuggestion.command?.(item);
    },

    /** Execute selected ref item */
    selectRefItem(item: RefSuggestionItem) {
      if (activeSuggestion?.kind === 'ref') activeSuggestion.command?.(item);
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
