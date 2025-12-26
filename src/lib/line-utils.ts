import type { Line, LineOrigin, DiffSemantics } from './types';

/** The kind of a diff line from semantics */
export type DiffKind = DiffSemantics['kind'];

/**
 * Get the display line number from a line's origin.
 * Returns null for virtual lines (portal headers, etc).
 */
export function getLineNumber(line: Line): number | null {
  switch (line.origin.type) {
    case 'document':
      return line.origin.line;
    case 'diff':
      // For diff lines, prefer new_line, fallback to old_line
      return line.origin.new_line ?? line.origin.old_line;
    case 'external':
      return line.origin.line;
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
 * Get the file index for a line (diff mode only).
 * Returns null for non-diff lines.
 */
export function getFileIndex(line: Line): number | null {
  return line.origin.type === 'diff' ? line.origin.file_index : null;
}

/**
 * Get the file path for a line (external/portal lines only).
 * Returns null for non-external lines.
 */
export function getFilePath(line: Line): string | null {
  return line.origin.type === 'external' ? line.origin.file : null;
}

/**
 * Get the portal ID for a line (external/portal lines only).
 * Returns null for non-external lines.
 */
export function getPortalId(line: Line): string | null {
  return line.origin.type === 'external' ? line.origin.portal_id : null;
}

/**
 * Check if a line can be selected/annotated.
 */
export function isSelectable(line: Line): boolean {
  // Virtual lines (portal headers) cannot be selected
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
  // Portal headers cannot be selected
  if (line.semantics.type === 'portal' && line.semantics.kind === 'header') {
    return false;
  }
  return true;
}
