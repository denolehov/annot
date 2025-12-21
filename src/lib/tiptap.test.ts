import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { trimContent, isContentEmpty, EditorShortcuts } from './tiptap';
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
