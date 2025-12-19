import { describe, it, expect } from 'vitest';
import { rangeToKey, keyToRange, isLineInRange } from './range';

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

  it('throws on zero or negative line numbers', () => {
    expect(() => rangeToKey({ start: 0, end: 10 })).toThrow('must be >= 1');
    expect(() => rangeToKey({ start: -1, end: 10 })).toThrow('must be >= 1');
    expect(() => rangeToKey({ start: 5, end: 0 })).toThrow('must be >= 1');
  });
});

describe('keyToRange', () => {
  it('parses string key to range', () => {
    expect(keyToRange('5-10')).toEqual({ start: 5, end: 10 });
  });

  it('handles single-line key', () => {
    expect(keyToRange('7-7')).toEqual({ start: 7, end: 7 });
  });

  it('throws on invalid format', () => {
    expect(() => keyToRange('invalid')).toThrow('expected "start-end" format');
    expect(() => keyToRange('5')).toThrow('expected "start-end" format');
    expect(() => keyToRange('5-10-15')).toThrow('expected "start-end" format');
    expect(() => keyToRange('-5-10')).toThrow('expected "start-end" format');
    expect(() => keyToRange('abc-def')).toThrow('expected "start-end" format');
  });

  it('throws on zero line numbers', () => {
    expect(() => keyToRange('0-10')).toThrow('must be >= 1');
    expect(() => keyToRange('5-0')).toThrow('must be >= 1');
  });

  it('throws when start > end (unnormalized)', () => {
    expect(() => keyToRange('10-5')).toThrow('start must be <= end');
  });
});

describe('isLineInRange', () => {
  it('returns true for line within range', () => {
    expect(isLineInRange(7, { start: 5, end: 10 })).toBe(true);
  });

  it('returns true for line at start of range', () => {
    expect(isLineInRange(5, { start: 5, end: 10 })).toBe(true);
  });

  it('returns true for line at end of range', () => {
    expect(isLineInRange(10, { start: 5, end: 10 })).toBe(true);
  });

  it('returns false for line outside range', () => {
    expect(isLineInRange(3, { start: 5, end: 10 })).toBe(false);
    expect(isLineInRange(12, { start: 5, end: 10 })).toBe(false);
  });

  it('handles reversed ranges', () => {
    expect(isLineInRange(7, { start: 10, end: 5 })).toBe(true);
  });
});
