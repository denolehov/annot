import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushSync } from 'svelte';

// Mock @tauri-apps/api/core before importing the composable
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { useAnnotations, type AnnotationEntry } from './useAnnotations.svelte';
import { invoke } from '@tauri-apps/api/core';
import { deriveDisplay } from '$lib/display-rows';
import type { Anchor } from '$lib/anchor';
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

/** Source anchor into the mock lines (line numbers == display rows there). */
function anchor(start: number, end: number, path = '/test/file.ts'): Anchor {
  return { type: 'source', path, start, end };
}

const CONTENT = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Test' }] }] };

describe('useAnnotations', () => {
  const mockLines = createMockLines(30);
  const getLines = () => mockLines;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts with empty annotations', () => {
    const state = useAnnotations({ getLines });
    expect(state.annotations).toEqual({});
    expect(state.atEndRow(10)).toBeNull();
  });

  it('upserts an annotation and syncs to backend', async () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);

    // Local state updates synchronously; backend sync is debounced until flush.
    expect(state.annotations['a1']).toBeDefined();
    expect(state.annotations['a1'].content).toEqual(CONTENT);

    await state.flush();
    expect(invoke).toHaveBeenCalledWith('upsert_annotation', {
      id: 'a1',
      path: '/test/file.ts',
      anchor: { type: 'source', path: '/test/file.ts', start: 5, end: 10 },
      content: expect.any(Array),
    });
  });

  it('debounces backend sync and coalesces repeated edits', async () => {
    const state = useAnnotations({ getLines });

    // Rapid edits to the same annotation (the per-keystroke case) update local
    // state immediately but must not each fire an IPC.
    state.upsert('a1', anchor(5, 10), CONTENT);
    state.upsert('a1', anchor(5, 10), CONTENT);
    state.upsert('a1', anchor(5, 10), CONTENT);
    expect(invoke).not.toHaveBeenCalled();

    // Flush sends a single coalesced upsert for the three edits.
    await state.flush();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('deletes annotation when content is null', async () => {
    const state = useAnnotations({ getLines });

    // First add an annotation and sync it
    state.upsert('a1', anchor(5, 10), CONTENT);
    await state.flush();
    expect(state.annotations['a1']).toBeDefined();

    // Then remove it
    state.upsert('a1', anchor(5, 10), null);
    expect(state.annotations['a1']).toBeUndefined();

    await state.flush();
    expect(invoke).toHaveBeenCalledWith('delete_annotation', {
      path: '/test/file.ts',
      id: 'a1',
    });
  });

  it('cancels a never-flushed upsert instead of deleting on the backend', async () => {
    const state = useAnnotations({ getLines });

    // Create locally, then empty it before any flush — but the local entry
    // exists, so a delete is enqueued... unless the entry never existed:
    state.upsert('a1', anchor(5, 10), null);
    await state.flush();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('deletes annotation when content is empty', () => {
    const state = useAnnotations({ getLines });
    const emptyContent = { type: 'doc', content: [{ type: 'paragraph' }] };

    state.upsert('a1', anchor(5, 10), CONTENT);
    expect(state.annotations['a1']).toBeDefined();

    state.upsert('a1', anchor(5, 10), emptyContent);
    expect(state.annotations['a1']).toBeUndefined();
  });

  it('typing does not change the store key set', () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);
    const before = Object.keys(state.annotations);

    const edited = { ...CONTENT, content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Edited' }] }] };
    state.upsert('a1', anchor(5, 10), edited);

    expect(Object.keys(state.annotations)).toEqual(before);
    expect(state.annotations['a1'].content).toEqual(edited);
  });

  it('resolves entries at their end row', () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);

    // Annotation's resolved span ends at row 10
    expect(state.atEndRow(10)?.id).toBe('a1');
    // Row 5 is not the end row
    expect(state.atEndRow(5)).toBeNull();
    // Row 15 has no annotation
    expect(state.atEndRow(15)).toBeNull();
  });

  it('checks if a row is covered by an annotation', () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);

    expect(state.hasAnnotation(5)).toBe(true);
    expect(state.hasAnnotation(7)).toBe(true);
    expect(state.hasAnnotation(10)).toBe(true);
    expect(state.hasAnnotation(4)).toBe(false);
    expect(state.hasAnnotation(11)).toBe(false);
  });

  it('finds an entry by exact span', () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);

    expect(state.atSpan({ start: 5, end: 10 })?.id).toBe('a1');
    expect(state.atSpan({ start: 10, end: 5 })?.id).toBe('a1'); // normalized
    expect(state.atSpan({ start: 5, end: 9 })).toBeNull();
  });

  it('resolves spans of ids and raw anchors', () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);

    expect(state.spanOf('a1')).toEqual({ start: 5, end: 10 });
    expect(state.spanOf('missing')).toBeNull();
    expect(state.spanOfAnchor(anchor(20, 25))).toEqual({ start: 20, end: 25 });
    expect(state.spanOfAnchor(anchor(20, 99))).toBeNull(); // beyond the lines
  });

  it('resolves diff anchors side-aware through the walk, including mixed-side spans', () => {
    // Replacement hunk: header 1, hunk 2, rows 3–8
    // (context, two removed, two added, context).
    const rows = [
      { old_line: 1, new_line: 1 },
      { old_line: 2, new_line: null },
      { old_line: 3, new_line: null },
      { old_line: null, new_line: 2 },
      { old_line: null, new_line: 3 },
      { old_line: 4, new_line: 4 },
    ].map((sides, i) => ({ ...sides, content: ` row ${i + 1}`, html: null }));
    const display = deriveDisplay([{
      path: 'file.rs',
      old_path: null,
      status: 'modified',
      unavailable: false,
      language: 'rs',
      hunks: [{
        old_range: { start: 1, end: 5 },
        new_range: { start: 1, end: 5 },
        function_context: null,
        function_context_html: null,
        rows,
      }],
    }]);
    const state = useAnnotations({ getLines: () => [], getDisplay: () => display });

    // Mixed-side anchor: old:2 (row 4) → new:3 (row 7)
    const mixed: Anchor = {
      type: 'diff',
      path: 'file.rs',
      start: { side: 'old', line: 2 },
      end: { side: 'new', line: 3 },
    };
    state.upsert('m1', mixed, CONTENT);

    expect(state.spanOf('m1')).toEqual({ start: 4, end: 7 });
    expect(state.atEndRow(7)?.id).toBe('m1');
    expect(state.hasAnnotation(5)).toBe(true);
    expect(state.hasAnnotation(3)).toBe(false);

    // Context rows answer on both sides: old:4 and new:4 both hit row 8.
    expect(
      state.spanOfAnchor({ type: 'diff', path: 'file.rs', start: { side: 'old', line: 4 }, end: { side: 'new', line: 4 } })
    ).toEqual({ start: 8, end: 8 });
  });

  it('removes annotation locally by id', () => {
    const state = useAnnotations({ getLines });

    state.upsert('a1', anchor(5, 10), CONTENT);
    expect(state.annotations['a1']).toBeDefined();

    flushSync(() => {
      state.remove('a1');
    });

    expect(state.annotations['a1']).toBeUndefined();
  });

  it('restore diffs by id and syncs deletions and upserts', async () => {
    const state = useAnnotations({ getLines });

    state.upsert('keep', anchor(1, 2), CONTENT);
    state.upsert('gone', anchor(5, 6), CONTENT);
    await state.flush();
    vi.clearAllMocks();

    const snapshot: Record<string, AnnotationEntry> = {
      keep: { id: 'keep', anchor: anchor(1, 2), content: CONTENT },
      fresh: { id: 'fresh', anchor: anchor(9, 9), content: CONTENT },
    };
    state.restore(snapshot);

    expect(Object.keys(state.annotations).sort()).toEqual(['fresh', 'keep']);

    await state.flush();
    const calls = (invoke as ReturnType<typeof vi.fn>).mock.calls;
    expect(calls).toContainEqual(['delete_annotation', { path: '/test/file.ts', id: 'gone' }]);
    expect(calls.filter(([cmd]) => cmd === 'upsert_annotation').map(([, args]) => args.id).sort())
      .toEqual(['fresh', 'keep']);
  });

  it('restore deep-clones the snapshot', () => {
    const state = useAnnotations({ getLines });

    const entry: AnnotationEntry = { id: 'a1', anchor: anchor(5, 10), content: JSON.parse(JSON.stringify(CONTENT)) };
    state.restore({ a1: entry });

    (entry.anchor as { start: number }).start = 999;
    expect((state.annotations['a1'].anchor as { start: number }).start).toBe(5);
  });
});
