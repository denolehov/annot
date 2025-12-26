import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushSync } from 'svelte';

// Mock @tauri-apps/api/core before importing the composable
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { useAnnotations } from './useAnnotations.svelte';
import { invoke } from '@tauri-apps/api/core';
import type { Line } from '$lib/types';

/**
 * Create mock lines for testing. Each line has source origin with the given path.
 * Line numbers match the 1-indexed position in the array.
 */
function createMockLines(count: number, path = '/test/file.ts'): Line[] {
  return Array.from({ length: count }, (_, i) => ({
    content: `line ${i + 1}`,
    html: null,
    origin: { type: 'source' as const, path, line: i + 1 },
    semantics: { type: 'plain' as const },
  }));
}

describe('useAnnotations', () => {
  const mockLines = createMockLines(30);
  const getLines = () => mockLines;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts with empty annotations', () => {
    const state = useAnnotations({ getLines });
    expect(state.annotations).toEqual({});
    expect(state.allRanges()).toEqual([]);
  });

  it('upserts an annotation and syncs to backend', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);

    expect(state.annotations['5-10']).toBeDefined();
    expect(state.annotations['5-10'].content).toEqual(content);
    expect(invoke).toHaveBeenCalledWith('upsert_annotation', {
      path: '/test/file.ts',
      startLine: 5,
      endLine: 10,
      content: expect.any(Array),
    });
  });

  it('deletes annotation when content is null', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    // First add an annotation
    await state.upsert({ start: 5, end: 10 }, content);
    expect(state.annotations['5-10']).toBeDefined();

    // Then remove it
    await state.upsert({ start: 5, end: 10 }, null);

    expect(state.annotations['5-10']).toBeUndefined();
    expect(invoke).toHaveBeenCalledWith('delete_annotation', {
      path: '/test/file.ts',
      startLine: 5,
      endLine: 10,
    });
  });

  it('deletes annotation when content is empty', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };
    const emptyContent = { type: 'doc', content: [{ type: 'paragraph' }] };

    await state.upsert({ start: 5, end: 10 }, content);
    expect(state.annotations['5-10']).toBeDefined();

    await state.upsert({ start: 5, end: 10 }, emptyContent);

    expect(state.annotations['5-10']).toBeUndefined();
  });

  it('gets annotation by range', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);

    const result = state.get({ start: 5, end: 10 });
    expect(result).toEqual(content);

    const missing = state.get({ start: 1, end: 2 });
    expect(missing).toBeUndefined();
  });

  it('gets annotation by key', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);

    const entry = state.getByKey('5-10');
    expect(entry?.content).toEqual(content);

    const missing = state.getByKey('1-2');
    expect(missing).toBeUndefined();
  });

  it('seals and unseals annotations', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);
    expect(state.isSealed('5-10')).toBe(false);

    flushSync(() => {
      state.seal('5-10');
    });
    expect(state.isSealed('5-10')).toBe(true);

    flushSync(() => {
      state.unseal('5-10');
    });
    expect(state.isSealed('5-10')).toBe(false);
  });

  it('gets annotation at line (by end line)', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);

    // Annotation ends at line 10
    const result = state.getAtLine(10);
    expect(result?.key).toBe('5-10');
    expect(result?.content).toEqual(content);

    // Line 5 is not the end line
    const notEnd = state.getAtLine(5);
    expect(notEnd).toBeNull();

    // Line 15 has no annotation
    const missing = state.getAtLine(15);
    expect(missing).toBeNull();
  });

  it('checks if line has annotation', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);

    expect(state.hasAnnotation(5)).toBe(true);
    expect(state.hasAnnotation(7)).toBe(true);
    expect(state.hasAnnotation(10)).toBe(true);
    expect(state.hasAnnotation(4)).toBe(false);
    expect(state.hasAnnotation(11)).toBe(false);
  });

  it('returns all ranges', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);
    await state.upsert({ start: 20, end: 25 }, content);

    const ranges = state.allRanges();
    expect(ranges.length).toBe(2);
    expect(ranges).toContainEqual({ key: '5-10', start: 5, end: 10 });
    expect(ranges).toContainEqual({ key: '20-25', start: 20, end: 25 });
  });

  it('removes annotation by key', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    await state.upsert({ start: 5, end: 10 }, content);
    expect(state.annotations['5-10']).toBeDefined();

    flushSync(() => {
      state.remove('5-10');
    });

    expect(state.annotations['5-10']).toBeUndefined();
  });

  it('rejects annotation on invalid range (missing lines)', async () => {
    const state = useAnnotations({ getLines });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    // Range extends beyond available lines
    await state.upsert({ start: 25, end: 35 }, content);

    // Should not create annotation
    expect(state.annotations['25-35']).toBeUndefined();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('rejects annotation on virtual lines', async () => {
    const linesWithVirtual: Line[] = [
      ...createMockLines(5),
      { content: 'virtual', html: null, origin: { type: 'virtual' }, semantics: { type: 'plain' } },
      ...createMockLines(5, '/test/file.ts'),
    ];
    const state = useAnnotations({ getLines: () => linesWithVirtual });
    const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

    // Range includes virtual line
    await state.upsert({ start: 5, end: 7 }, content);

    // Should not create annotation
    expect(state.annotations['5-7']).toBeUndefined();
    expect(invoke).not.toHaveBeenCalled();
  });
});
