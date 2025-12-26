import type { Line, DiffSemantics } from './types';

/** The kind of a diff line from semantics */
export type DiffKind = DiffSemantics['kind'];

/**
 * Get the display line number from a line's origin.
 * Returns null for virtual lines (portal headers/footers, etc).
 */
export function getLineNumber(line: Line): number | null {
  switch (line.origin.type) {
    case 'source':
      return line.origin.line;
    case 'diff':
      // For diff lines, prefer new_line, fallback to old_line
      return line.origin.new_line ?? line.origin.old_line;
    case 'virtual':
      return null;
  }
}

/**
 * Get a unique identifier for a line (for keying, data attributes, etc).
 * For virtual lines, returns a synthetic ID.
 */
export function getLineId(line: Line, index: number): number {
  const num = getLineNumber(line);
  return num ?? -(index + 1); // Negative for virtual lines
}

/**
 * Check if a line is a diff line and get its diff kind.
 */
export function getDiffKind(line: Line): DiffKind | null {
  if (line.semantics.type === 'diff') {
    return line.semantics.kind;
  }
  return null;
}

/**
 * Get the file path from a line's origin.
 * Returns null for virtual lines.
 */
export function getFilePath(line: Line): string | null {
  switch (line.origin.type) {
    case 'source':
    case 'diff':
      return line.origin.path;
    case 'virtual':
      return null;
  }
}

/**
 * Check if a line is part of a portal (has portal semantics).
 */
export function isPortalLine(line: Line): boolean {
  return line.semantics.type === 'portal';
}

/**
 * Check if a line can be selected/annotated.
 */
export function isSelectable(line: Line): boolean {
  // Virtual lines (portal headers/footers) cannot be selected
  if (line.origin.type === 'virtual') {
    return false;
  }
  // Diff file headers and hunk headers cannot be selected
  if (line.semantics.type === 'diff') {
    const kind = line.semantics.kind;
    if (kind === 'file_header' || kind === 'hunk_header') {
      return false;
    }
  }
  // Portal headers and footers cannot be selected
  if (line.semantics.type === 'portal' && (line.semantics.kind === 'header' || line.semantics.kind === 'footer')) {
    return false;
  }
  return true;
}
