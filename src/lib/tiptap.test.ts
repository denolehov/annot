import { describe, it, expect } from 'vitest';
import { trimContent, isContentEmpty } from './tiptap';
import type { JSONContent } from '@tiptap/core';

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
