import { describe, it, expect } from 'vitest';
import { deriveFileEntries, diffTotals } from './file-tree';
import type { DiffFileInfo, DiffMetadata, Line } from './types';

function diffLine(kind: 'file_header' | 'added' | 'deleted' | 'context', path = 'a'): Line {
  return {
    content: '',
    html: null,
    origin: { type: 'diff', path, old_line: null, new_line: null },
    semantics: { type: 'diff', kind },
  };
}

function file(partial: Partial<DiffFileInfo>): DiffFileInfo {
  return {
    old_name: null,
    new_name: null,
    language: 'ts',
    start_line: 1,
    end_line: 1,
    hunks: [],
    ...partial,
  };
}

describe('deriveFileEntries', () => {
  it('returns [] without diff metadata', () => {
    expect(deriveFileEntries([], null)).toEqual([]);
  });

  it('splits the path into a dimmable directory prefix and a basename', () => {
    const meta: DiffMetadata = {
      files: [
        file({ new_name: 'src/lib/types.ts', start_line: 1, end_line: 1 }),
        file({ new_name: 'README.md', start_line: 2, end_line: 2 }),
      ],
    };

    const entries = deriveFileEntries([diffLine('file_header'), diffLine('file_header')], meta);

    expect(entries[0]).toMatchObject({ path: 'src/lib/types.ts', dir: 'src/lib/', name: 'types.ts' });
    expect(entries[1]).toMatchObject({ path: 'README.md', dir: '', name: 'README.md' });
  });

  it('falls back to old_name for deleted files', () => {
    const meta: DiffMetadata = { files: [file({ old_name: 'gone.ts', new_name: null })] };

    expect(deriveFileEntries([diffLine('file_header')], meta)[0].path).toBe('gone.ts');
  });

  it('counts added/deleted lines within each file range', () => {
    // display: 1 header, 2 added, 3 deleted, 4 context | 5 header, 6 added
    const lines = [
      diffLine('file_header'),
      diffLine('added'),
      diffLine('deleted'),
      diffLine('context'),
      diffLine('file_header'),
      diffLine('added'),
    ];
    const meta: DiffMetadata = {
      files: [
        file({ new_name: 'a.ts', start_line: 1, end_line: 4 }),
        file({ new_name: 'b.ts', start_line: 5, end_line: 6 }),
      ],
    };

    const entries = deriveFileEntries(lines, meta);

    expect(entries[0]).toMatchObject({ index: 0, added: 1, deleted: 1, startLine: 1, endLine: 4 });
    expect(entries[1]).toMatchObject({ index: 1, added: 1, deleted: 0, startLine: 5, endLine: 6 });
  });
});

describe('diffTotals', () => {
  it('sums added/deleted across entries', () => {
    const lines = [
      diffLine('file_header'),
      diffLine('added'),
      diffLine('deleted'),
      diffLine('file_header'),
      diffLine('added'),
    ];
    const meta: DiffMetadata = {
      files: [
        file({ new_name: 'a.ts', start_line: 1, end_line: 3 }),
        file({ new_name: 'b.ts', start_line: 4, end_line: 5 }),
      ],
    };

    expect(diffTotals(deriveFileEntries(lines, meta))).toEqual({ added: 2, deleted: 1 });
  });
});
