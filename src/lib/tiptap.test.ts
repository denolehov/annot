import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { trimContent, isContentEmpty, EditorShortcuts, extractContentNodes } from './tiptap';
import { Editor, type JSONContent } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';

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
