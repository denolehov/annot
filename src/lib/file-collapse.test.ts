import { describe, it, expect } from 'vitest';
import { groupByFile, autoCollapsedIndices, fileContaining, AUTO_COLLAPSE_THRESHOLD } from './file-collapse';
import type { FileEntry } from './file-tree';
import type { DisplayLine } from './composables/useLineSegments.svelte';
import type { Line } from './types';

type Kind = 'file_header' | 'hunk_header' | 'meta' | 'added' | 'deleted' | 'context';

function diffLine(kind: Kind, path = 'a'): Line {
  const semantics =
    kind === 'hunk_header'
      ? ({ type: 'diff', kind, context: null } as const)
      : ({ type: 'diff', kind } as const);
  return {
    content: '',
    html: null,
    origin: { type: 'diff', path, old_line: null, new_line: null },
    semantics,
  };
}

function toDisplay(lines: Line[]): DisplayLine[] {
  return lines.map((line, i) => ({ line, displayIndex: i + 1 }));
}

function entry(partial: Partial<FileEntry> & Pick<FileEntry, 'index' | 'startLine' | 'endLine'>): FileEntry {
  return {
    path: 'a.ts',
    dir: '',
    name: 'a.ts',
    added: 0,
    deleted: 0,
    ...partial,
  };
}

// display: 1 header, 2 meta, 3 hunk, 4 added, 5 deleted | 6 header, 7 meta, 8 hunk, 9 added
const LINES = [
  diffLine('file_header', 'a.ts'),
  diffLine('meta', 'a.ts'),
  diffLine('hunk_header', 'a.ts'),
  diffLine('added', 'a.ts'),
  diffLine('deleted', 'a.ts'),
  diffLine('file_header', 'b.ts'),
  diffLine('meta', 'b.ts'),
  diffLine('hunk_header', 'b.ts'),
  diffLine('added', 'b.ts'),
];
const ENTRIES = [
  entry({ index: 0, startLine: 1, endLine: 5 }),
  entry({ index: 1, startLine: 6, endLine: 9 }),
];

describe('groupByFile', () => {
  it('returns null without file entries (non-diff content)', () => {
    expect(groupByFile(toDisplay(LINES), [])).toBeNull();
  });

  it('splits lines into per-file sections with the header separated out', () => {
    const grouped = groupByFile(toDisplay(LINES), ENTRIES)!;

    expect(grouped.leading).toEqual([]);
    expect(grouped.sections).toHaveLength(2);
    expect(grouped.sections[0].header.displayIndex).toBe(1);
    expect(grouped.sections[1].header.displayIndex).toBe(6);
    expect(grouped.sections[1].entry.index).toBe(1);
  });

  it('excludes meta plumbing lines from section bodies', () => {
    const grouped = groupByFile(toDisplay(LINES), ENTRIES)!;

    expect(grouped.sections[0].body.map((dl) => dl.displayIndex)).toEqual([3, 4, 5]);
    expect(grouped.sections[1].body.map((dl) => dl.displayIndex)).toEqual([8, 9]);
  });

  it('handles the last file ending at the final line', () => {
    const grouped = groupByFile(toDisplay(LINES), ENTRIES)!;
    const last = grouped.sections[1].body.at(-1)!;

    expect(last.displayIndex).toBe(9);
  });

  it('keeps display indices and line references byte-identical (collapse invariant)', () => {
    // Annotation semantics are display-index-keyed: grouping (the render-skip
    // input) must never re-index or clone lines, whatever gets collapsed.
    const display = toDisplay(LINES);
    const annotatedIndex = 9; // "annotated" line in file 2

    const grouped = groupByFile(display, ENTRIES)!;
    const pair = grouped.sections
      .flatMap((s) => [s.header, ...s.body])
      .find((dl) => dl.displayIndex === annotatedIndex)!;

    expect(pair).toBe(display[annotatedIndex - 1]);
    expect(pair.line).toBe(LINES[annotatedIndex - 1]);
  });

  it('routes lines before the first file header into leading', () => {
    const lines = [diffLine('context'), ...LINES];
    const entries = [
      entry({ index: 0, startLine: 2, endLine: 6 }),
      entry({ index: 1, startLine: 7, endLine: 10 }),
    ];

    const grouped = groupByFile(toDisplay(lines), entries)!;

    expect(grouped.leading.map((dl) => dl.displayIndex)).toEqual([1]);
    expect(grouped.sections[0].header.displayIndex).toBe(2);
  });
});

describe('autoCollapsedIndices', () => {
  it('collapses files strictly above the threshold', () => {
    const entries = [
      entry({ index: 0, startLine: 1, endLine: 2, added: AUTO_COLLAPSE_THRESHOLD, deleted: 1 }),
      entry({ index: 1, startLine: 3, endLine: 4, added: AUTO_COLLAPSE_THRESHOLD, deleted: 0 }),
      entry({ index: 2, startLine: 5, endLine: 6, added: 3, deleted: 4 }),
    ];

    expect(autoCollapsedIndices(entries)).toEqual([0]);
  });
});

describe('fileContaining', () => {
  it('resolves boundary and interior display indices, null outside', () => {
    expect(fileContaining(ENTRIES, 1)?.index).toBe(0);
    expect(fileContaining(ENTRIES, 5)?.index).toBe(0);
    expect(fileContaining(ENTRIES, 6)?.index).toBe(1);
    expect(fileContaining(ENTRIES, 42)).toBeNull();
  });
});
