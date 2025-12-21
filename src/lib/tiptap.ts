import { Node, Extension, mergeAttributes, type JSONContent } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import Suggestion, { type SuggestionOptions, type SuggestionProps } from '@tiptap/suggestion';
import type { ContentNode, Tag } from './types';

// Unique plugin keys for each suggestion type
const TagSuggestionPluginKey = new PluginKey('tagSuggestion');
const SlashSuggestionPluginKey = new PluginKey('slashSuggestion');

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
            detail: { pos, elements: node.attrs.elements },
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

  parseHTML() {
    return [{ tag: 'span[data-excalidraw-placeholder]' }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'span',
      mergeAttributes(HTMLAttributes, {
        'data-excalidraw-placeholder': '',
        class: 'tag-chip excalidraw-placeholder',
      }),
      '[📐 Drawing...]',
    ];
  },

  addNodeView() {
    return ({ getPos }) => {
      const chip = document.createElement('span');
      chip.className = 'tag-chip excalidraw-placeholder';
      chip.innerHTML = `
        <span class="tag-icon">📐</span>
        <span class="tag-content">Drawing...</span>
      `;

      // Dispatch event to open modal immediately
      requestAnimationFrame(() => {
        const pos = typeof getPos === 'function' ? getPos() : null;
        if (pos !== null) {
          const event = new CustomEvent('excalidraw-create', {
            bubbles: true,
            detail: { pos },
          });
          chip.dispatchEvent(event);
        }
      });

      return { dom: chip };
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
 * Only active in ephemeral mode.
 */
export interface ImagePasteHandlerOptions {
  ephemeral: boolean;
  onPasteBlocked?: () => void;
}

export const ImagePasteHandler = Extension.create<ImagePasteHandlerOptions>({
  name: 'imagePasteHandler',

  addOptions() {
    return {
      ephemeral: false,
      onPasteBlocked: undefined,
    };
  },

  addStorage() {
    return {
      ephemeral: this.options.ephemeral,
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

            // Check ephemeral from storage
            const { ephemeral } = extension.storage;
            const { onPasteBlocked } = extension.options;

            // Block paste if not in ephemeral mode
            if (!ephemeral) {
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
 * Create the suggestion configuration for slash commands.
 */
export function createSlashSuggestion(): Omit<SuggestionOptions<SlashCommand>, 'editor'> {
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
            { type: 'excalidrawPlaceholder' },
            { type: 'text', text: ' ' },
          ])
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
    if (pendingText) {
      nodes.push({ type: 'text', text: pendingText });
      pendingText = '';
    }
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
          elements: node.elements,
          image: node.image,
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
