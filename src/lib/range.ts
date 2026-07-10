/**
 * A range of display indices (1-indexed positions in the lines array).
 * Ephemeral selection state only — annotation identity is an id and its
 * position an Anchor in source coordinates (see anchor.ts).
 */
export type Range = {
  start: number;  // Display index (1-indexed)
  end: number;    // Display index (1-indexed)
};

/** Check if a display index is within a range */
export function isLineInRange(displayIdx: number, range: Range): boolean {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);
  return displayIdx >= min && displayIdx <= max;
}
