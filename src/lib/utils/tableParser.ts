/**
 * Table parsing utilities for markdown tables.
 */

export type Alignment = 'left' | 'center' | 'right';

export interface ParsedTable {
  headerRow: number; // Index of the header row in the lines array
  separatorRow: number; // Index of the separator row
  alignments: Alignment[];
  columnCount: number;
}

/**
 * Check if a line is a table separator row (e.g., |:---|:---:|---:|).
 */
export function isSeparatorRow(content: string): boolean {
  const trimmed = content.trim();
  // Must contain pipes and dashes/colons
  if (!trimmed.includes('|') || !trimmed.includes('-')) return false;

  // Split by pipes and check each cell
  const cells = splitTableRow(trimmed);
  if (cells.length === 0) return false;

  // Each cell must match separator pattern: optional colon, dashes, optional colon
  const separatorPattern = /^:?-+:?$/;
  return cells.every((cell) => separatorPattern.test(cell.trim()));
}

/**
 * Split a table row into cells by pipe delimiter.
 * Handles leading/trailing pipes.
 */
export function splitTableRow(content: string): string[] {
  let trimmed = content.trim();

  // Remove leading pipe if present
  if (trimmed.startsWith('|')) {
    trimmed = trimmed.slice(1);
  }
  // Remove trailing pipe if present
  if (trimmed.endsWith('|')) {
    trimmed = trimmed.slice(0, -1);
  }

  return trimmed.split('|').map((cell) => cell.trim());
}

/**
 * Parse alignment from a separator cell.
 * :--- = left, :---: = center, ---: = right, --- = left (default)
 */
export function parseAlignment(cell: string): Alignment {
  const trimmed = cell.trim();
  const hasLeftColon = trimmed.startsWith(':');
  const hasRightColon = trimmed.endsWith(':');

  if (hasLeftColon && hasRightColon) return 'center';
  if (hasRightColon) return 'right';
  return 'left';
}

/**
 * Parse alignments from a separator row.
 */
export function parseAlignments(separatorContent: string): Alignment[] {
  const cells = splitTableRow(separatorContent);
  return cells.map(parseAlignment);
}

/**
 * Analyze table structure from a sequence of table lines.
 * Returns parsed table info with header row index, separator row index, and alignments.
 */
export function analyzeTable(
  lines: Array<{ content: string; displayIndex: number }>
): ParsedTable | null {
  if (lines.length < 2) return null;

  // Find separator row (typically the second row)
  let separatorIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (isSeparatorRow(lines[i].content)) {
      separatorIdx = i;
      break;
    }
  }

  if (separatorIdx === -1 || separatorIdx === 0) {
    // No separator found, or separator is first row (invalid)
    return null;
  }

  const alignments = parseAlignments(lines[separatorIdx].content);
  const headerCells = splitTableRow(lines[separatorIdx - 1].content);

  return {
    headerRow: separatorIdx - 1,
    separatorRow: separatorIdx,
    alignments,
    columnCount: Math.max(alignments.length, headerCells.length),
  };
}
