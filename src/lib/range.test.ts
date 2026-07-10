import { describe, it, expect } from 'vitest';
import { isLineInRange } from './range';

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
