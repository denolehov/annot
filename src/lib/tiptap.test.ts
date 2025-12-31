import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  trimContent,
  isContentEmpty,
  EditorShortcuts,
  extractContentNodes,
  parseFenceFromJson,
  transformReplaceFenceToPreview,
  transformReplacePreviewToFence,
  ReplacePreview,
  shouldChip,
} from './tiptap';
import { Editor, type JSONContent } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';

// ============================================================================
// Test Helpers - Factory functions for TipTap JSON nodes
// ============================================================================

/** Create a text node, optionally with marks */
const text = (t: string, marks?: Array<{ type: string; attrs?: Record<string, unknown> }>): JSONContent =>
  marks ? { type: 'text', text: t, marks } : { type: 'text', text: t };

/** Create a paragraph node with inline content */
const p = (...content: (string | JSONContent)[]): JSONContent => ({
  type: 'paragraph',
  content: content.length > 0
    ? content.map((c) => (typeof c === 'string' ? text(c) : c))
    : undefined,
});

/** Create a document node with block content */
const doc = (...content: JSONContent[]): JSONContent => ({
  type: 'doc',
  content,
});

/** Create a tagChip node */
const tagChip = (id: string, name: string, instruction: string): JSONContent => ({
  type: 'tagChip',
  attrs: { id, name, instruction },
});

/** Create a replacePreview node */
const replacePreview = (original: string, replacement: string, blockId?: string): JSONContent => ({
  type: 'replacePreview',
  attrs: { blockId: blockId ?? 'test-id', original, replacement },
});

/** Create a hardBreak node */
const hardBreak = (): JSONContent => ({ type: 'hardBreak' });

describe('trimContent', () => {
  it('removes trailing empty paragraphs', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Hello' }] },
        { type: 'paragraph' },
        { type: 'paragraph', content: [] },
      ],
    };
    const result = trimContent(input);
    expect(result.content).toHaveLength(1);
    expect(result.content![0].content![0].text).toBe('Hello');
  });

  it('removes paragraphs with only whitespace', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Hello' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '   ' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '\n\t' }] },
      ],
    };
    const result = trimContent(input);
    expect(result.content).toHaveLength(1);
  });

  it('preserves non-empty content', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Line 1' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'Line 2' }] },
      ],
    };
    const result = trimContent(input);
    expect(result.content).toHaveLength(2);
  });

  it('handles empty document', () => {
    const input: JSONContent = { type: 'doc', content: [] };
    const result = trimContent(input);
    expect(result.content).toEqual([]);
  });

  it('handles document with no content property', () => {
    const input: JSONContent = { type: 'doc' };
    const result = trimContent(input);
    expect(result).toEqual({ type: 'doc' });
  });

  it('does not mutate the original', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Hello' }] },
        { type: 'paragraph' },
      ],
    };
    const original = JSON.stringify(input);
    trimContent(input);
    expect(JSON.stringify(input)).toBe(original);
  });

  it('preserves non-paragraph nodes', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Hello' }] },
        { type: 'bulletList', content: [{ type: 'listItem' }] },
        { type: 'paragraph' },
      ],
    };
    const result = trimContent(input);
    expect(result.content).toHaveLength(2);
    expect(result.content![1].type).toBe('bulletList');
  });
});

describe('isContentEmpty', () => {
  it('returns true for empty content array', () => {
    expect(isContentEmpty({ type: 'doc', content: [] })).toBe(true);
  });

  it('returns true for missing content property', () => {
    expect(isContentEmpty({ type: 'doc' })).toBe(true);
  });

  it('returns true for only empty paragraphs', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [{ type: 'paragraph' }, { type: 'paragraph', content: [] }],
    };
    expect(isContentEmpty(input)).toBe(true);
  });

  it('returns true for paragraphs with only whitespace', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text: '   ' }] }],
    };
    expect(isContentEmpty(input)).toBe(true);
  });

  it('returns false for non-empty content', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Hello' }] }],
    };
    expect(isContentEmpty(input)).toBe(false);
  });
});

describe('EditorShortcuts', () => {
  let editor: Editor;
  let container: HTMLDivElement;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    editor?.destroy();
    container?.remove();
  });

  it('calls onSubmit on Ctrl-Enter and prevents newline insertion', () => {
    // Note: TipTap's "Mod-Enter" maps to Ctrl+Enter in JSDOM (non-macOS environment).
    // On real macOS, Mod maps to Cmd. This test verifies the core behavior works.
    const onSubmit = vi.fn();

    editor = new Editor({
      element: container,
      extensions: [
        StarterKit,
        EditorShortcuts.configure({ onSubmit }),
      ],
      content: '<p>Hello</p>',
    });

    editor.commands.focus();

    const contentBefore = editor.getText();

    // Simulate Ctrl-Enter
    const event = new KeyboardEvent('keydown', {
      key: 'Enter',
      ctrlKey: true,
      bubbles: true,
    });
    container.querySelector('.ProseMirror')?.dispatchEvent(event);

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(editor.getText()).toBe(contentBefore);
  });

  it('calls onDismiss on Escape', () => {
    const onDismiss = vi.fn();

    editor = new Editor({
      element: container,
      extensions: [
        StarterKit,
        EditorShortcuts.configure({ onDismiss }),
      ],
      content: '<p>Hello</p>',
    });

    editor.commands.focus();

    // Simulate Escape keydown
    const event = new KeyboardEvent('keydown', {
      key: 'Escape',
      bubbles: true,
    });
    container.querySelector('.ProseMirror')?.dispatchEvent(event);

    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('does not call callbacks when Enter is pressed without modifier', () => {
    const onSubmit = vi.fn();

    editor = new Editor({
      element: container,
      extensions: [
        StarterKit,
        EditorShortcuts.configure({ onSubmit }),
      ],
      content: '<p>Hello</p>',
    });

    editor.commands.focus();

    // Simulate plain Enter
    const event = new KeyboardEvent('keydown', {
      key: 'Enter',
      bubbles: true,
    });
    container.querySelector('.ProseMirror')?.dispatchEvent(event);

    // onSubmit should NOT be called for plain Enter
    expect(onSubmit).not.toHaveBeenCalled();
  });
});

describe('extractContentNodes', () => {
  it('preserves bold formatting as markdown', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'normal ' },
            { type: 'text', text: 'bold', marks: [{ type: 'bold' }] },
            { type: 'text', text: ' text' },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'normal **bold** text' });
  });

  it('preserves italic formatting as markdown', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'some ' },
            { type: 'text', text: 'italic', marks: [{ type: 'italic' }] },
            { type: 'text', text: ' here' },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'some *italic* here' });
  });

  it('preserves strikethrough formatting as markdown', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'crossed ' },
            { type: 'text', text: 'out', marks: [{ type: 'strike' }] },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'crossed ~~out~~' });
  });

  it('preserves inline code formatting as markdown', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'run ' },
            { type: 'text', text: 'npm install', marks: [{ type: 'code' }] },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'run `npm install`' });
  });

  it('handles multiple marks on same text (bold+italic)', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: 'emphasis',
              marks: [{ type: 'bold' }, { type: 'italic' }],
            },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    // Bold wraps first, then italic wraps around that
    expect(nodes[0]).toEqual({ type: 'text', text: '***emphasis***' });
  });

  it('preserves underline formatting as HTML', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'some ' },
            { type: 'text', text: 'underlined', marks: [{ type: 'underline' }] },
            { type: 'text', text: ' text' },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'some <u>underlined</u> text' });
  });

  it('preserves link formatting as markdown', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'click ' },
            {
              type: 'text',
              text: 'here',
              marks: [{ type: 'link', attrs: { href: 'https://example.com' } }],
            },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'click [here](https://example.com)' });
  });

  it('handles link with other formatting (bold link)', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: 'important',
              marks: [
                { type: 'bold' },
                { type: 'link', attrs: { href: 'https://example.com' } },
              ],
            },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    // Bold applied first, then wrapped in link
    expect(nodes[0]).toEqual({ type: 'text', text: '[**important**](https://example.com)' });
  });

  it('preserves bullet list formatting', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'bulletList',
          content: [
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Item 1' }] }] },
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Item 2' }] }] },
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Item 3' }] }] },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: '- Item 1\n- Item 2\n- Item 3' });
  });

  it('preserves ordered list formatting', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'orderedList',
          content: [
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'First' }] }] },
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Second' }] }] },
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Third' }] }] },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: '1. First\n2. Second\n3. Third' });
  });

  it('preserves nested list formatting', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'bulletList',
          content: [
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Item 1' }] }] },
            {
              type: 'listItem',
              content: [
                { type: 'paragraph', content: [{ type: 'text', text: 'Item 2' }] },
                {
                  type: 'bulletList',
                  content: [
                    { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Sub 1' }] }] },
                    { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Sub 2' }] }] },
                  ],
                },
              ],
            },
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Item 3' }] }] },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({
      type: 'text',
      text: '- Item 1\n- Item 2\n  - Sub 1\n  - Sub 2\n- Item 3',
    });
  });

  it('preserves hard breaks', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'Line one' },
            { type: 'hardBreak' },
            { type: 'text', text: 'Line two' },
          ],
        },
      ],
    };
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'text', text: 'Line one\nLine two' });
  });

});

describe('parseFenceFromJson', () => {
  it('parses a simple isolated fence', () => {
    const input = doc(
      p('```replace'),
      p('const x = 2;'),
      p('```')
    );

    const result = parseFenceFromJson(input);

    expect(result).toEqual({
      replacement: 'const x = 2;',
      startIndex: 0,
      endIndex: 3,
    });
  });

  it('parses fence with content before', () => {
    const input = doc(
      p('Some text'),
      p('```replace'),
      p('code'),
      p('```')
    );

    const result = parseFenceFromJson(input);

    expect(result).toEqual({
      replacement: 'code',
      startIndex: 1,
      endIndex: 4,
    });
  });

  it('returns null for non-isolated fence (mixed content in opening marker)', () => {
    const input = doc(p('prefix ```replace'), p('code'), p('```'));

    const result = parseFenceFromJson(input);

    expect(result).toBeNull();
  });

  it('returns null when opening marker is not alone', () => {
    const input = doc(
      p(tagChip('1', 'FIX', 'Fix'), '```replace'),
      p('code'),
      p('```')
    );

    const result = parseFenceFromJson(input);

    expect(result).toBeNull();
  });

  it('returns null when no fence present', () => {
    const input = doc(p('Just some text'));

    const result = parseFenceFromJson(input);

    expect(result).toBeNull();
  });

  it('returns null when closing fence is missing', () => {
    const input = doc(
      p('```replace'),
      p('code'),
      p('more code')
    );

    const result = parseFenceFromJson(input);

    expect(result).toBeNull();
  });

  it('handles multiline content', () => {
    const input = doc(
      p('```replace'),
      p('line 1'),
      p('line 2'),
      p('line 3'),
      p('```')
    );

    const result = parseFenceFromJson(input);

    expect(result).toEqual({
      replacement: 'line 1\nline 2\nline 3',
      startIndex: 0,
      endIndex: 5,
    });
  });

  it('handles empty lines in content', () => {
    const input = doc(
      p('```replace'),
      p('line 1'),
      p(), // empty paragraph
      p('line 2'),
      p('```')
    );

    const result = parseFenceFromJson(input);

    expect(result).toEqual({
      replacement: 'line 1\n\nline 2',
      startIndex: 0,
      endIndex: 5,
    });
  });
});

describe('transformReplaceFenceToPreview', () => {
  // Note: The new implementation requires ISOLATED fences:
  // - Opening ```replace must be alone in its paragraph
  // - Content lines are separate paragraphs
  // - Closing ``` must be alone in its paragraph

  it('transforms an isolated fence to ReplacePreview node', () => {
    // New format: each line is a separate paragraph
    const input = doc(
      p('```replace'),
      p('const x = 2;'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'const x = 1;', 'const x = 2;');

    expect(result).toMatchObject({
      content: [
        {
          type: 'replacePreview',
          attrs: {
            original: 'const x = 1;',
            replacement: 'const x = 2;',
          },
        },
      ],
    });
    // blockId should be generated
    expect(result.content?.[0]?.attrs?.blockId).toBeDefined();
  });

  it('preserves text before the fence', () => {
    const input = doc(
      p('Here is my fix:'),
      p('```replace'),
      p('const x = 2;'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'const x = 1;', 'const x = 2;');

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Here is my fix:' }] },
        { type: 'replacePreview' },
      ],
    });
  });

  it('handles multiline replacement content', () => {
    const original = 'function foo() {\n  return 1;\n}';
    const replacement = 'function foo() {\n  return 2;\n}';
    // Multi-line content = multiple paragraphs
    const input = doc(
      p('```replace'),
      p('function foo() {'),
      p('  return 2;'),
      p('}'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, original, replacement);

    expect(result).toMatchObject({
      content: [
        {
          type: 'replacePreview',
          attrs: { original, replacement },
        },
      ],
    });
  });

  it('only transforms the first valid fence', () => {
    const input = doc(
      p('```replace'),
      p('first'),
      p('```'),
      p('```replace'),
      p('second'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'original', 'first');

    expect(result).toMatchObject({
      content: [
        { type: 'replacePreview' },
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'second' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
      ],
    });
  });

  it('returns unchanged JSON when no fence is present', () => {
    const input = doc(p('Just some regular text'));

    const result = transformReplaceFenceToPreview(input, 'original', 'replacement');

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Just some regular text' }] },
      ],
    });
  });

  it('handles empty document', () => {
    const input: JSONContent = { type: 'doc' };

    const result = transformReplaceFenceToPreview(input, 'orig', 'repl');

    expect(result).toEqual({ type: 'doc' });
  });

  it('ignores non-isolated fence (mixed content in same paragraph)', () => {
    // Fence markers mixed with other content should NOT transform
    const input = doc(
      p(tagChip('1', 'FIX', 'Fix this'), ' some text'),
      p('```replace'),
      p('fixed'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'broken', 'fixed');

    // The fence should still transform because the markers are isolated
    expect(result.content?.some((n) => n.type === 'replacePreview')).toBe(true);
    // And the tagChip paragraph is preserved
    expect(result.content?.[0]?.type).toBe('paragraph');
  });

  it('ignores fence if opening marker has other content', () => {
    // Opening marker with extra text - should NOT transform
    const input = doc(p('prefix ```replace'), p('code'), p('```'));

    const result = transformReplaceFenceToPreview(input, 'orig', 'code');

    // Should return unchanged
    expect(result).toEqual(input);
  });

  it('handles multi-paragraph fence (each line is a separate paragraph)', () => {
    // When user types in TipTap with Enter, each line becomes a separate paragraph
    const input = doc(
      p('```replace'),
      p('const x = 2;'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'const x = 1;', 'const x = 2;');

    expect(result).toMatchObject({
      content: [
        {
          type: 'replacePreview',
          attrs: { original: 'const x = 1;', replacement: 'const x = 2;' },
        },
      ],
    });
  });

  it('handles multi-paragraph fence with content before and after', () => {
    const input = doc(
      p('Here is a fix:'),
      p('```replace'),
      p('new code'),
      p('```'),
      p('Please review.')
    );

    const result = transformReplaceFenceToPreview(input, 'old code', 'new code');

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Here is a fix:' }] },
        { type: 'replacePreview' },
        { type: 'paragraph', content: [{ type: 'text', text: 'Please review.' }] },
      ],
    });
  });

  it('handles multi-line replacement in multi-paragraph fence', () => {
    // The replacement value is passed in, not extracted from paragraphs
    const input = doc(
      p('```replace'),
      p('line 1'),
      p('line 2'),
      p('line 3'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'original', 'line 1\nline 2\nline 3');

    expect(result).toMatchObject({
      content: [
        {
          type: 'replacePreview',
          attrs: { replacement: 'line 1\nline 2\nline 3' },
        },
      ],
    });
  });

  // Edge cases

  it('handles fence marker with trailing space', () => {
    // User might accidentally type "```replace " with trailing space
    const input = doc(p('```replace \ncode\n```'));

    const result = transformReplaceFenceToPreview(input, 'orig', 'code');

    // Current implementation requires exact match, so this should NOT transform
    // This documents current behavior - adjust if we want to be more lenient
    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: '```replace \ncode\n```' }] },
      ],
    });
  });

  it('leaves document unchanged when closing fence is missing', () => {
    const input = doc(
      p('```replace'),
      p('code without closing fence'),
      p('more text')
    );

    const result = transformReplaceFenceToPreview(input, 'orig', 'code');

    // Without closing fence, document should remain unchanged
    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'code without closing fence' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'more text' }] },
      ],
    });
  });

  it('handles empty paragraphs between fence lines', () => {
    const input = doc(
      p('```replace'),
      p('line 1'),
      p(), // empty paragraph
      p('line 2'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'orig', 'line 1\n\nline 2');

    expect(result).toMatchObject({
      content: [
        {
          type: 'replacePreview',
          attrs: { replacement: 'line 1\n\nline 2' },
        },
      ],
    });
  });

  it('handles fence content with special characters', () => {
    const replacement = 'const regex = /```/g;';
    // Note: Content containing ``` must be carefully crafted
    // The closing ``` must be alone in its paragraph
    const input = doc(
      p('```replace'),
      p(replacement),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'orig', replacement);

    expect(result).toMatchObject({
      content: [
        {
          type: 'replacePreview',
          attrs: { replacement },
        },
      ],
    });
  });

  it('does not leave trailing empty paragraph after sealed node', () => {
    const input = doc(
      p('```replace'),
      p('content'),
      p('```')
    );

    const result = transformReplaceFenceToPreview(input, 'orig', 'content');

    // Should only have the replacePreview, no trailing empty paragraph
    expect(result.content).toHaveLength(1);
    expect(result.content?.[0].type).toBe('replacePreview');
  });
});

describe('trimContent with replacePreview', () => {
  it('removes trailing empty paragraph after replacePreview node', () => {
    // ProseMirror often adds a trailing paragraph for cursor positioning
    // trimContent should remove it
    const input = doc(
      replacePreview('orig', 'repl'),
      p() // empty trailing paragraph
    );

    const result = trimContent(input);

    expect(result.content).toHaveLength(1);
    expect(result.content?.[0].type).toBe('replacePreview');
  });
});

describe('transformReplacePreviewToFence', () => {
  // Note: Now outputs isolated paragraphs (one per line) to match insertion format

  it('transforms ReplacePreview node back to isolated paragraphs', () => {
    const input = doc(replacePreview('const x = 1;', 'const x = 2;'));

    const result = transformReplacePreviewToFence(input);

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'const x = 2;' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
      ],
    });
  });

  it('preserves multiline replacement as separate paragraphs', () => {
    const replacement = 'function foo() {\n  return 2;\n}';
    const input = doc(replacePreview('original code', replacement));

    const result = transformReplacePreviewToFence(input);

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'function foo() {' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '  return 2;' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '}' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
      ],
    });
  });

  it('preserves other nodes around ReplacePreview', () => {
    const input = doc(
      p('Before'),
      replacePreview('orig', 'repl'),
      p('After')
    );

    const result = transformReplacePreviewToFence(input);

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Before' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'repl' }] },
        { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'After' }] },
      ],
    });
  });

  it('handles empty replacement', () => {
    const input = doc(replacePreview('something', ''));

    const result = transformReplacePreviewToFence(input);

    expect(result).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph' }, // empty paragraph for empty line
        { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
      ],
    });
  });

  it('handles missing attrs gracefully', () => {
    const input: JSONContent = {
      type: 'doc',
      content: [{ type: 'replacePreview' }], // Missing attrs
    };

    const result = transformReplacePreviewToFence(input);

    // Should preserve the node as-is since no attrs
    expect(result).toMatchObject({
      content: [{ type: 'replacePreview' }],
    });
  });

  it('handles empty document', () => {
    const input: JSONContent = { type: 'doc' };

    const result = transformReplacePreviewToFence(input);

    expect(result).toEqual({ type: 'doc' });
  });

  it('is inverse of transformReplaceFenceToPreview (round-trip)', () => {
    const original = 'const x = 1;';
    const replacement = 'const x = 2;';
    // Use isolated paragraph format for input
    const fenceDoc = doc(
      p('```replace'),
      p(replacement),
      p('```')
    );

    // Fence -> Preview
    const previewDoc = transformReplaceFenceToPreview(fenceDoc, original, replacement);
    expect(previewDoc).toMatchObject({
      content: [{ type: 'replacePreview' }],
    });

    // Preview -> Fence (outputs isolated paragraphs)
    const backToFence = transformReplacePreviewToFence(previewDoc);
    expect(backToFence).toMatchObject({
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: '```replace' }] },
        { type: 'paragraph', content: [{ type: 'text', text: replacement }] },
        { type: 'paragraph', content: [{ type: 'text', text: '```' }] },
      ],
    });
  });
});

describe('ReplacePreview position verification', () => {
  let editor: Editor;
  let container: HTMLDivElement;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    editor?.destroy();
    container?.remove();
  });

  it('first block child in document has position 0', () => {
    // Verify that ProseMirror position 0 corresponds to the first block child
    editor = new Editor({
      element: container,
      extensions: [StarterKit, ReplacePreview],
      content: {
        type: 'doc',
        content: [
          {
            type: 'replacePreview',
            attrs: { blockId: 'test', original: 'old', replacement: 'new' },
          },
        ],
      },
    });

    // In ProseMirror, the first block child starts at position 0
    const firstNode = editor.state.doc.nodeAt(0);
    expect(firstNode?.type.name).toBe('replacePreview');
  });

  it('second block child has position > 0', () => {
    editor = new Editor({
      element: container,
      extensions: [StarterKit, ReplacePreview],
      content: {
        type: 'doc',
        content: [
          { type: 'paragraph', content: [{ type: 'text', text: 'hello' }] },
          {
            type: 'replacePreview',
            attrs: { blockId: 'test', original: 'old', replacement: 'new' },
          },
        ],
      },
    });

    // First node at position 0 should be paragraph
    const firstNode = editor.state.doc.nodeAt(0);
    expect(firstNode?.type.name).toBe('paragraph');

    // ReplacePreview should be at position > 0 (after paragraph)
    // Paragraph with "hello" = 1 (p open) + 5 (text) + 1 (p close) = 7
    // So replacePreview should be at position 7
    const secondNode = editor.state.doc.nodeAt(7);
    expect(secondNode?.type.name).toBe('replacePreview');
  });
});

describe('shouldChip', () => {
  it('returns false for empty text', () => {
    expect(shouldChip('')).toBe(false);
  });

  it('returns false for short single line', () => {
    expect(shouldChip('Hello world')).toBe(false);
  });

  it('returns false for short multi-line text (under 6 lines)', () => {
    const text = 'Line 1\nLine 2\nLine 3\nLine 4\nLine 5';
    expect(shouldChip(text)).toBe(false);
  });

  it('returns true for tall content (6+ lines)', () => {
    const text = 'Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6';
    expect(shouldChip(text)).toBe(true);
  });

  it('returns true for code block (10 lines)', () => {
    const code = Array(10).fill('const x = 1;').join('\n');
    expect(shouldChip(code)).toBe(true);
  });

  it('returns false for normal paragraph (3 lines, under 400 chars)', () => {
    const text = 'This is the first sentence of my paragraph.\nThis is the second line.\nAnd this is the third.';
    expect(shouldChip(text)).toBe(false);
  });

  it('returns true for horizontal sprawl (single line, 400+ chars)', () => {
    const longLine = 'x'.repeat(400);
    expect(shouldChip(longLine)).toBe(true);
  });

  it('returns true for horizontal sprawl (2 lines, 400+ chars)', () => {
    const text = 'x'.repeat(200) + '\n' + 'y'.repeat(201);
    expect(shouldChip(text)).toBe(true);
  });

  it('returns false for 3 lines with 400+ chars (not horizontal sprawl)', () => {
    // 3 lines means it's not a "massive single/double line" case
    const text = 'x'.repeat(150) + '\n' + 'y'.repeat(150) + '\n' + 'z'.repeat(150);
    expect(shouldChip(text)).toBe(false);
  });

  it('returns true for minified JSON (single long line)', () => {
    const json = '{"key":"value","nested":{"a":1,"b":2},"array":[1,2,3,4,5]}'.repeat(10);
    expect(json.length).toBeGreaterThan(400);
    expect(shouldChip(json)).toBe(true);
  });

  it('returns true for stack trace (20 lines)', () => {
    const stackTrace = Array(20).fill('at SomeClass.method (file.ts:123:45)').join('\n');
    expect(shouldChip(stackTrace)).toBe(true);
  });

  it('returns false for short URL', () => {
    expect(shouldChip('https://example.com/page')).toBe(false);
  });

  it('returns true for very long URL', () => {
    const longUrl = 'https://example.com/' + 'a'.repeat(400);
    expect(shouldChip(longUrl)).toBe(true);
  });
});

/** Create a pasteChip node */
const pasteChip = (content: string, lineCount: number): JSONContent => ({
  type: 'pasteChip',
  attrs: { content, lineCount },
});

describe('extractContentNodes with pasteChip', () => {
  it('extracts pasteChip as PasteNode', () => {
    const input = doc(p(pasteChip('Hello\nWorld', 2)));
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toEqual({ type: 'paste', content: 'Hello\nWorld' });
  });

  it('extracts pasteChip mixed with text', () => {
    const input = doc(p('See this: ', pasteChip('code here', 1), ' for details'));
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(3);
    expect(nodes[0]).toEqual({ type: 'text', text: 'See this: ' });
    expect(nodes[1]).toEqual({ type: 'paste', content: 'code here' });
    expect(nodes[2]).toEqual({ type: 'text', text: ' for details' });
  });

  it('extracts multiple pasteChips', () => {
    const input = doc(
      p(pasteChip('first paste', 1)),
      p(pasteChip('second paste', 2))
    );
    const nodes = extractContentNodes(input);
    expect(nodes).toHaveLength(2);
    expect(nodes[0]).toEqual({ type: 'paste', content: 'first paste' });
    expect(nodes[1]).toEqual({ type: 'paste', content: 'second paste' });
  });
});
