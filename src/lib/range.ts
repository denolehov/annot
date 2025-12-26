import type { Line } from './types';
import { getLineNumber, getFileIndex, getFilePath } from './line-utils';

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
 * Extract source coordinates from a display index range.
 * Returns fileIndex, filePath, and source line numbers for backend API calls.
 * - fileIndex is used for diff mode (identifies which file in the diff)
 * - filePath is used for portal mode (identifies the external source file)
 * Returns null if the range spans virtual lines without source coordinates.
 */
export function rangeToSourceCoords(
  range: Range,
  lines: Line[]
): { fileIndex: number | null; filePath: string | null; startLine: number; endLine: number } | null {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);

  const startLine = lines[min - 1];
  const endLine = lines[max - 1];
  if (!startLine || !endLine) return null;

  const fileIndex = getFileIndex(startLine);
  const filePath = getFilePath(startLine);
  const startSource = getLineNumber(startLine);
  const endSource = getLineNumber(endLine);

  if (startSource === null || endSource === null) return null;

  return {
    fileIndex,
    filePath,
    startLine: Math.min(startSource, endSource),
    endLine: Math.max(startSource, endSource),
  };
}
