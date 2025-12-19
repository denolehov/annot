export type Range = { start: number; end: number };

/** Convert a range to a normalized string key (min-max) */
export function rangeToKey(range: Range): string {
  if (!Number.isInteger(range.start) || !Number.isInteger(range.end)) {
    throw new Error(`Invalid range: start and end must be integers, got ${range.start}-${range.end}`);
  }
  if (range.start < 1 || range.end < 1) {
    throw new Error(`Invalid range: line numbers must be >= 1, got ${range.start}-${range.end}`);
  }
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);
  return `${min}-${max}`;
}

/** Parse a string key back to a range */
export function keyToRange(key: string): Range {
  const match = key.match(/^(\d+)-(\d+)$/);
  if (!match) {
    throw new Error(`Invalid range key: expected "start-end" format, got "${key}"`);
  }
  const start = parseInt(match[1], 10);
  const end = parseInt(match[2], 10);
  if (start < 1 || end < 1) {
    throw new Error(`Invalid range key: line numbers must be >= 1, got "${key}"`);
  }
  if (start > end) {
    throw new Error(`Invalid range key: start must be <= end, got "${key}"`);
  }
  return { start, end };
}

/** Check if a line number is within a range */
export function isLineInRange(lineNum: number, range: Range): boolean {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);
  return lineNum >= min && lineNum <= max;
}
