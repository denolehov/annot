import { Node, Extension, mergeAttributes, type JSONContent } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import Suggestion, { type SuggestionOptions, type SuggestionProps, type SuggestionKeyDownProps } from '@tiptap/suggestion';
import { computePosition, offset, flip, shift, arrow } from '@floating-ui/dom';
import type { ContentNode, Tag } from './types';
import { fuzzySearch } from './fuzzy';

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
 */
export function createSuggestionRender<T>(
  getState: () => SuggestionState<T>,
  setState: (state: SuggestionState<T>) => void,
  getCommand: () => ((item: T) => void) | null,
  setCommand: (cmd: ((item: T) => void) | null) => void
) {
  return () => ({
    onStart: (props: SuggestionProps<T>) => {
      setCommand(props.command);
      setState({
        active: true,
        items: props.items,
        selectedIndex: 0,
        clientRect: props.clientRect ?? null,
      });
    },
    onUpdate: (props: SuggestionProps<T>) => {
      setCommand(props.command);
      setState({
        ...getState(),
        items: props.items,
        clientRect: props.clientRect ?? null,
      });
    },
    onKeyDown: (props: SuggestionKeyDownProps) => {
      const state = getState();
      const command = getCommand();
      if (props.event.key === 'ArrowUp') {
        setState({
          ...state,
          selectedIndex: (state.selectedIndex - 1 + state.items.length) % state.items.length,
        });
        return true;
      }
      if (props.event.key === 'ArrowDown') {
        setState({
          ...state,
          selectedIndex: (state.selectedIndex + 1) % state.items.length,
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

// Unique plugin keys for each suggestion type
const TagSuggestionPluginKey = new PluginKey('tagSuggestion');
const SlashSuggestionPluginKey = new PluginKey('slashSuggestion');

/**
 * Escape HTML special characters in a string.
 */
function escapeHtml(str: string): string {
  if (typeof str !== 'string') return '';
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Compute a simple line-based diff using LCS (Longest Common Subsequence).
 * Returns an array of diff operations: 'equal', 'insert', or 'delete'.
 */
interface DiffLine {
  type: 'equal' | 'insert' | 'delete';
  line: string;
}

function computeDiff(original: string[], replacement: string[]): DiffLine[] {
  const m = original.length;
  const n = replacement.length;

  // Build LCS table
  const lcs: number[][] = Array(m + 1).fill(null).map(() => Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (original[i - 1] === replacement[j - 1]) {
        lcs[i][j] = lcs[i - 1][j - 1] + 1;
      } else {
        lcs[i][j] = Math.max(lcs[i - 1][j], lcs[i][j - 1]);
      }
    }
  }

  // Backtrack to find diff
  const result: DiffLine[] = [];
  let i = m, j = n;

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && original[i - 1] === replacement[j - 1]) {
      result.unshift({ type: 'equal', line: original[i - 1] });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || lcs[i][j - 1] >= lcs[i - 1][j])) {
      result.unshift({ type: 'insert', line: replacement[j - 1] });
      j--;
    } else {
      result.unshift({ type: 'delete', line: original[i - 1] });
      i--;
    }
  }

  return result;
}

/**
 * TagChip node - an inline, atomic node representing a tag.
 * Rendered as [# TAG_NAME] in the editor.
 */

export const TagChip = Node.create({
  name: 'tagChip',
  group: 'inline',
  inline: true,
  atom: true, // Non-editable, treated as single unit

  addAttributes() {
    return {
      id: { default: null },
      name: { default: null },
      instruction: { default: null },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-tag-chip]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            id: element.getAttribute('data-id') || null,
            name: element.getAttribute('data-name') || '',
            instruction: element.getAttribute('data-instruction') || '',
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    // Fallback for SSR/serialization - NodeView handles actual rendering
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-tag-chip': '',
        'data-id': node.attrs.id || '',
        'data-name': node.attrs.name,
        'data-instruction': node.attrs.instruction || '',
        class: 'tag-chip tag-tag',
      }),
      `[# ${node.attrs.name}]`,
    ];
  },

  addNodeView() {
    return ({ node }) => {
      const { name, instruction } = node.attrs;

      const chip = document.createElement('span');
      chip.className = 'tag-chip tag-tag';
      chip.setAttribute('data-tag-chip', '');

      const tooltipContent = instruction ? escapeHtml(instruction) : '';

      chip.innerHTML = `
        <span class="tag-icon">#</span>
        <span class="tag-content">${escapeHtml(name)}</span>
        ${tooltipContent ? `<div class="chip-tooltip"><div class="chip-tooltip-content">${tooltipContent}</div><div class="chip-tooltip-arrow"></div></div>` : ''}
      `;

      // Position tooltip on hover using Floating UI
      if (tooltipContent) {
        const tooltip = chip.querySelector('.chip-tooltip') as HTMLElement;
        const arrowEl = chip.querySelector('.chip-tooltip-arrow') as HTMLElement;

        const updatePosition = async () => {
          const { x, y, placement, middlewareData } = await computePosition(chip, tooltip, {
            placement: 'top',
            middleware: [
              offset(8),
              flip(),
              shift({ padding: 8 }),
              arrow({ element: arrowEl }),
            ],
          });

          Object.assign(tooltip.style, {
            left: `${x}px`,
            top: `${y}px`,
          });

          // Position arrow
          if (middlewareData.arrow) {
            const { x: arrowX } = middlewareData.arrow;
            const staticSide = placement.includes('top') ? 'bottom' : 'top';

            Object.assign(arrowEl.style, {
              left: arrowX != null ? `${arrowX}px` : '',
              [staticSide]: '-4px',
            });
          }
        };

        chip.addEventListener('mouseenter', updatePosition);
      }

      return { dom: chip };
    };
  },

  addKeyboardShortcuts() {
    return {
      Backspace: () =>
        this.editor.commands.command(({ tr, state }) => {
          let isTagChip = false;
          const { selection } = state;
          const { empty, anchor } = selection;

          if (!empty) return false;

          state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
            if (node.type.name === this.name) {
              isTagChip = true;
              tr.insertText('', pos, pos + node.nodeSize);
              return false;
            }
          });

          return isTagChip;
        }),
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        pluginKey: TagSuggestionPluginKey,
        ...this.options.suggestion,
      }),
    ];
  },
});

export type TagChipOptions = {
  suggestion: Omit<SuggestionOptions<Tag>, 'editor' | 'pluginKey'>;
};

/**
 * MediaChip node - an inline, atomic node representing a pasted image.
 * Rendered as [🖼️ Image] in the editor.
 */
export const MediaChip = Node.create({
  name: 'mediaChip',
  group: 'inline',
  inline: true,
  atom: true, // Non-editable, treated as single unit

  addAttributes() {
    return {
      image: { default: '' }, // base64 data URL
      mimeType: { default: 'image/png' },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-media-chip]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            image: element.getAttribute('data-image') || '',
            mimeType: element.getAttribute('data-mime-type') || 'image/png',
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-media-chip': '',
        'data-image': node.attrs.image,
        'data-mime-type': node.attrs.mimeType,
        class: 'tag-chip media-chip',
      }),
      '[🖼️ Image]',
    ];
  },

  addNodeView() {
    return () => {
      const chip = document.createElement('span');
      chip.className = 'tag-chip media-chip';
      chip.setAttribute('data-media-chip', '');
      chip.innerHTML = `
        <span class="tag-icon">🖼️</span>
        <span class="tag-content">Image</span>
      `;
      return { dom: chip };
    };
  },

  addKeyboardShortcuts() {
    return {
      Backspace: () =>
        this.editor.commands.command(({ tr, state }) => {
          let isMediaChip = false;
          const { selection } = state;
          const { empty, anchor } = selection;

          if (!empty) return false;

          state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
            if (node.type.name === this.name) {
              isMediaChip = true;
              tr.insertText('', pos, pos + node.nodeSize);
              return false;
            }
          });

          return isMediaChip;
        }),
    };
  },
});

/**
 * ExcalidrawChip node - an inline, atomic node representing an Excalidraw diagram.
 * Rendered as [📐 Diagram] in the editor. Click to edit.
 */
export const ExcalidrawChip = Node.create({
  name: 'excalidrawChip',
  group: 'inline',
  inline: true,
  atom: true,

  addAttributes() {
    return {
      nodeId: {
        default: () => crypto.randomUUID(),
        parseHTML: (element) =>
          element.getAttribute('data-node-id') || crypto.randomUUID(),
      },
      elements: { default: '[]' }, // JSON string of Excalidraw elements
      image: { default: null }, // base64 PNG for preview/export
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-excalidraw-chip]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            nodeId: element.getAttribute('data-node-id') || crypto.randomUUID(),
            elements: element.getAttribute('data-elements') || '[]',
            image: element.getAttribute('data-image') || null,
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-excalidraw-chip': '',
        'data-node-id': node.attrs.nodeId,
        'data-elements': node.attrs.elements,
        'data-image': node.attrs.image || '',
        class: 'tag-chip excalidraw-chip',
      }),
      '[📐 Diagram]',
    ];
  },

  addNodeView() {
    return ({ node, getPos }) => {
      const chip = document.createElement('span');
      chip.className = 'tag-chip excalidraw-chip';
      chip.setAttribute('data-excalidraw-chip', '');
      chip.innerHTML = `
        <span class="tag-icon">📐</span>
        <span class="tag-content">Diagram</span>
      `;

      // Click to edit - dispatch custom event
      chip.addEventListener('click', () => {
        const pos = typeof getPos === 'function' ? getPos() : null;
        if (pos !== null) {
          const event = new CustomEvent('excalidraw-edit', {
            bubbles: true,
            detail: {
              pos,
              elements: node.attrs.elements,
              nodeId: node.attrs.nodeId,
            },
          });
          chip.dispatchEvent(event);
        }
      });

      return { dom: chip };
    };
  },

  addKeyboardShortcuts() {
    return {
      Backspace: () =>
        this.editor.commands.command(({ tr, state }) => {
          let isChip = false;
          const { selection } = state;
          const { empty, anchor } = selection;

          if (!empty) return false;

          state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
            if (node.type.name === this.name) {
              isChip = true;
              tr.insertText('', pos, pos + node.nodeSize);
              return false;
            }
          });

          return isChip;
        }),
    };
  },
});

/**
 * ExcalidrawPlaceholder node - transient node that auto-triggers modal.
 * Inserted by slash command, deleted if user cancels.
 */
export const ExcalidrawPlaceholder = Node.create({
  name: 'excalidrawPlaceholder',
  group: 'inline',
  inline: true,
  atom: true,

  addAttributes() {
    return {
      placeholderId: { default: () => crypto.randomUUID() },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-excalidraw-placeholder]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            placeholderId: element.getAttribute('data-placeholder-id') || crypto.randomUUID(),
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-excalidraw-placeholder': '',
        'data-placeholder-id': node.attrs.placeholderId,
        class: 'tag-chip excalidraw-placeholder',
      }),
      '[📐 Drawing...]',
    ];
  },

  addNodeView() {
    return ({ node, getPos }) => {
      const chip = document.createElement('span');
      chip.className = 'tag-chip excalidraw-placeholder';
      chip.innerHTML = `
        <span class="tag-icon">📐</span>
        <span class="tag-content">Drawing...</span>
      `;

      // Dispatch event to open window immediately with placeholderId
      requestAnimationFrame(() => {
        const pos = typeof getPos === 'function' ? getPos() : null;
        if (pos !== null) {
          const event = new CustomEvent('excalidraw-create', {
            bubbles: true,
            detail: { pos, placeholderId: node.attrs.placeholderId },
          });
          chip.dispatchEvent(event);
        }
      });

      return { dom: chip };
    };
  },
});

/**
 * ReplacePreview node - sealed atomic node showing an inline diff preview.
 * Created when ```replace fence is sealed. Click to unseal (converts back to fence text).
 */
export const ReplacePreview = Node.create({
  name: 'replacePreview',
  group: 'block',
  atom: true,

  addAttributes() {
    return {
      blockId: { default: null },
      original: { default: '' },
      replacement: { default: '' },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'div[data-replace-preview]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            blockId: element.getAttribute('data-block-id') || null,
            original: element.getAttribute('data-original') || '',
            replacement: element.getAttribute('data-replacement') || '',
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(HTMLAttributes, {
        'data-replace-preview': '',
        'data-block-id': node.attrs.blockId,
        'data-original': node.attrs.original,
        'data-replacement': node.attrs.replacement,
        class: 'replace-preview',
      }),
      '[REPLACE]',
    ];
  },

  addNodeView() {
    return ({ node, getPos, editor }) => {
      const { original, replacement } = node.attrs;

      const wrapper = document.createElement('div');
      wrapper.className = 'replace-preview';
      wrapper.setAttribute('data-replace-preview', '');

      // Show header only if this is the first node (no content above)
      const pos = typeof getPos === 'function' ? getPos() : null;
      const isFirst = pos === 0;

      let diffHtml = isFirst ? '<div class="replace-preview-header">Replace</div>' : '';

      // Generate diff display using LCS algorithm
      const originalLines = original.split('\n');
      const replacementLines = replacement.split('\n');
      const diff = computeDiff(originalLines, replacementLines);

      for (const { type, line } of diff) {
        const gutterChar = type === 'delete' ? '-' : type === 'insert' ? '+' : ' ';
        const lineClass = type === 'delete' ? 'removed' : type === 'insert' ? 'added' : 'context';
        diffHtml += `<div class="replace-preview-line ${lineClass}">`;
        diffHtml += `<span class="replace-preview-gutter">${gutterChar}</span>`;
        diffHtml += `<span class="replace-preview-content">${escapeHtml(line)}</span>`;
        diffHtml += `</div>`;
      }
      wrapper.innerHTML = diffHtml;

      // No click handler here - clicks bubble up to the sealed editor container,
      // which calls onUnseal(). The unseal effect in useAnnotationEditor transforms
      // all ReplacePreview nodes back to fence text and focuses the editor.

      return { dom: wrapper };
    };
  },
});

/**
 * ErrorChip node - an inline, atomic node representing a system-generated error.
 * Rendered as [⚠️ source: message] in the editor. Deletable by user.
 */
export const ErrorChip = Node.create({
  name: 'errorChip',
  group: 'inline',
  inline: true,
  atom: true,

  addAttributes() {
    return {
      source: { default: '' }, // Error source (e.g., 'mermaid')
      message: { default: '' }, // Full error message
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-error-chip]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            source: element.getAttribute('data-source') || '',
            message: element.getAttribute('data-message') || '',
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-error-chip': '',
        'data-source': node.attrs.source,
        'data-message': node.attrs.message,
        class: 'tag-chip error-chip',
      }),
      `[⚠️ ${node.attrs.source} error]`,
    ];
  },

  addNodeView() {
    return ({ node }) => {
      const { source, message } = node.attrs;

      const chip = document.createElement('span');
      chip.className = 'tag-chip error-chip';
      chip.setAttribute('data-error-chip', '');

      // Truncate message for display
      const shortMessage = message.length > 50 ? message.slice(0, 50) + '...' : message;
      const tooltipContent = escapeHtml(message);

      chip.innerHTML = `
        <span class="tag-icon">⚠️</span>
        <span class="tag-content">${escapeHtml(source)} syntax error</span>
        <div class="chip-tooltip error-tooltip"><div class="chip-tooltip-content">${tooltipContent}</div><div class="chip-tooltip-arrow"></div></div>
      `;

      // Position tooltip on hover using Floating UI
      const tooltip = chip.querySelector('.chip-tooltip') as HTMLElement;
      const arrowEl = chip.querySelector('.chip-tooltip-arrow') as HTMLElement;

      const updatePosition = async () => {
        const { x, y, placement, middlewareData } = await computePosition(chip, tooltip, {
          placement: 'top',
          middleware: [
            offset(8),
            flip(),
            shift({ padding: 8 }),
            arrow({ element: arrowEl }),
          ],
        });

        Object.assign(tooltip.style, {
          left: `${x}px`,
          top: `${y}px`,
        });

        // Position arrow
        if (middlewareData.arrow) {
          const { x: arrowX } = middlewareData.arrow;
          const staticSide = placement.includes('top') ? 'bottom' : 'top';

          Object.assign(arrowEl.style, {
            left: arrowX != null ? `${arrowX}px` : '',
            [staticSide]: '-4px',
          });
        }
      };

      chip.addEventListener('mouseenter', updatePosition);

      return { dom: chip };
    };
  },

  addKeyboardShortcuts() {
    return {
      Backspace: () =>
        this.editor.commands.command(({ tr, state }) => {
          let isChip = false;
          const { selection } = state;
          const { empty, anchor } = selection;

          if (!empty) return false;

          state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
            if (node.type.name === this.name) {
              isChip = true;
              tr.insertText('', pos, pos + node.nodeSize);
              return false;
            }
          });

          return isChip;
        }),
    };
  },
});

/**
 * EditorShortcuts extension - handles keyboard shortcuts at the TipTap level
 * to prevent default behavior from firing first.
 */
export interface EditorShortcutsOptions {
  onSubmit?: () => void;
  onDismiss?: () => void;
}

export const EditorShortcuts = Extension.create<EditorShortcutsOptions>({
  name: 'editorShortcuts',

  addOptions() {
    return {
      onSubmit: undefined,
      onDismiss: undefined,
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-Enter': () => {
        this.options.onSubmit?.();
        return true; // Prevent default Enter behavior
      },
      Escape: () => {
        this.options.onDismiss?.();
        return true;
      },
    };
  },
});

/**
 * ImagePasteHandler extension - intercepts paste events and inserts MediaChip nodes for images.
 * Only active when image paste is allowed (MCP content mode).
 */
export interface ImagePasteHandlerOptions {
  allowsImagePaste: boolean;
  onPasteBlocked?: () => void;
}

export const ImagePasteHandler = Extension.create<ImagePasteHandlerOptions>({
  name: 'imagePasteHandler',

  addOptions() {
    return {
      allowsImagePaste: false,
      onPasteBlocked: undefined,
    };
  },

  addStorage() {
    return {
      allowsImagePaste: this.options.allowsImagePaste,
    };
  },

  addProseMirrorPlugins() {
    const extension = this;
    const editor = this.editor;

    return [
      new Plugin({
        key: new PluginKey('imagePasteHandler'),
        props: {
          handlePaste(view, event) {
            const items = event.clipboardData?.items;
            if (!items) return false;

            // Find image in clipboard
            let imageFile: File | null = null;
            for (const item of Array.from(items)) {
              if (item.type.startsWith('image/')) {
                imageFile = item.getAsFile();
                break;
              }
            }

            if (!imageFile) return false;

            // Check allowsImagePaste from storage
            const { allowsImagePaste } = extension.storage;
            const { onPasteBlocked } = extension.options;

            // Block paste if not allowed
            if (!allowsImagePaste) {
              onPasteBlocked?.();
              return true; // Consume the event
            }

            // Convert to base64 and insert MediaChip
            const reader = new FileReader();
            reader.onloadend = () => {
              const dataUrl = reader.result as string;
              editor
                .chain()
                .focus()
                .insertContent([
                  {
                    type: 'mediaChip',
                    attrs: {
                      image: dataUrl,
                      mimeType: imageFile!.type,
                    },
                  },
                  { type: 'text', text: ' ' },
                ])
                .run();
            };
            reader.readAsDataURL(imageFile);

            return true; // Consume the event
          },
        },
      }),
    ];
  },
});

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
      return fuzzySearch(
        tags,
        query,
        [
          { name: 'name', weight: 2 },
          { name: 'instruction', weight: 1 },
        ],
        5
      );
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
 * SlashCommand interface for extensible slash commands.
 */
export interface SlashCommand {
  id: string;
  name: string;
  description: string;
  icon: string;
  action: (editor: import('@tiptap/core').Editor, range: import('@tiptap/core').Range) => void;
}

/**
 * SlashCommands extension - provides `/` triggered command menu.
 */
export interface SlashCommandsOptions {
  suggestion: Omit<SuggestionOptions<SlashCommand>, 'editor' | 'pluginKey'>;
}

export const SlashCommands = Extension.create<SlashCommandsOptions>({
  name: 'slashCommands',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        items: () => [],
        command: ({ editor, range, props }) => {
          props.action(editor, range);
        },
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        pluginKey: SlashSuggestionPluginKey,
        ...this.options.suggestion,
      }),
    ];
  },
});

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
      icon: '📐',
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
      icon: '✏️',
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
          .run();
      },
    },
  ];

  return {
    char: '/',
    items: ({ query }) => {
      return fuzzySearch(commands, query, [
        { name: 'name', weight: 2 },
        { name: 'description', weight: 1 },
      ]);
    },
    command: ({ editor, range, props }) => {
      props.action(editor, range);
    },
  };
}

/**
 * Check if a TipTap paragraph node is empty (no content or only whitespace/hardBreaks)
 */
function isEmptyParagraph(node: JSONContent): boolean {
  if (node.type !== 'paragraph') return false;
  if (!node.content || node.content.length === 0) return true;
  // Check if all children are whitespace text or hardBreaks
  return node.content.every(
    (child) =>
      child.type === 'hardBreak' ||
      (child.type === 'text' && (!child.text || child.text.trim() === ''))
  );
}

/**
 * Trim trailing hardBreaks from a paragraph node.
 * Returns a new node; does not mutate the input.
 */
function trimTrailingHardBreaks(node: JSONContent): JSONContent {
  if (node.type !== 'paragraph' || !node.content || node.content.length === 0) {
    return node;
  }

  const trimmed = [...node.content];
  while (trimmed.length > 0 && trimmed[trimmed.length - 1].type === 'hardBreak') {
    trimmed.pop();
  }

  return { ...node, content: trimmed };
}

/**
 * Trim trailing empty paragraphs and hardBreaks from TipTap JSON content.
 * Returns a new object; does not mutate the input.
 */
export function trimContent(json: JSONContent): JSONContent {
  if (!json.content || json.content.length === 0) {
    return json;
  }

  const trimmed = [...json.content];

  // Remove trailing empty paragraphs
  while (trimmed.length > 0 && isEmptyParagraph(trimmed[trimmed.length - 1])) {
    trimmed.pop();
  }

  // Trim trailing hardBreaks from the last paragraph
  if (trimmed.length > 0) {
    const last = trimmed[trimmed.length - 1];
    if (last.type === 'paragraph') {
      trimmed[trimmed.length - 1] = trimTrailingHardBreaks(last);
    }
  }

  return { ...json, content: trimmed };
}

/**
 * Check if TipTap JSON content is effectively empty
 * (no content, or only empty paragraphs)
 */
export function isContentEmpty(json: JSONContent): boolean {
  if (!json.content || json.content.length === 0) return true;
  return json.content.every(isEmptyParagraph);
}

/**
 * Find the first excalidrawChip node in TipTap JSON content.
 * Returns the chip's attributes if found, null otherwise.
 */
export function findExcalidrawChip(json: JSONContent): {
  nodeId: string;
  elements: string;
  image?: string;
} | null {
  function walk(node: JSONContent): ReturnType<typeof findExcalidrawChip> {
    if (node.type === 'excalidrawChip' && node.attrs) {
      return {
        // Fallback to generated UUID if nodeId is missing (legacy chips)
        nodeId: node.attrs.nodeId || crypto.randomUUID(),
        elements: node.attrs.elements,
        image: node.attrs.image,
      };
    }
    if (node.content) {
      for (const child of node.content) {
        const found = walk(child);
        if (found) return found;
      }
    }
    return null;
  }
  return walk(json);
}

/**
 * Replace the first excalidrawChip node in TipTap JSON content.
 * Returns a new JSONContent with the chip replaced, preserving other content.
 */
export function replaceExcalidrawChip(
  json: JSONContent,
  newChip: { type: 'excalidrawChip'; attrs: { elements: string; image: string } }
): JSONContent {
  let replaced = false;

  function walk(node: JSONContent): JSONContent {
    if (node.type === 'excalidrawChip' && !replaced) {
      replaced = true;
      return newChip;
    }
    if (node.content) {
      return {
        ...node,
        content: node.content.map(walk),
      };
    }
    return node;
  }

  return walk(json);
}

/**
 * Extract ContentNode array from TipTap JSON.
 * Handles text, tagChip, and mediaChip nodes.
 */
export function extractContentNodes(json: JSONContent): ContentNode[] {
  if (!json.content || json.content.length === 0) {
    return [];
  }

  const nodes: ContentNode[] = [];
  let pendingText = '';

  // Track list context for proper markdown formatting
  type ListContext = { type: 'bullet' | 'ordered'; index: number };
  const listStack: ListContext[] = [];

  function flushText() {
    if (!pendingText) return;

    // Parse for ```replace blocks
    // Format: ```replace\n{original}\n---\n{replacement}\n```
    const replacePattern = /```replace\n([\s\S]*?)\n---\n([\s\S]*?)\n```/g;
    let lastIndex = 0;
    let match;

    while ((match = replacePattern.exec(pendingText)) !== null) {
      // Add text before the match
      if (match.index > lastIndex) {
        const beforeText = pendingText.slice(lastIndex, match.index);
        if (beforeText.trim()) {
          nodes.push({ type: 'text', text: beforeText });
        }
      }

      // Add the replace node
      const original = match[1];
      const replacement = match[2];
      nodes.push({ type: 'replace', original, replacement });

      lastIndex = match.index + match[0].length;
    }

    // Add remaining text after last match
    if (lastIndex < pendingText.length) {
      const afterText = pendingText.slice(lastIndex);
      if (afterText.trim()) {
        nodes.push({ type: 'text', text: afterText });
      }
    } else if (lastIndex === 0) {
      // No matches found, add as plain text
      nodes.push({ type: 'text', text: pendingText });
    }

    pendingText = '';
  }

  function getListPrefix(): string {
    if (listStack.length === 0) return '';
    const indent = '  '.repeat(listStack.length - 1);
    const ctx = listStack[listStack.length - 1];
    if (ctx.type === 'bullet') {
      return `${indent}- `;
    } else {
      return `${indent}${ctx.index}. `;
    }
  }

  function walkNode(node: JSONContent) {
    if (node.type === 'text' && node.text) {
      // Apply marks as markdown (StarterKit v3 includes: bold, italic, strike, code, underline, link)
      let text = node.text;
      let linkHref: string | null = null;
      if (node.marks) {
        for (const mark of node.marks) {
          switch (mark.type) {
            case 'bold':
              text = `**${text}**`;
              break;
            case 'italic':
              text = `*${text}*`;
              break;
            case 'strike':
              text = `~~${text}~~`;
              break;
            case 'code':
              text = `\`${text}\``;
              break;
            case 'underline':
              // No standard markdown for underline, use HTML
              text = `<u>${text}</u>`;
              break;
            case 'link':
              // Capture href, apply after other marks
              linkHref = mark.attrs?.href ?? null;
              break;
          }
        }
        // Apply link last so it wraps the formatted text
        if (linkHref) {
          text = `[${text}](${linkHref})`;
        }
      }
      pendingText += text;
    } else if (node.type === 'tagChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'tag',
        id: node.attrs.id,
        name: node.attrs.name,
        instruction: node.attrs.instruction,
      });
    } else if (node.type === 'mediaChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'media',
        image: node.attrs.image,
        mime_type: node.attrs.mimeType,
      });
    } else if (node.type === 'excalidrawChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'excalidraw',
        elements: node.attrs.elements,
        image: node.attrs.image,
      });
    } else if (node.type === 'replacePreview' && node.attrs) {
      flushText();
      nodes.push({
        type: 'replace',
        original: node.attrs.original,
        replacement: node.attrs.replacement,
      });
    } else if (node.type === 'errorChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'error',
        source: node.attrs.source,
        message: node.attrs.message,
      });
    } else if (node.type === 'bulletList') {
      // Push bullet list context
      listStack.push({ type: 'bullet', index: 0 });
      if (node.content) {
        node.content.forEach(walkNode);
      }
      listStack.pop();
    } else if (node.type === 'orderedList') {
      // Push ordered list context (start from attrs or default to 1)
      const start = node.attrs?.start ?? 1;
      listStack.push({ type: 'ordered', index: start - 1 });
      if (node.content) {
        node.content.forEach(walkNode);
      }
      listStack.pop();
    } else if (node.type === 'listItem') {
      // Increment index for ordered lists
      if (listStack.length > 0) {
        listStack[listStack.length - 1].index++;
      }
      // Add newline before list item (except first item at top level)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      // Add list marker
      pendingText += getListPrefix();
      // Walk children but handle nested lists specially
      if (node.content) {
        for (const child of node.content) {
          if (child.type === 'paragraph') {
            // Don't add newline for first paragraph in list item
            if (child.content) {
              child.content.forEach(walkNode);
            }
          } else if (child.type === 'bulletList' || child.type === 'orderedList') {
            // Nested list - walk it
            walkNode(child);
          } else {
            walkNode(child);
          }
        }
      }
    } else if (node.type === 'hardBreak') {
      // Hard break within a paragraph - preserve as newline
      pendingText += '\n';
    } else if (node.type === 'paragraph') {
      // Add newline between paragraphs (except first)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      if (node.content) {
        node.content.forEach(walkNode);
      }
    } else if (node.content) {
      node.content.forEach(walkNode);
    }
  }

  json.content.forEach(walkNode);
  flushText();

  // Trim trailing whitespace from last text node
  if (nodes.length > 0) {
    const last = nodes[nodes.length - 1];
    if (last.type === 'text') {
      last.text = last.text.trimEnd();
      if (!last.text) {
        nodes.pop();
      }
    }
  }

  return nodes;
}

/**
 * Convert ContentNode array back to TipTap JSONContent.
 * Used to hydrate the editor with content from the backend.
 * Handles text nodes (with newlines as paragraph breaks), tag nodes, and media nodes.
 */
export function contentNodesToTipTap(nodes: ContentNode[] | null): JSONContent | undefined {
  if (!nodes || nodes.length === 0) {
    return undefined;
  }

  // Build paragraphs from content nodes
  const paragraphs: JSONContent[] = [];
  let currentParagraph: JSONContent[] = [];

  function flushParagraph() {
    paragraphs.push({
      type: 'paragraph',
      content: currentParagraph.length > 0 ? currentParagraph : [],
    });
    currentParagraph = [];
  }

  for (const node of nodes) {
    if (node.type === 'text') {
      // Split text by newlines into separate paragraphs
      const lines = node.text.split('\n');
      for (let i = 0; i < lines.length; i++) {
        if (i > 0) {
          flushParagraph();
        }
        if (lines[i]) {
          currentParagraph.push({ type: 'text', text: lines[i] });
        }
      }
    } else if (node.type === 'tag') {
      // Insert tag chip inline
      currentParagraph.push({
        type: 'tagChip',
        attrs: {
          id: node.id,
          name: node.name,
          instruction: node.instruction,
        },
      });
    } else if (node.type === 'media') {
      // Insert media chip inline
      currentParagraph.push({
        type: 'mediaChip',
        attrs: {
          image: node.image,
          mimeType: node.mime_type,
        },
      });
    } else if (node.type === 'excalidraw') {
      // Insert excalidraw chip inline
      currentParagraph.push({
        type: 'excalidrawChip',
        attrs: {
          nodeId: crypto.randomUUID(),
          elements: node.elements,
          image: node.image,
        },
      });
    } else if (node.type === 'replace') {
      // Flush current paragraph before block-level node
      if (currentParagraph.length > 0) {
        flushParagraph();
      }
      // Insert replace preview as block-level node
      paragraphs.push({
        type: 'replacePreview',
        attrs: {
          blockId: crypto.randomUUID(),
          original: node.original,
          replacement: node.replacement,
        },
      });
    } else if (node.type === 'error') {
      // Insert error chip inline
      currentParagraph.push({
        type: 'errorChip',
        attrs: {
          source: node.source,
          message: node.message,
        },
      });
    }
  }

  // Flush remaining content
  if (currentParagraph.length > 0 || paragraphs.length === 0) {
    flushParagraph();
  }

  return {
    type: 'doc',
    content: paragraphs,
  };
}

/**
 * Result of parsing a replace fence from TipTap JSON.
 */
export interface ParsedFence {
  /** The replacement text content inside the fence */
  replacement: string;
  /** Start index (inclusive) in doc.content of the fence */
  startIndex: number;
  /** End index (exclusive) in doc.content of the fence */
  endIndex: number;
}

/**
 * Parse an isolated replace fence from TipTap JSON content.
 * Returns null if no valid isolated fence is found.
 *
 * A valid fence must be isolated - the opening ```replace and closing ```
 * must each be the sole content of their paragraphs. Content paragraphs
 * between the markers are collected as replacement text.
 *
 * @param json - The TipTap document JSON
 * @returns ParsedFence if a valid isolated fence is found, null otherwise
 */
export function parseFenceFromJson(json: JSONContent): ParsedFence | null {
  if (!json.content) return null;

  // Helper to get the text content of a paragraph (if it's simple text only)
  const getParagraphText = (node: JSONContent): string | null => {
    if (node.type !== 'paragraph') return null;
    if (!node.content) return '';

    // Check for simple text-only content (no inline nodes like tagChip)
    const texts: string[] = [];
    for (const child of node.content) {
      if (child.type === 'text' && child.text) {
        texts.push(child.text);
      } else if (child.type === 'hardBreak') {
        texts.push('\n');
      } else {
        // Non-text content (tagChip, etc) - not isolated
        return null;
      }
    }
    return texts.join('');
  };

  // Find opening ```replace marker
  let startIndex = -1;
  for (let i = 0; i < json.content.length; i++) {
    const text = getParagraphText(json.content[i]);
    if (text === '```replace') {
      startIndex = i;
      break;
    }
  }

  if (startIndex === -1) return null;

  // Find closing ``` marker
  let endIndex = -1;
  const contentLines: string[] = [];

  for (let i = startIndex + 1; i < json.content.length; i++) {
    const text = getParagraphText(json.content[i]);
    if (text === null) {
      // Non-text content in the fence body - invalid
      return null;
    }
    if (text === '```') {
      endIndex = i + 1; // exclusive
      break;
    }
    contentLines.push(text);
  }

  if (endIndex === -1) return null;

  return {
    replacement: contentLines.join('\n'),
    startIndex,
    endIndex,
  };
}

/**
 * Transform TipTap JSON content: replace ```replace fence text with ReplacePreview node.
 * Only transforms isolated fences (where opening/closing markers are sole content of their paragraphs).
 *
 * @param json - The TipTap document JSON
 * @param original - The original content (from annotation context)
 * @param replacement - The replacement content (from parseFenceFromJson)
 * @returns Transformed JSON with ReplacePreview node instead of fence text
 */
export function transformReplaceFenceToPreview(
  json: JSONContent,
  original: string,
  replacement: string
): JSONContent {
  if (!json.content) return json;

  // Use the centralized parser to find an isolated fence
  const parsed = parseFenceFromJson(json);
  if (!parsed) {
    // No valid isolated fence found, return unchanged
    return json;
  }

  // Build new content: before fence + replacePreview + after fence
  const newContent: JSONContent[] = [
    // Content before the fence
    ...json.content.slice(0, parsed.startIndex),
    // The ReplacePreview node
    {
      type: 'replacePreview',
      attrs: {
        blockId: crypto.randomUUID(),
        original,
        replacement,
      },
    },
    // Content after the fence
    ...json.content.slice(parsed.endIndex),
  ];

  return { ...json, content: newContent };
}

/**
 * Transform TipTap JSON content: replace ReplacePreview node with fence text.
 * Used when unsealing to return to editable plain text.
 * Outputs isolated paragraphs matching the insertion format.
 *
 * @param json - The TipTap document JSON containing ReplacePreview node(s)
 * @returns Transformed JSON with fence text instead of ReplacePreview
 */
export function transformReplacePreviewToFence(json: JSONContent): JSONContent {
  if (!json.content) return json;

  const newContent: JSONContent[] = [];

  for (const node of json.content) {
    if (node.type === 'replacePreview' && node.attrs) {
      const replacement = (node.attrs.replacement as string) || '';
      // Convert to isolated paragraphs matching the insertion format
      newContent.push({ type: 'paragraph', content: [{ type: 'text', text: '```replace' }] });
      for (const line of replacement.split('\n')) {
        newContent.push({
          type: 'paragraph',
          content: line ? [{ type: 'text', text: line }] : undefined,
        });
      }
      newContent.push({ type: 'paragraph', content: [{ type: 'text', text: '```' }] });
    } else {
      newContent.push(node);
    }
  }

  return { ...json, content: newContent };
}
