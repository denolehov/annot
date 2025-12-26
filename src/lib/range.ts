import type { Line } from './types';
import { getLineNumber, getFilePath } from './line-utils';

/**
 * A range of display indices (1-indexed positions in the lines array).
 * Display indices are inherently unique across all files/content.
 */
export type Range = {
  start: number;  // Display index (1-indexed)
  end: number;    // Display index (1-indexed)
};

/**
 * Convert a range to a normalized string key.
 * Format: "start-end" where start <= end.
 */
export function rangeToKey(range: Range): string {
  if (!Number.isInteger(range.start) || !Number.isInteger(range.end)) {
    throw new Error(`Invalid range: start and end must be integers, got ${range.start}-${range.end}`);
  }
  if (range.start < 1 || range.end < 1) {
    throw new Error(`Invalid range: display indices must be >= 1, got ${range.start}-${range.end}`);
  }
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);
  return `${min}-${max}`;
}

/**
 * Parse a string key back to a range.
 * Format: "start-end"
 */
export function keyToRange(key: string): Range {
  const match = key.match(/^(\d+)-(\d+)$/);
  if (!match) {
    throw new Error(`Invalid range key: expected "start-end" format, got "${key}"`);
  }
  const start = parseInt(match[1], 10);
  const end = parseInt(match[2], 10);
  if (start < 1 || end < 1) {
    throw new Error(`Invalid range key: display indices must be >= 1, got "${key}"`);
  }
  if (start > end) {
    throw new Error(`Invalid range key: start must be <= end, got "${key}"`);
  }
  return { start, end };
}

/** Check if a display index is within a range */
export function isLineInRange(displayIdx: number, range: Range): boolean {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);
  return displayIdx >= min && displayIdx <= max;
}

/**
 * Validate a range and extract source coordinates for backend API calls.
 * Validates:
 * 1. All lines in range have non-virtual origin
 * 2. All lines share the same origin.path
 * 3. No line number discontinuities (for portal boundary detection)
 *
 * Returns null if validation fails.
 */
export function validateRange(
  range: Range,
  lines: Line[]
): { path: string; startLine: number; endLine: number } | null {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);

  const startLine = lines[min - 1];
  const endLine = lines[max - 1];
  if (!startLine || !endLine) return null;

  // Get path from start line - must be non-virtual
  const path = getFilePath(startLine);
  if (path === null) return null;

  // Check all lines in range share the same path and have no gaps
  let prevLineNum: number | null = null;
  for (let i = min - 1; i < max; i++) {
    const line = lines[i];
    const linePath = getFilePath(line);
    const lineNum = getLineNumber(line);

    // All lines must have same path
    if (linePath !== path) return null;

    // All lines must have line numbers (non-virtual)
    if (lineNum === null) return null;

    // Check for line number discontinuity (gap > 1 indicates portal boundary)
    if (prevLineNum !== null && Math.abs(lineNum - prevLineNum) > 1) {
      return null;
    }
    prevLineNum = lineNum;
  }

  const startSource = getLineNumber(startLine)!;
  const endSource = getLineNumber(endLine)!;

  return {
    path,
    startLine: Math.min(startSource, endSource),
    endLine: Math.max(startSource, endSource),
  };
}
