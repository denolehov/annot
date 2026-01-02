import { Node, Extension, mergeAttributes, type JSONContent } from '@tiptap/core';
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';
import Suggestion, { type SuggestionOptions, type SuggestionProps, type SuggestionKeyDownProps } from '@tiptap/suggestion';
import { computePosition, offset, flip, shift, arrow } from '@floating-ui/dom';
import type { ContentNode, Tag, Bookmark } from './types';
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
const BookmarkSuggestionPluginKey = new PluginKey('bookmarkSuggestion');

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
 * BookmarkChip node - an inline, atomic node representing a bookmark reference.
 * Rendered as [@ k3u · Label...] in the editor.
 */
export const BookmarkChip = Node.create<BookmarkChipOptions>({
  name: 'bookmarkChip',
  group: 'inline',
  inline: true,
  atom: true, // Non-editable, treated as single unit

  addAttributes() {
    return {
      id: { default: null },
      label: { default: null },
      // Full bookmark data embedded at insertion time (for detachment)
      bookmark: { default: null },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-bookmark-chip]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          const bookmarkData = element.getAttribute('data-bookmark');
          return {
            id: element.getAttribute('data-id') || null,
            label: element.getAttribute('data-label') || '',
            bookmark: bookmarkData ? JSON.parse(bookmarkData) : null,
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    const shortId = node.attrs.id?.slice(0, 3) || '';
    const label = node.attrs.label || '';
    const displayLabel = label.length > 30 ? label.slice(0, 30) + '...' : label;
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-bookmark-chip': '',
        'data-id': node.attrs.id || '',
        'data-label': node.attrs.label || '',
        'data-bookmark': node.attrs.bookmark ? JSON.stringify(node.attrs.bookmark) : '',
        class: 'tag-chip bookmark-chip',
      }),
      `[@ ${shortId} · ${displayLabel}]`,
    ];
  },

  addNodeView() {
    return ({ node }) => {
      const { id, label } = node.attrs;
      const shortId = id?.slice(0, 3) || '';
      const displayLabel = label
        ? label.length > 25
          ? label.slice(0, 25) + '...'
          : label
        : '';

      const chip = document.createElement('span');
      chip.className = 'tag-chip bookmark-chip';
      chip.setAttribute('data-bookmark-chip', '');

      // Build tooltip content with bookmark details
      const tooltipContent = `<strong>${escapeHtml(id || '')}</strong><br>${escapeHtml(label || '')}`;

      chip.innerHTML = `
        <span class="bookmark-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" width="12" height="12"><path fill-rule="evenodd" d="M6.32 2.577a49.255 49.255 0 0 1 11.36 0c1.497.174 2.57 1.46 2.57 2.93V21a.75.75 0 0 1-1.085.67L12 18.089l-7.165 3.583A.75.75 0 0 1 3.75 21V5.507c0-1.47 1.073-2.756 2.57-2.93Z" clip-rule="evenodd" /></svg></span>
        <span class="bookmark-id">${escapeHtml(shortId)}</span>
        ${displayLabel ? `<span class="bookmark-divider">·</span><span class="bookmark-label">${escapeHtml(displayLabel)}</span>` : ''}
        <div class="chip-tooltip"><div class="chip-tooltip-content">${tooltipContent}</div><div class="chip-tooltip-arrow"></div></div>
      `;

      // Position tooltip on hover using Floating UI
      const tooltip = chip.querySelector('.chip-tooltip') as HTMLElement;
      const arrowEl = chip.querySelector('.chip-tooltip-arrow') as HTMLElement;

      const updatePosition = async () => {
        const { x, y, placement, middlewareData } = await computePosition(chip, tooltip, {
          placement: 'top',
          middleware: [offset(8), flip(), shift({ padding: 8 }), arrow({ element: arrowEl })],
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
          let isBookmarkChip = false;
          const { selection } = state;
          const { empty, anchor } = selection;

          if (!empty) return false;

          state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
            if (node.type.name === this.name) {
              isBookmarkChip = true;
              tr.insertText('', pos, pos + node.nodeSize);
              return false;
            }
          });

          return isBookmarkChip;
        }),
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        pluginKey: BookmarkSuggestionPluginKey,
        ...this.options.suggestion,
      }),
    ];
  },
});

export type BookmarkChipOptions = {
  suggestion: Omit<SuggestionOptions<Bookmark>, 'editor' | 'pluginKey'>;
};

/**
 * Determine if pasted text should be collapsed into a chip.
 * Focus on screen real estate - chip when content would visually dominate.
 */
export function shouldChip(text: string): boolean {
  if (!text) return false;

  const lines = text.split('\n');
  const lineCount = lines.length;
  const charCount = text.length;

  // Vertical sprawl: takes up too much height (6+ lines)
  if (lineCount >= 6) return true;

  // Horizontal sprawl: massive single/double line (minified, tokens, URLs)
  if (lineCount <= 2 && charCount >= 400) return true;

  return false;
}

/**
 * PasteChip node - an inline, atomic node representing pasted text content.
 * Rendered as [📋 Pasted (N lines)] in the editor with hover preview.
 */
export const PasteChip = Node.create({
  name: 'pasteChip',
  group: 'inline',
  inline: true,
  atom: true, // Non-editable, treated as single unit

  addAttributes() {
    return {
      content: { default: '' }, // Full pasted text
      lineCount: { default: 1 }, // For display label
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-paste-chip]',
        getAttrs: (dom) => {
          const element = dom as HTMLElement;
          return {
            content: element.getAttribute('data-content') || '',
            lineCount: parseInt(element.getAttribute('data-line-count') || '1', 10),
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    const label = node.attrs.lineCount > 1 ? `Pasted (${node.attrs.lineCount} lines)` : 'Pasted text';
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-paste-chip': '',
        'data-content': node.attrs.content,
        'data-line-count': node.attrs.lineCount,
        class: 'tag-chip paste-chip',
      }),
      `[📋 ${label}]`,
    ];
  },

  addNodeView() {
    return ({ node }) => {
      const { content, lineCount } = node.attrs;

      const chip = document.createElement('span');
      chip.className = 'tag-chip paste-chip';
      chip.setAttribute('data-paste-chip', '');

      const label = lineCount > 1 ? `Pasted (${lineCount} lines)` : 'Pasted text';

      // Build preview content (max 10 lines, then truncate)
      const lines = (content as string).split('\n');
      const maxPreviewLines = 10;
      const previewLines = lines.slice(0, maxPreviewLines);
      const hasMore = lines.length > maxPreviewLines;

      let previewHtml = `<pre class="paste-preview-content">${escapeHtml(previewLines.join('\n'))}</pre>`;
      if (hasMore) {
        previewHtml += `<div class="paste-preview-more">+${lines.length - maxPreviewLines} more lines</div>`;
      }

      chip.innerHTML = `
        <span class="tag-icon">📋</span>
        <span class="tag-content">${escapeHtml(label)}</span>
        <div class="chip-tooltip paste-tooltip"><div class="chip-tooltip-content">${previewHtml}</div><div class="chip-tooltip-arrow"></div></div>
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
          let isPasteChip = false;
          const { selection } = state;
          const { empty, anchor } = selection;

          if (!empty) return false;

          state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
            if (node.type.name === this.name) {
              isPasteChip = true;
              tr.insertText('', pos, pos + node.nodeSize);
              return false;
            }
          });

          return isPasteChip;
        }),
    };
  },
});

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
 * TextPasteHandler extension - intercepts text paste events and inserts PasteChip nodes
 * for large text content that would visually dominate the editor.
 */
export const TextPasteHandler = Extension.create({
  name: 'textPasteHandler',

  addProseMirrorPlugins() {
    const editor = this.editor;

    return [
      new Plugin({
        key: new PluginKey('textPasteHandler'),
        props: {
          handlePaste(view, event) {
            const clipboardData = event.clipboardData;
            if (!clipboardData) return false;

            // Only handle if there's no image (let ImagePasteHandler handle those)
            const hasImage = Array.from(clipboardData.items).some((item) =>
              item.type.startsWith('image/')
            );
            if (hasImage) return false;

            // Get plain text from clipboard
            const text = clipboardData.getData('text/plain');
            if (!text) return false;

            // Check if this text should be chipped
            if (!shouldChip(text)) return false;

            // Insert PasteChip instead of raw text
            const lineCount = text.split('\n').length;
            editor
              .chain()
              .focus()
              .insertContent([
                {
                  type: 'pasteChip',
                  attrs: {
                    content: text,
                    lineCount,
                  },
                },
                { type: 'text', text: ' ' },
              ])
              .run();

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

// ============================================================================
// extractContentNodes: Transform TipTap JSON to ContentNode[]
// ============================================================================

/**
 * Mark-to-Markdown wrappers.
 * Maps TipTap mark types to their markdown formatting functions.
 */
const MARK_WRAPPERS: Record<string, (text: string) => string> = {
  bold: (t) => `**${t}**`,
  italic: (t) => `*${t}*`,
  strike: (t) => `~~${t}~~`,
  code: (t) => `\`${t}\``,
  underline: (t) => `<u>${t}</u>`, // No standard markdown, use HTML
};

/**
 * Apply TipTap marks to text, converting to markdown format.
 * Links are applied last to wrap the formatted text.
 */
function applyMarks(text: string, marks?: JSONContent['marks']): string {
  if (!marks) return text;

  let result = text;
  let linkHref: string | null = null;

  for (const mark of marks) {
    if (mark.type === 'link') {
      linkHref = mark.attrs?.href ?? null;
    } else if (MARK_WRAPPERS[mark.type]) {
      result = MARK_WRAPPERS[mark.type](result);
    }
  }

  return linkHref ? `[${result}](${linkHref})` : result;
}

/**
 * Chip extractors registry.
 * Maps TipTap chip node types to ContentNode factory functions.
 * Adding a new chip type = add one entry here.
 */
type ChipExtractor = (attrs: Record<string, unknown>) => ContentNode;

const CHIP_EXTRACTORS: Record<string, ChipExtractor> = {
  tagChip: (attrs) => ({
    type: 'tag',
    id: attrs.id as string,
    name: attrs.name as string,
    instruction: attrs.instruction as string,
  }),
  mediaChip: (attrs) => ({
    type: 'media',
    image: attrs.image as string,
    mime_type: attrs.mimeType as string,
  }),
  excalidrawChip: (attrs) => ({
    type: 'excalidraw',
    elements: attrs.elements as string,
    image: attrs.image as string | undefined,
  }),
  replacePreview: (attrs) => ({
    type: 'replace',
    original: attrs.original as string,
    replacement: attrs.replacement as string,
  }),
  errorChip: (attrs) => ({
    type: 'error',
    source: attrs.source as string,
    message: attrs.message as string,
  }),
  pasteChip: (attrs) => ({
    type: 'paste',
    content: attrs.content as string,
  }),
  bookmarkChip: (attrs) => ({
    type: 'bookmarkref',
    id: attrs.id as string,
    label: attrs.label as string,
    bookmark: attrs.bookmark as Bookmark,
  }),
};

/**
 * Text accumulator with embedded replace block parsing.
 * Encapsulates pending text buffer and parsed ContentNode output.
 */
class TextAccumulator {
  private pending = '';
  private nodes: ContentNode[] = [];

  append(text: string): void {
    this.pending += text;
  }

  pushNode(node: ContentNode): void {
    this.flush();
    this.nodes.push(node);
  }

  flush(): void {
    if (!this.pending) return;
    this.parseAndPushText(this.pending);
    this.pending = '';
  }

  /**
   * Parse text for embedded ```replace blocks and push as nodes.
   * Format: ```replace\n{original}\n---\n{replacement}\n```
   */
  private parseAndPushText(text: string): void {
    const REPLACE_PATTERN = /```replace\n([\s\S]*?)\n---\n([\s\S]*?)\n```/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = REPLACE_PATTERN.exec(text)) !== null) {
      // Add text before the match
      if (match.index > lastIndex) {
        const beforeText = text.slice(lastIndex, match.index);
        if (beforeText.trim()) {
          this.nodes.push({ type: 'text', text: beforeText });
        }
      }

      // Add the replace node
      this.nodes.push({
        type: 'replace',
        original: match[1],
        replacement: match[2],
      });

      lastIndex = match.index + match[0].length;
    }

    // Add remaining text after last match
    if (lastIndex < text.length) {
      const afterText = text.slice(lastIndex);
      if (afterText.trim()) {
        this.nodes.push({ type: 'text', text: afterText });
      }
    } else if (lastIndex === 0) {
      // No matches found, add as plain text
      this.nodes.push({ type: 'text', text });
    }
  }

  getNodes(): ContentNode[] {
    return this.nodes;
  }

  get hasContent(): boolean {
    return this.pending.length > 0 || this.nodes.length > 0;
  }
}

/**
 * List context manager for markdown list formatting.
 * Tracks nested list state (bullet vs ordered, current index).
 */
type ListType = 'bullet' | 'ordered';

class ListContext {
  private stack: Array<{ type: ListType; index: number }> = [];

  enter(type: ListType, start = 1): void {
    this.stack.push({ type, index: type === 'ordered' ? start - 1 : 0 });
  }

  exit(): void {
    this.stack.pop();
  }

  incrementIndex(): void {
    if (this.stack.length > 0) {
      this.stack[this.stack.length - 1].index++;
    }
  }

  getPrefix(): string {
    if (this.stack.length === 0) return '';
    const indent = '  '.repeat(this.stack.length - 1);
    const ctx = this.stack[this.stack.length - 1];
    return ctx.type === 'bullet' ? `${indent}- ` : `${indent}${ctx.index}. `;
  }
}

/**
 * Extract ContentNode array from TipTap JSONContent.
 * Transforms the rich text tree into a flat array suitable for backend storage and LLM consumption.
 */
export function extractContentNodes(json: JSONContent): ContentNode[] {
  if (!json.content?.length) return [];

  const accumulator = new TextAccumulator();
  const listCtx = new ListContext();

  function walk(node: JSONContent): void {
    const { type, attrs, content, text, marks } = node;

    // Text node with optional marks
    if (type === 'text' && text) {
      accumulator.append(applyMarks(text, marks));
      return;
    }

    // Chip node — check extractors registry
    if (type && attrs && CHIP_EXTRACTORS[type]) {
      accumulator.pushNode(CHIP_EXTRACTORS[type](attrs));
      return;
    }

    // Structural nodes
    switch (type) {
      case 'bulletList':
        listCtx.enter('bullet');
        content?.forEach(walk);
        listCtx.exit();
        break;

      case 'orderedList':
        listCtx.enter('ordered', (attrs?.start as number) ?? 1);
        content?.forEach(walk);
        listCtx.exit();
        break;

      case 'listItem':
        listCtx.incrementIndex();
        if (accumulator.hasContent) accumulator.append('\n');
        accumulator.append(listCtx.getPrefix());
        // Walk children, handling paragraph wrapper specially
        for (const child of content ?? []) {
          if (child.type === 'paragraph') {
            // Don't add newline for first paragraph in list item
            child.content?.forEach(walk);
          } else {
            walk(child);
          }
        }
        break;

      case 'hardBreak':
        accumulator.append('\n');
        break;

      case 'paragraph':
        if (accumulator.hasContent) accumulator.append('\n');
        content?.forEach(walk);
        break;

      default:
        // Generic container — recurse into children
        content?.forEach(walk);
    }
  }

  json.content.forEach(walk);
  accumulator.flush();

  // Trim trailing whitespace from last text node
  const nodes = accumulator.getNodes();
  if (nodes.length > 0) {
    const last = nodes[nodes.length - 1];
    if (last.type === 'text') {
      last.text = last.text.trimEnd();
      if (!last.text) nodes.pop();
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
    } else if (node.type === 'paste') {
      // Insert paste chip inline
      const lineCount = node.content.split('\n').length;
      currentParagraph.push({
        type: 'pasteChip',
        attrs: {
          content: node.content,
          lineCount,
        },
      });
    } else if (node.type === 'bookmarkref') {
      // Insert bookmark chip inline with full embedded bookmark data
      currentParagraph.push({
        type: 'bookmarkChip',
        attrs: {
          id: node.id,
          label: node.label,
          bookmark: node.bookmark,
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
