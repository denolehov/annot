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
 * Check if a line is a code block fence line (``` markers).
 */
export function isCodeBlockFence(line: Line): boolean {
  if (line.semantics.type === 'markdown') {
    const kind = line.semantics.kind;
    return kind === 'code_block_start' || kind === 'code_block_end';
  }
  return false;
}

/**
 * Check if a line is inside a code block (content, not fence).
 */
export function isCodeBlockContent(line: Line): boolean {
  return line.semantics.type === 'markdown' && line.semantics.kind === 'code_block_content';
}

/**
 * Check if a line is any part of a code block (fence or content).
 */
export function isCodeBlockLine(line: Line): boolean {
  if (line.semantics.type === 'markdown') {
    const kind = line.semantics.kind;
    return kind === 'code_block_start' || kind === 'code_block_content' || kind === 'code_block_end';
  }
  return false;
}

/**
 * Check if a line is a table row.
 */
export function isTableLine(line: Line): boolean {
  return line.semantics.type === 'markdown' && line.semantics.kind === 'table_row';
}

/**
 * Extract content from a code block in a specific file.
 * Filters by source line numbers AND file path to exclude portal content.
 *
 * @param lines - All lines in the document
 * @param startLine - Source line number of the opening fence (exclusive)
 * @param endLine - Source line number of the closing fence (exclusive)
 * @param filePath - Path of the file containing the code block
 * @returns The code block content as a single string
 */
export function extractCodeBlockContent(
  lines: Line[],
  startLine: number,
  endLine: number,
  filePath: string
): string {
  return lines
    .filter(l => {
      const num = getLineNumber(l);
      const path = getFilePath(l);
      return num !== null && num > startLine && num < endLine && path === filePath;
    })
    .map(l => l.content)
    .join('\n');
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
  // Code block fence lines (``` markers) cannot be selected
  if (isCodeBlockFence(line)) {
    return false;
  }
  return true;
}
