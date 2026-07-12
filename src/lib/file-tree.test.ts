import { describe, it, expect } from 'vitest';
import { buildFileTree, flattenFileTree } from './file-tree';
import type { DocView } from './display-rows';
import type { DiffDocument } from './types';

// Only `path`/`dir`/`name`/`index` are read by file-tree.ts; the rest is
// present to satisfy DocView's shape.
function dv(path: string, index: number): DocView {
  const slash = path.lastIndexOf('/');
  const doc: DiffDocument = {
    path,
    old_path: null,
    status: 'modified',
    unavailable: false,
    new_len: null,
    language: '',
    hunks: [],
  };
  return {
    index,
    doc,
    path,
    dir: path.slice(0, slash + 1),
    name: path.slice(slash + 1),
    added: 0,
    deleted: 0,
    headerDisplayIndex: index,
    endDisplayIndex: index,
    hunks: [],
    trailingGap: 0,
  };
}

function rows(docs: DocView[], collapsed: Set<string> = new Set()) {
  return flattenFileTree(buildFileTree(docs), (path) => !collapsed.has(path));
}

describe('buildFileTree / flattenFileTree', () => {
  it('places root-level files at depth 0 with no directory rows', () => {
    const result = rows([dv('README.md', 0), dv('Cargo.toml', 1)]);
    expect(result).toEqual([
      { kind: 'file', dv: expect.objectContaining({ path: 'Cargo.toml' }), depth: 0 },
      { kind: 'file', dv: expect.objectContaining({ path: 'README.md' }), depth: 0 },
    ]);
  });

  it('compacts a single-child directory chain into one row', () => {
    const result = rows([dv('src/lib/composables/useFileTree.svelte.ts', 0)]);
    expect(result[0]).toMatchObject({ kind: 'dir', path: 'src/lib/composables', name: 'src/lib/composables', depth: 0 });
    expect(result[1]).toMatchObject({ kind: 'file', depth: 1 });
  });

  it('does not compact a directory that branches', () => {
    const result = rows([dv('src/lib/a.ts', 0), dv('src/tauri/b.rs', 1)]);
    const dirs = result.filter((r) => r.kind === 'dir');
    expect(dirs).toEqual([
      { kind: 'dir', path: 'src', name: 'src', depth: 0 },
      { kind: 'dir', path: 'src/lib', name: 'lib', depth: 1 },
      { kind: 'dir', path: 'src/tauri', name: 'tauri', depth: 1 },
    ]);
  });

  it('sorts directories before files, alphabetically within each', () => {
    const result = rows([dv('b.ts', 0), dv('a.ts', 1), dv('zdir/x.ts', 2), dv('adir/y.ts', 3)]);
    expect(result.map((r) => (r.kind === 'dir' ? r.name : r.dv.path))).toEqual([
      'adir',
      'adir/y.ts',
      'zdir',
      'zdir/x.ts',
      'a.ts',
      'b.ts',
    ]);
  });

  it('skips a collapsed directory\'s descendants but keeps its own row', () => {
    const docs = [dv('src/a.ts', 0), dv('src/b.ts', 1)];
    const result = rows(docs, new Set(['src']));
    expect(result).toEqual([{ kind: 'dir', path: 'src', name: 'src', depth: 0 }]);
  });

  it('is expanded by default (nothing collapsed unless named)', () => {
    const result = rows([dv('src/a.ts', 0)]);
    expect(result).toHaveLength(2);
  });
});
