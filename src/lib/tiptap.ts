import { Node, mergeAttributes, type JSONContent } from '@tiptap/core';
import Suggestion, { type SuggestionOptions } from '@tiptap/suggestion';
import type { ContentNode, Tag } from './types';

/**
 * TagChip node - an inline, atomic node representing a tag.
 * Rendered as [# TAG_NAME] in the editor.
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
        ...this.options.suggestion,
      }),
    ];
  },
});

export type TagChipOptions = {
  suggestion: Omit<SuggestionOptions<Tag>, 'editor'>;
};

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
 * Handles text and tagChip nodes.
 */
export function extractContentNodes(json: JSONContent): ContentNode[] {
  if (!json.content || json.content.length === 0) {
    return [];
  }

  const nodes: ContentNode[] = [];
  let pendingText = '';

  function flushText() {
    if (pendingText) {
      nodes.push({ type: 'text', text: pendingText });
      pendingText = '';
    }
  }

  function walkNode(node: JSONContent) {
    if (node.type === 'text' && node.text) {
      pendingText += node.text;
    } else if (node.type === 'tagChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'tag',
        id: node.attrs.id,
        name: node.attrs.name,
        instruction: node.attrs.instruction,
      });
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
 * Handles text nodes (with newlines as paragraph breaks) and tag nodes.
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
