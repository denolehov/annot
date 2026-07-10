import type { DiffMetadata, Line } from './types';
import { getDiffKind } from './line-utils';

/** A changed file in a diff, ready for display in the file tree / palette. */
export interface FileEntry {
  /** Index into DiffMetadata.files */
  index: number;
  /** Full path — new_name, falling back to old_name for deletions */
  path: string;
  /** Directory prefix, with trailing slash, or '' for root-level files */
  dir: string;
  /** Basename */
  name: string;
  added: number;
  deleted: number;
  /** Display index of the file header row */
  startLine: number;
  /** Display index of the file's last row */
  endLine: number;
}

/** Sum added/deleted counts across entries. */
export function diffTotals(entries: FileEntry[]): { added: number; deleted: number } {
  return {
    added: entries.reduce((sum, e) => sum + e.added, 0),
    deleted: entries.reduce((sum, e) => sum + e.deleted, 0),
  };
}

/**
 * Derive the file list from diff metadata + rendered lines.
 *
 * The single place that binds navigation to the diff wire model — when the wire
 * model becomes per-file documents, only this function moves.
 */
export function deriveFileEntries(lines: Line[], meta: DiffMetadata | null): FileEntry[] {
  if (!meta) return [];

  return meta.files.map((file, index) => {
    const path = file.new_name ?? file.old_name ?? '';
    const slash = path.lastIndexOf('/');

    const kinds = lines
      .slice(file.start_line - 1, file.end_line)
      .map(getDiffKind);

    return {
      index,
      path,
      dir: path.slice(0, slash + 1),
      name: path.slice(slash + 1),
      added: kinds.filter((k) => k === 'added').length,
      deleted: kinds.filter((k) => k === 'deleted').length,
      startLine: file.start_line,
      endLine: file.end_line,
    };
  });
}
