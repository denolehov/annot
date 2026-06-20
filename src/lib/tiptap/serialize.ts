import type { JSONContent } from '@tiptap/core';
import type { ContentNode, RefSnapshot } from '../types';

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
};

/**
 * Apply TipTap marks to text, converting to markdown format.
 */
function applyMarks(text: string, marks?: JSONContent['marks']): string {
  if (!marks) return text;

  let result = text;
  for (const mark of marks) {
    if (MARK_WRAPPERS[mark.type]) {
      result = MARK_WRAPPERS[mark.type](result);
    }
  }
  return result;
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
  refChip: (attrs) => {
    // File refs have a path attribute instead of snapshot
    if (attrs.refType === 'file' && attrs.path) {
      return {
        type: 'file',
        path: attrs.path as string,
      };
    }
    // Heading refs have section attributes
    if (attrs.refType === 'heading') {
      return {
        type: 'ref',
        ref_type: 'heading',
        snapshot: {
          type: 'heading',
          line: attrs.sectionLine as number,
          level: attrs.sectionLevel as number,
          title: attrs.sectionTitle as string,
        },
      };
    }
    // Annotation refs have snapshot
    return {
      type: 'ref',
      ref_type: attrs.refType as 'annotation',
      snapshot: attrs.snapshot as RefSnapshot,
    };
  },
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
    } else if (node.type === 'ref') {
      // Unified ref chip inline - handles annotation and heading refs
      currentParagraph.push({
        type: 'refChip',
        attrs: {
          refType: node.ref_type,
          snapshot: node.snapshot,
        },
      });
    } else if (node.type === 'file') {
      // File ref chip inline
      currentParagraph.push({
        type: 'refChip',
        attrs: {
          refType: 'file',
          path: node.path,
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
