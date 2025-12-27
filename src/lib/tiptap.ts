import { Node, Extension, mergeAttributes, type JSONContent } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import Suggestion, { type SuggestionOptions, type SuggestionProps, type SuggestionKeyDownProps } from '@tiptap/suggestion';
import type { ContentNode, Tag } from './types';

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
        ${tooltipContent ? `<div class="chip-tooltip">${tooltipContent}</div>` : ''}
      `;

      // Position tooltip on hover (for position: fixed)
      if (tooltipContent) {
        chip.addEventListener('mouseenter', () => {
          const rect = chip.getBoundingClientRect();
          chip.style.setProperty('--tooltip-x', `${rect.left + rect.width / 2}px`);
          chip.style.setProperty('--tooltip-y', `${rect.top}px`);
        });
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
 * ReplaceBlock node - editable code block for proposing code replacements.
 * Inserted via /replace slash command. Transforms to ReplacePreview on seal.
 */
export const ReplaceBlock = Node.create({
  name: 'replaceBlock',
  group: 'block',
  content: 'text*',
  code: true,
  isolating: true,

  addAttributes() {
    return {
      blockId: {
        default: null,
        parseHTML: (element) =>
          element.getAttribute('data-block-id') || crypto.randomUUID(),
      },
      original: { default: '' }, // Original content captured at /replace time
    };
  },

  parseHTML() {
    return [
      {
        tag: 'div[data-replace-block]',
        preserveWhitespace: 'full',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            blockId: element.getAttribute('data-block-id') || crypto.randomUUID(),
            original: element.getAttribute('data-original') || '',
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(HTMLAttributes, {
        'data-replace-block': '',
        'data-block-id': node.attrs.blockId,
        'data-original': node.attrs.original,
        class: 'replace-block',
      }),
      ['pre', { class: 'replace-block-content' }, 0],
    ];
  },

  addNodeView() {
    return ({ node }) => {
      const { original } = node.attrs;

      const wrapper = document.createElement('div');
      wrapper.className = 'replace-block';
      wrapper.setAttribute('data-replace-block', '');

      // Header
      const header = document.createElement('div');
      header.className = 'replace-block-header';
      header.textContent = 'REPLACE';

      // Original section (read-only)
      const originalSection = document.createElement('div');
      originalSection.className = 'replace-block-original';

      const originalLabel = document.createElement('div');
      originalLabel.className = 'replace-block-label';
      originalLabel.textContent = 'Original:';

      const originalPre = document.createElement('pre');
      originalPre.className = 'replace-block-original-code';
      originalPre.textContent = original || '(empty)';

      originalSection.appendChild(originalLabel);
      originalSection.appendChild(originalPre);

      // Replacement section (editable)
      const replacementSection = document.createElement('div');
      replacementSection.className = 'replace-block-replacement';

      const replacementLabel = document.createElement('div');
      replacementLabel.className = 'replace-block-label';
      replacementLabel.textContent = 'Replacement:';

      const content = document.createElement('pre');
      content.className = 'replace-block-content';

      replacementSection.appendChild(replacementLabel);
      replacementSection.appendChild(content);

      wrapper.appendChild(header);
      wrapper.appendChild(originalSection);
      wrapper.appendChild(replacementSection);

      return {
        dom: wrapper,
        contentDOM: content,
      };
    };
  },

  addKeyboardShortcuts() {
    return {
      // Exit the code block with arrow down at end
      ArrowDown: ({ editor }) => {
        const { state } = editor;
        const { selection } = state;
        const { $from, empty } = selection;

        if (!empty) return false;

        const node = $from.node();
        if (node.type.name !== this.name) return false;

        // Check if cursor is at the end of the block
        const isAtEnd = $from.parentOffset === node.content.size;
        if (!isAtEnd) return false;

        // Move cursor after this block
        const pos = $from.after();
        editor.commands.setTextSelection(pos);
        return true;
      },
      // Exit the code block with arrow up at start
      ArrowUp: ({ editor }) => {
        const { state } = editor;
        const { selection } = state;
        const { $from, empty } = selection;

        if (!empty) return false;

        const node = $from.node();
        if (node.type.name !== this.name) return false;

        // Check if cursor is at the start of the block
        const isAtStart = $from.parentOffset === 0;
        if (!isAtStart) return false;

        // Move cursor before this block
        const pos = $from.before();
        editor.commands.setTextSelection(pos);
        return true;
      },
      // Tab to exit after block
      Tab: ({ editor }) => {
        const { state } = editor;
        const { selection } = state;
        const { $from } = selection;

        const node = $from.node();
        if (node.type.name !== this.name) return false;

        // Move cursor after this block
        const pos = $from.after();
        editor.commands.setTextSelection(pos);
        return true;
      },
      // Shift+Tab to exit before block
      'Shift-Tab': ({ editor }) => {
        const { state } = editor;
        const { selection } = state;
        const { $from } = selection;

        const node = $from.node();
        if (node.type.name !== this.name) return false;

        // Move cursor before this block
        const pos = $from.before();
        editor.commands.setTextSelection(pos);
        return true;
      },
    };
  },
});

/**
 * ReplacePreview node - sealed atomic node showing a diff preview.
 * Created from ReplaceBlock on annotation seal. Click to unseal.
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

      const header = document.createElement('div');
      header.className = 'replace-preview-header';
      header.textContent = 'REPLACE';

      const diffContainer = document.createElement('pre');
      diffContainer.className = 'replace-preview-diff';

      // Generate diff display using LCS algorithm
      const originalLines = original.split('\n');
      const replacementLines = replacement.split('\n');
      const diff = computeDiff(originalLines, replacementLines);

      let diffHtml = '';
      for (const { type, line } of diff) {
        if (type === 'equal') {
          diffHtml += `<div class="diff-line diff-context">  ${escapeHtml(line)}</div>`;
        } else if (type === 'delete') {
          diffHtml += `<div class="diff-line diff-removed">- ${escapeHtml(line)}</div>`;
        } else {
          diffHtml += `<div class="diff-line diff-added">+ ${escapeHtml(line)}</div>`;
        }
      }
      diffContainer.innerHTML = diffHtml;

      wrapper.appendChild(header);
      wrapper.appendChild(diffContainer);

      // No click handler here - clicks bubble up to the sealed editor container,
      // which calls onUnseal(). The unseal effect in useAnnotationEditor transforms
      // all ReplacePreview nodes to ReplaceBlock and focuses the editor.

      return { dom: wrapper };
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
      return tags
        .filter((tag) => tag.name.toLowerCase().includes(query.toLowerCase()))
        .slice(0, 5);
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
      description: 'Propose code replacement',
      icon: '✏️',
      action: (editor, range) => {
        // Check if there's already a replace block (limit to one per annotation)
        let hasReplaceBlock = false;
        editor.state.doc.descendants((node) => {
          if (node.type.name === 'replaceBlock' || node.type.name === 'replacePreview') {
            hasReplaceBlock = true;
            return false;
          }
        });
        if (hasReplaceBlock) {
          editor.chain().focus().deleteRange(range).run();
          return;
        }

        const original = getOriginalLines?.() ?? '';
        if (!original) {
          editor.chain().focus().deleteRange(range).run();
          return;
        }

        // Insert ReplaceBlock node with original stored in attrs
        editor
          .chain()
          .focus()
          .deleteRange(range)
          .insertContent({
            type: 'replaceBlock',
            attrs: {
              blockId: crypto.randomUUID(),
              original: original,
            },
            content: [{ type: 'text', text: original }],
          })
          .run();
      },
    },
  ];

  return {
    char: '/',
    items: ({ query }) => {
      return commands.filter((cmd) =>
        cmd.name.toLowerCase().includes(query.toLowerCase())
      );
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
    } else if (node.type === 'heading') {
      // Add newline before heading (except first)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      // Add markdown heading marker based on level
      const level = node.attrs?.level ?? 1;
      pendingText += '#'.repeat(level) + ' ';
      if (node.content) {
        node.content.forEach(walkNode);
      }
    } else if (node.type === 'blockquote') {
      // Add newline before blockquote (except first)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      // Collect blockquote content, then prefix each line with >
      const savedText = pendingText;
      pendingText = '';
      if (node.content) {
        node.content.forEach(walkNode);
      }
      const quotedContent = pendingText;
      pendingText = savedText;
      // Prefix each line with >
      const lines = quotedContent.split('\n');
      pendingText += lines.map(line => `> ${line}`).join('\n');
    } else if (node.type === 'codeBlock') {
      // Add newline before code block (except first)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      const language = node.attrs?.language ?? '';
      pendingText += '```' + language + '\n';
      // Code blocks contain text directly, not paragraphs
      if (node.content) {
        for (const child of node.content) {
          if (child.type === 'text' && child.text) {
            pendingText += child.text;
          }
        }
      }
      pendingText += '\n```';
    } else if (node.type === 'hardBreak') {
      // Hard break within a paragraph - preserve as newline
      pendingText += '\n';
    } else if (node.type === 'horizontalRule') {
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      pendingText += '---';
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
