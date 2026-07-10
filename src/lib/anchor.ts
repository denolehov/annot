import type { Line } from './types';
import type { Range } from './range';
import { getLineNumber, getFilePath, getSide } from './line-utils';

/**
 * Annotation identity and position.
 *
 * An annotation's `id` is its identity; the `anchor` is where it sits, in
 * source coordinates. Display rows are resolved from anchors at render time
 * (see useAnnotations) and never persisted.
 */

export type Side = 'old' | 'new';

/** One endpoint of a diff anchor: which side, and the 1-indexed source line. */
export type Endpoint = { side: Side; line: number };

/** Mirrors the backend `Anchor` enum: sides only exist where a diff does. */
export type Anchor =
  | { type: 'source'; path: string; start: number; end: number }
  | { type: 'diff'; path: string; start: Endpoint; end: Endpoint };

/** Identity + position of an annotation slot (saved entry or draft). */
export type SlotRef = { id: string; anchor: Anchor };

/** Separator for coordinate keys — cannot occur in paths. */
const SEP = '\u0000';

/**
 * Coordinate keys a line answers to when resolving anchors to display rows.
 * Diff context lines carry both sides and answer on either; virtual lines
 * (portal headers/footers) answer on none.
 */
export function endpointKeys(line: Line): string[] {
  switch (line.origin.type) {
    case 'source':
      return [`${line.origin.path}${SEP}${line.origin.line}`];
    case 'diff': {
      const { path, old_line, new_line } = line.origin;
      const keys: string[] = [];
      if (old_line !== null) keys.push(`${path}${SEP}old${SEP}${old_line}`);
      if (new_line !== null) keys.push(`${path}${SEP}new${SEP}${new_line}`);
      return keys;
    }
    case 'virtual':
      return [];
  }
}

/** The anchor's two lookup keys (start, end). Source anchors are side-less. */
export function anchorKeys(anchor: Anchor): [string, string] {
  if (anchor.type === 'source') {
    return [`${anchor.path}${SEP}${anchor.start}`, `${anchor.path}${SEP}${anchor.end}`];
  }
  return [
    `${anchor.path}${SEP}${anchor.start.side}${SEP}${anchor.start.line}`,
    `${anchor.path}${SEP}${anchor.end.side}${SEP}${anchor.end.line}`,
  ];
}

/** Start/end source lines regardless of variant (labels, ordering). */
export function anchorLines(anchor: Anchor): { start: number; end: number } {
  return anchor.type === 'source'
    ? { start: anchor.start, end: anchor.end }
    : { start: anchor.start.line, end: anchor.end.line };
}

/** Human-readable line label for an anchor, e.g. "50" or "50-55". */
export function anchorLabel(anchor: Anchor): string {
  const { start, end } = anchorLines(anchor);
  return start === end ? `${start}` : `${start}-${end}`;
}

/**
 * Convert a display selection into an anchor at creation time.
 * Validates:
 * 1. All lines in range have non-virtual origin
 * 2. All lines share the same origin.path
 * 3. No line number discontinuities (for portal boundary detection)
 *
 * Returns null if the selection is not annotatable.
 */
export function selectionToAnchor(range: Range, lines: Line[]): Anchor | null {
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

    // Check for line number discontinuity (gap > 1 indicates portal boundary).
    // Skip for diff lines: removed lines number in old-file coordinates while
    // added/context lines use new-file coordinates, so adjacent rows in a hunk
    // legitimately jump (e.g. context new=124, removed old=123, added new=125).
    // Diffs have no portals; hunk/file boundaries are already rejected above
    // because their header lines have no line numbers.
    if (line.origin.type !== 'diff' && prevLineNum !== null && Math.abs(lineNum - prevLineNum) > 1) {
      return null;
    }
    prevLineNum = lineNum;
  }

  const startPoint = { line: getLineNumber(startLine)!, side: getSide(startLine) };
  const endPoint = { line: getLineNumber(endLine)!, side: getSide(endLine) };

  // startLine/endLine (display order) can number in reverse of source order
  // within a diff hunk (old vs new numbering) — swap the pair as a unit so
  // each endpoint's line and side stay paired together.
  const [lo, hi] = endPoint.line < startPoint.line ? [endPoint, startPoint] : [startPoint, endPoint];

  if (startLine.origin.type === 'diff') {
    return {
      type: 'diff',
      path,
      start: { side: lo.side, line: lo.line },
      end: { side: hi.side, line: hi.line },
    };
  }
  return { type: 'source', path, start: lo.line, end: hi.line };
}
