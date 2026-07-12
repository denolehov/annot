/**
 * File tree derivation for the diff-review sidebar — a pure projection over
 * the flat DocView list, mirroring display-rows.ts's derive-from-canonical-
 * state shape. Never mutates DocView; nesting and single-child chain
 * compaction are recomputed on every call, so there is nothing to keep in
 * sync with the flat list.
 */

import type { DocView } from './display-rows';

export interface DirNode {
  name: string;
  /** Full path from the tree root — stable identity for expand state. */
  path: string;
  dirs: Map<string, DirNode>;
  files: DocView[];
}

export type FileTreeRow =
  | { kind: 'dir'; path: string; name: string; depth: number }
  | { kind: 'file'; dv: DocView; depth: number };

/** Builds the directory tree for a set of changed files. */
export function buildFileTree(docs: DocView[]): DirNode {
  const root: DirNode = { name: '', path: '', dirs: new Map(), files: [] };
  for (const dv of docs) {
    const segments = dv.dir === '' ? [] : dv.dir.slice(0, -1).split('/');
    let node = root;
    let path = '';
    for (const seg of segments) {
      path = path ? `${path}/${seg}` : seg;
      let child = node.dirs.get(seg);
      if (!child) {
        child = { name: seg, path, dirs: new Map(), files: [] };
        node.dirs.set(seg, child);
      }
      node = child;
    }
    node.files.push(dv);
  }
  return root;
}

/** Follows a chain of single-child, file-less directories, folding it into one row. */
function collapseChain(node: DirNode): DirNode {
  let name = node.name;
  let current = node;
  while (current.files.length === 0 && current.dirs.size === 1) {
    const only = [...current.dirs.values()][0];
    name = `${name}/${only.name}`;
    current = only;
  }
  return current.name === name ? current : { ...current, name };
}

function flatten(node: DirNode, depth: number, isExpanded: (path: string) => boolean, out: FileTreeRow[]) {
  const dirs = [...node.dirs.values()].sort((a, b) => a.name.localeCompare(b.name));
  for (const raw of dirs) {
    const dir = collapseChain(raw);
    out.push({ kind: 'dir', path: dir.path, name: dir.name, depth });
    if (isExpanded(dir.path)) flatten(dir, depth + 1, isExpanded, out);
  }
  const files = [...node.files].sort((a, b) => a.name.localeCompare(b.name));
  for (const dv of files) out.push({ kind: 'file', dv, depth });
}

/**
 * Depth-first row list for rendering: directories before files, alphabetical
 * within each, single-child directory chains compacted into one row.
 * `isExpanded` gates whether a directory's children are walked at all.
 */
export function flattenFileTree(root: DirNode, isExpanded: (path: string) => boolean): FileTreeRow[] {
  const rows: FileTreeRow[] = [];
  flatten(root, 0, isExpanded, rows);
  return rows;
}
