import { describe, it, expect } from 'vitest';
import { rangeToKey, keyToRange, isLineInRange, rangeToSourceCoords } from './range';
import type { Line } from './types';

describe('rangeToKey', () => {
  it('converts range to string key', () => {
    expect(rangeToKey({ start: 5, end: 10 })).toBe('5-10');
  });

  it('normalizes reversed ranges', () => {
    expect(rangeToKey({ start: 10, end: 5 })).toBe('5-10');
  });

  it('handles single-line range', () => {
    expect(rangeToKey({ start: 7, end: 7 })).toBe('7-7');
  });

  it('throws on non-integer values', () => {
    expect(() => rangeToKey({ start: 5.5, end: 10 })).toThrow('must be integers');
    expect(() => rangeToKey({ start: 5, end: 10.5 })).toThrow('must be integers');
  });

  it('throws on zero or negative display indices', () => {
    expect(() => rangeToKey({ start: 0, end: 10 })).toThrow('must be >= 1');
    expect(() => rangeToKey({ start: -1, end: 10 })).toThrow('must be >= 1');
    expect(() => rangeToKey({ start: 5, end: 0 })).toThrow('must be >= 1');
  });
});

describe('keyToRange', () => {
  it('parses key to range', () => {
    expect(keyToRange('5-10')).toEqual({ start: 5, end: 10 });
  });

  it('roundtrips correctly', () => {
    const range = { start: 5, end: 10 };
    expect(keyToRange(rangeToKey(range))).toEqual(range);
  });

  it('handles single-line key', () => {
    expect(keyToRange('7-7')).toEqual({ start: 7, end: 7 });
  });

  it('throws on invalid format', () => {
    expect(() => keyToRange('invalid')).toThrow();
    expect(() => keyToRange('5')).toThrow();
    expect(() => keyToRange('abc-def')).toThrow();
    // Old fileIndex format no longer supported
    expect(() => keyToRange('0:5-10')).toThrow();
  });

  it('throws on zero display indices', () => {
    expect(() => keyToRange('0-10')).toThrow('must be >= 1');
    expect(() => keyToRange('5-0')).toThrow('must be >= 1');
  });

  it('throws when start > end (unnormalized)', () => {
    expect(() => keyToRange('10-5')).toThrow('start must be <= end');
  });
});

describe('isLineInRange', () => {
  it('returns true for display index within range', () => {
    expect(isLineInRange(7, { start: 5, end: 10 })).toBe(true);
  });

  it('returns true for display index at start of range', () => {
    expect(isLineInRange(5, { start: 5, end: 10 })).toBe(true);
  });

  it('returns true for display index at end of range', () => {
    expect(isLineInRange(10, { start: 5, end: 10 })).toBe(true);
  });

  it('returns false for display index outside range', () => {
    expect(isLineInRange(3, { start: 5, end: 10 })).toBe(false);
    expect(isLineInRange(12, { start: 5, end: 10 })).toBe(false);
  });

  it('handles reversed ranges', () => {
    expect(isLineInRange(7, { start: 10, end: 5 })).toBe(true);
  });
});

describe('rangeToSourceCoords', () => {
  // Helper to create mock lines
  function makeLine(origin: Line['origin']): Line {
    return {
      content: 'test',
      html: null,
      origin,
      semantics: { type: 'plain' },
    };
  }

  it('extracts source coordinates from file mode lines', () => {
    const lines: Line[] = [
      makeLine({ type: 'document', line: 10 }),
      makeLine({ type: 'document', line: 11 }),
      makeLine({ type: 'document', line: 12 }),
    ];

    const coords = rangeToSourceCoords({ start: 1, end: 3 }, lines);
    expect(coords).toEqual({
      fileIndex: null,
      startLine: 10,
      endLine: 12,
    });
  });

  it('extracts source coordinates from diff mode lines', () => {
    const lines: Line[] = [
      makeLine({ type: 'diff', old_line: null, new_line: 5, file_index: 0 }),
      makeLine({ type: 'diff', old_line: null, new_line: 6, file_index: 0 }),
      makeLine({ type: 'diff', old_line: null, new_line: 7, file_index: 0 }),
    ];

    const coords = rangeToSourceCoords({ start: 1, end: 3 }, lines);
    expect(coords).toEqual({
      fileIndex: 0,
      startLine: 5,
      endLine: 7,
    });
  });

  it('returns null for virtual lines', () => {
    const lines: Line[] = [
      makeLine({ type: 'virtual' }),
      makeLine({ type: 'document', line: 10 }),
    ];

    // Range starts at virtual line
    const coords = rangeToSourceCoords({ start: 1, end: 2 }, lines);
    expect(coords).toBeNull();
  });

  it('returns null for out of bounds range', () => {
    const lines: Line[] = [
      makeLine({ type: 'document', line: 10 }),
    ];

    const coords = rangeToSourceCoords({ start: 1, end: 5 }, lines);
    expect(coords).toBeNull();
  });

  it('normalizes source line order', () => {
    // Lines might be in display order but source lines could be reversed
    // (though unusual, the function handles it)
    const lines: Line[] = [
      makeLine({ type: 'document', line: 15 }),
      makeLine({ type: 'document', line: 10 }),
    ];

    const coords = rangeToSourceCoords({ start: 1, end: 2 }, lines);
    expect(coords).toEqual({
      fileIndex: null,
      startLine: 10,
      endLine: 15,
    });
  });
});
