import { describe, it, expect, beforeEach } from 'vitest';
import { setFileItems, getFileItems, filterFileItems } from './files';
import { createQueryContext } from './index';
import type { FileEntry } from '$lib/file-tree';

function entry(index: number, path: string, startLine: number): FileEntry {
  const slash = path.lastIndexOf('/');
  return {
    index,
    path,
    dir: path.slice(0, slash + 1),
    name: path.slice(slash + 1),
    added: 1,
    deleted: 0,
    startLine,
  };
}

describe('files namespace', () => {
  beforeEach(() => {
    setFileItems([]);
  });

  it('turns each file into a jump action carrying its display index', () => {
    setFileItems([entry(0, 'src/lib/types.ts', 12)]);

    expect(getFileItems()[0]).toMatchObject({
      id: 'file-0',
      name: 'src/lib/types.ts',
      action: { type: 'EMIT_EVENT', event: 'JUMP_TO_FILE', payload: 12 },
    });
  });

  it('fuzzy-matches on the full path', () => {
    setFileItems([entry(0, 'src/lib/types.ts', 1), entry(1, 'src-tauri/src/diff.rs', 40)]);

    expect(filterFileItems('lib/typ').map((i) => i.name)).toEqual(['src/lib/types.ts']);
  });

  it('is hidden from the palette when there are no files', () => {
    expect(createQueryContext().namespaces.map((n) => n.id)).not.toContain('files');

    setFileItems([entry(0, 'a.ts', 1)]);

    expect(createQueryContext().namespaces.map((n) => n.id)).toContain('files');
  });
});
