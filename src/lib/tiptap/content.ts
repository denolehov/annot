import type { JSONContent } from '@tiptap/core';

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
