import type { JSONContent } from '@tiptap/core';

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
