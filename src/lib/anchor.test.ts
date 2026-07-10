import { describe, it, expect } from 'vitest';
import { selectionToAnchor, endpointKeys, anchorKeys, anchorLines, anchorLabel, type Anchor } from './anchor';
import type { Line } from './types';

// Helper to create mock lines
function makeLine(origin: Line['origin']): Line {
  return {
    content: 'test',
    html: null,
    origin,
    semantics: { type: 'plain' },
  };
}

describe('selectionToAnchor', () => {
  it('builds a source anchor from file mode lines', () => {
    const lines: Line[] = [
      makeLine({ type: 'source', path: 'test.rs', line: 10 }),
      makeLine({ type: 'source', path: 'test.rs', line: 11 }),
      makeLine({ type: 'source', path: 'test.rs', line: 12 }),
    ];

    const anchor = selectionToAnchor({ start: 1, end: 3 }, lines);
    expect(anchor).toEqual({
      type: 'source',
      path: 'test.rs',
      start: 10,
      end: 12,
    });
  });

  it('builds a diff anchor from diff mode lines', () => {
    const lines: Line[] = [
      makeLine({ type: 'diff', path: 'file.rs', old_line: null, new_line: 5 }),
      makeLine({ type: 'diff', path: 'file.rs', old_line: null, new_line: 6 }),
      makeLine({ type: 'diff', path: 'file.rs', old_line: null, new_line: 7 }),
    ];

    const anchor = selectionToAnchor({ start: 1, end: 3 }, lines);
    expect(anchor).toEqual({
      type: 'diff',
      path: 'file.rs',
      start: { side: 'new', line: 5 },
      end: { side: 'new', line: 7 },
    });
  });

  it('returns null for virtual lines', () => {
    const lines: Line[] = [
      makeLine({ type: 'virtual' }),
      makeLine({ type: 'source', path: 'test.rs', line: 10 }),
    ];

    // Range starts at virtual line
    const anchor = selectionToAnchor({ start: 1, end: 2 }, lines);
    expect(anchor).toBeNull();
  });

  it('returns null for out of bounds range', () => {
    const lines: Line[] = [
      makeLine({ type: 'source', path: 'test.rs', line: 10 }),
    ];

    const anchor = selectionToAnchor({ start: 1, end: 5 }, lines);
    expect(anchor).toBeNull();
  });

  it('returns null when lines have different paths', () => {
    const lines: Line[] = [
      makeLine({ type: 'source', path: 'file1.rs', line: 10 }),
      makeLine({ type: 'source', path: 'file2.rs', line: 11 }),
    ];

    const anchor = selectionToAnchor({ start: 1, end: 2 }, lines);
    expect(anchor).toBeNull();
  });

  it('accepts diff ranges mixing removed and added lines', () => {
    // Real hunk shape: removed lines carry old-file numbers, added/context
    // lines carry new-file numbers, so consecutive rows "jump" numerically.
    const lines: Line[] = [
      makeLine({ type: 'diff', path: 'output.rs', old_line: 122, new_line: 124 }), // context
      makeLine({ type: 'diff', path: 'output.rs', old_line: 123, new_line: null }), // removed
      makeLine({ type: 'diff', path: 'output.rs', old_line: null, new_line: 125 }), // added
      makeLine({ type: 'diff', path: 'output.rs', old_line: null, new_line: 126 }), // added
    ];

    const anchor = selectionToAnchor({ start: 1, end: 4 }, lines);
    expect(anchor).toEqual({
      type: 'diff',
      path: 'output.rs',
      start: { side: 'new', line: 124 },
      end: { side: 'new', line: 126 },
    });
  });

  it('marks a removed-only line as old side', () => {
    const lines: Line[] = [
      makeLine({ type: 'diff', path: 'output.rs', old_line: 122, new_line: null }), // removed
      makeLine({ type: 'diff', path: 'output.rs', old_line: 123, new_line: null }), // removed
    ];

    const anchor = selectionToAnchor({ start: 1, end: 2 }, lines);
    expect(anchor).toEqual({
      type: 'diff',
      path: 'output.rs',
      start: { side: 'old', line: 122 },
      end: { side: 'old', line: 123 },
    });
  });

  it('builds a mixed-side anchor over a replacement span', () => {
    // A deletion followed by its added replacement: the anchor endpoints
    // land on different sides. Endpoints swap as a (line, side) unit when
    // display order reverses source order.
    const lines: Line[] = [
      makeLine({ type: 'diff', path: 'file.rs', old_line: 2, new_line: null }), // removed
      makeLine({ type: 'diff', path: 'file.rs', old_line: 3, new_line: null }), // removed
      makeLine({ type: 'diff', path: 'file.rs', old_line: null, new_line: 2 }), // added
      makeLine({ type: 'diff', path: 'file.rs', old_line: null, new_line: 3 }), // added
    ];

    const anchor = selectionToAnchor({ start: 1, end: 4 }, lines);
    expect(anchor).toEqual({
      type: 'diff',
      path: 'file.rs',
      start: { side: 'old', line: 2 },
      end: { side: 'new', line: 3 },
    });
  });

  it('returns null when line numbers have gap > 1', () => {
    const lines: Line[] = [
      makeLine({ type: 'source', path: 'test.rs', line: 10 }),
      makeLine({ type: 'source', path: 'test.rs', line: 15 }), // gap of 5
    ];

    const anchor = selectionToAnchor({ start: 1, end: 2 }, lines);
    expect(anchor).toBeNull();
  });

  it('normalizes source line order', () => {
    // Lines might be in display order but source lines could be reversed
    // (though unusual, the function handles it)
    const lines: Line[] = [
      makeLine({ type: 'source', path: 'test.rs', line: 15 }),
      makeLine({ type: 'source', path: 'test.rs', line: 14 }),
    ];

    const anchor = selectionToAnchor({ start: 1, end: 2 }, lines);
    expect(anchor).toEqual({
      type: 'source',
      path: 'test.rs',
      start: 14,
      end: 15,
    });
  });
});

describe('endpointKeys', () => {
  it('registers one side-less key for source lines', () => {
    const keys = endpointKeys(makeLine({ type: 'source', path: 'a.rs', line: 7 }));
    expect(keys).toHaveLength(1);
  });

  it('registers both sides for diff context lines', () => {
    const keys = endpointKeys(makeLine({ type: 'diff', path: 'a.rs', old_line: 3, new_line: 5 }));
    expect(keys).toHaveLength(2);
  });

  it('registers a single side for added and deleted lines', () => {
    expect(endpointKeys(makeLine({ type: 'diff', path: 'a.rs', old_line: null, new_line: 5 }))).toHaveLength(1);
    expect(endpointKeys(makeLine({ type: 'diff', path: 'a.rs', old_line: 3, new_line: null }))).toHaveLength(1);
  });

  it('registers nothing for virtual lines', () => {
    expect(endpointKeys(makeLine({ type: 'virtual' }))).toEqual([]);
  });

  it('resolves anchors against the keys lines register', () => {
    const contextLine = makeLine({ type: 'diff', path: 'a.rs', old_line: 3, new_line: 5 });
    const oldSide: Anchor = { type: 'diff', path: 'a.rs', start: { side: 'old', line: 3 }, end: { side: 'old', line: 3 } };
    const newSide: Anchor = { type: 'diff', path: 'a.rs', start: { side: 'new', line: 5 }, end: { side: 'new', line: 5 } };
    const keys = endpointKeys(contextLine);
    expect(keys).toContain(anchorKeys(oldSide)[0]);
    expect(keys).toContain(anchorKeys(newSide)[0]);

    const sourceLine = makeLine({ type: 'source', path: 'a.rs', line: 7 });
    const source: Anchor = { type: 'source', path: 'a.rs', start: 7, end: 7 };
    expect(endpointKeys(sourceLine)).toContain(anchorKeys(source)[0]);
  });
});

describe('anchorLines / anchorLabel', () => {
  it('extracts lines from both variants', () => {
    expect(anchorLines({ type: 'source', path: 'a', start: 3, end: 7 })).toEqual({ start: 3, end: 7 });
    expect(
      anchorLines({ type: 'diff', path: 'a', start: { side: 'old', line: 2 }, end: { side: 'new', line: 4 } })
    ).toEqual({ start: 2, end: 4 });
  });

  it('labels single-line and multi-line anchors', () => {
    expect(anchorLabel({ type: 'source', path: 'a', start: 5, end: 5 })).toBe('5');
    expect(anchorLabel({ type: 'source', path: 'a', start: 5, end: 9 })).toBe('5-9');
  });
});
