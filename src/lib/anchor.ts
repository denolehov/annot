import type { Line } from './types';
import type { Range } from './range';
import { getLineNumber, getFilePath } from './line-utils';

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

/** Lookup key for a side-less source coordinate. */
export function sourceKey(path: string, line: number): string {
  return `${path}${SEP}${line}`;
}

/** Lookup key for a diff coordinate. Shared with the display walk's byEndpoint. */
export function diffKey(path: string, side: Side, line: number): string {
  return `${path}${SEP}${side}${SEP}${line}`;
}

/**
 * Coordinate keys a line answers to when resolving anchors to display rows
 * (non-diff modes; the display walk owns diff coordinates). Virtual lines
 * (portal headers/footers) answer on none.
 */
export function endpointKeys(line: Line): string[] {
  return line.origin.type === 'source' ? [sourceKey(line.origin.path, line.origin.line)] : [];
}

/** The anchor's two lookup keys (start, end). Source anchors are side-less. */
export function anchorKeys(anchor: Anchor): [string, string] {
  if (anchor.type === 'source') {
    return [sourceKey(anchor.path, anchor.start), sourceKey(anchor.path, anchor.end)];
  }
  return [
    diffKey(anchor.path, anchor.start.side, anchor.start.line),
    diffKey(anchor.path, anchor.end.side, anchor.end.line),
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
 * Convert a display selection into a source anchor at creation time
 * (non-diff modes; diff selections resolve through the display walk —
 * see display-rows.ts selectionToDiffAnchor).
 *
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

    // Check for line number discontinuity (gap > 1 indicates portal boundary)
    if (prevLineNum !== null && Math.abs(lineNum - prevLineNum) > 1) {
      return null;
    }
    prevLineNum = lineNum;
  }

  const start = getLineNumber(startLine)!;
  const end = getLineNumber(endLine)!;
  return { type: 'source', path, start: Math.min(start, end), end: Math.max(start, end) };
}
