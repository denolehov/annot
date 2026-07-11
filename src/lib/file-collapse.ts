import type { DisplayLine } from './composables/useLineSegments.svelte';
import type { FileEntry } from './file-tree';
import { getDiffKind } from './line-utils';

/** A file's rows within a segment: the header row plus its renderable body. */
export interface FileSection {
  entry: FileEntry;
  header: DisplayLine;
  /** Lines strictly after the header, meta plumbing excluded. */
  body: DisplayLine[];
}

export interface GroupedLines {
  /** Lines preceding the first file header (or orphaned from any section). */
  leading: DisplayLine[];
  sections: FileSection[];
}

/**
 * Split a segment's display lines into per-file sections.
 *
 * Returns null when there are no file entries (non-diff content) so callers
 * can keep the flat render path untouched.
 */
export function groupByFile(displayLines: DisplayLine[], entries: FileEntry[]): GroupedLines | null {
  if (entries.length === 0) return null;

  const leading: DisplayLine[] = [];
  const sections: FileSection[] = [];
  let entryIdx = 0;
  let current: FileSection | null = null;

  for (const dl of displayLines) {
    while (entryIdx < entries.length - 1 && dl.displayIndex > entries[entryIdx].endLine) {
      entryIdx++;
    }
    const entry = entries[entryIdx];

    if (dl.displayIndex === entry.startLine) {
      current = { entry, header: dl, body: [] };
      sections.push(current);
    } else if (current?.entry === entry && dl.displayIndex <= entry.endLine) {
      if (getDiffKind(dl.line) !== 'meta') {
        current.body.push(dl);
      }
    } else {
      leading.push(dl);
    }
  }

  return { leading, sections };
}

/** The entry whose [startLine, endLine] range contains the display index, or null. */
export function fileContaining(entries: FileEntry[], displayIndex: number): FileEntry | null {
  return entries.find((e) => displayIndex >= e.startLine && displayIndex <= e.endLine) ?? null;
}
