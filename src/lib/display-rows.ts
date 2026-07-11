/**
 * The DisplayRow spine — one derived walk over per-file diff documents is the
 * single source of display truth for diff mode.
 *
 * Strangler: while the wire is still flat, documents are *synthesized* from
 * `lines` + `metadata.files` by `synthesizeDocs`, carrying transitional
 * wire-space display indexes so peeled and unpeeled consumers keep speaking
 * one index space. The per-file wire migration swaps the walk's input to real
 * wire documents and deletes the synthesis — nothing below `deriveDisplay`
 * changes; the `wireIndex ?? position` fallback flips indexes to positional
 * by itself.
 *
 * The walk is total: every doc, every row, always. Collapse is a render-time
 * visibility skip, never a walk concern — display indexes are stable under
 * toggle.
 */

import type { DiffDocument, DiffFileInfo, HunkV2, Line, LineRange, Row } from './types';
import type { FileEntry } from './file-tree';
import type { Range } from './range';
import { diffKey, type Anchor, type Endpoint } from './anchor';

// =============================================================================
// Pseudo-documents (flat-wire shim, deleted with the per-file wire migration)
// =============================================================================

/** `Row` plus its transitional wire-space display index. */
export interface PseudoRow extends Row {
  wireIndex?: number;
}

/** `HunkV2` plus the wire index of its `@@` header row. */
export interface PseudoHunk extends HunkV2 {
  rows: PseudoRow[];
  wireIndex?: number;
}

/**
 * `DiffDocument` plus transitional wire indexes. `status`/`unavailable` are
 * stubs the flat wire cannot fill; the per-file wire makes them real.
 */
export interface PseudoDoc extends DiffDocument {
  hunks: PseudoHunk[];
  /** Wire index of the file-header row. */
  wireIndex?: number;
  /** Wire index of the file's last row (trailing meta plumbing included). */
  endWireIndex?: number;
}

/**
 * Promote the flat wire into per-file documents. Meta plumbing rows (binary
 * markers) are structure the documents don't carry; header rows become the
 * walk's synthesized entries.
 */
export function synthesizeDocs(lines: Line[], files: DiffFileInfo[]): PseudoDoc[] {
  return files.map((file) => {
    const path = file.new_name ?? file.old_name ?? '';

    const hunks = file.hunks.map((hunk, i): PseudoHunk => {
      const headerIdx = hunk.display_line;
      const nextHeaderIdx = file.hunks[i + 1]?.display_line ?? file.end_line + 1;
      const rows = lines
        .slice(headerIdx, nextHeaderIdx - 1)
        .map((line, offset): PseudoRow | null =>
          line.origin.type === 'diff' && (line.origin.old_line !== null || line.origin.new_line !== null)
            ? {
                old_line: line.origin.old_line,
                new_line: line.origin.new_line,
                content: line.content,
                html: line.html,
                wireIndex: headerIdx + 1 + offset,
              }
            : null,
        )
        .filter((row): row is PseudoRow => row !== null);

      return {
        old_range: { start: hunk.old_start, end: hunk.old_start + hunk.old_count },
        new_range: { start: hunk.new_start, end: hunk.new_start + hunk.new_count },
        function_context: hunk.function_context,
        function_context_html: hunk.function_context_html,
        rows,
        wireIndex: headerIdx,
      };
    });

    const renamed =
      file.old_name !== null && file.new_name !== null && file.old_name !== file.new_name;

    return {
      path,
      old_path: renamed ? file.old_name : null,
      status: 'modified',
      unavailable: false,
      language: file.language,
      hunks,
      wireIndex: file.start_line,
      endWireIndex: file.end_line,
    };
  });
}

// =============================================================================
// The walk
// =============================================================================

export type RowKind = 'added' | 'deleted' | 'context';

export type DisplayRow =
  | { kind: 'file-header'; docIdx: number; displayIndex: number }
  | { kind: 'hunk-header'; docIdx: number; hunkIdx: number; displayIndex: number }
  | { kind: 'row'; docIdx: number; hunkIdx: number; row: Row; rowKind: RowKind; displayIndex: number };

/** A hunk's footprint in display space. */
export interface HunkView {
  headerDisplayIndex: number;
  /** Display-index span of the hunk's rows — the selectable region. */
  rowStart: number;
  rowEnd: number;
}

/** Per-document view: `deriveFileEntries` reborn inside the walk. */
export interface DocView {
  /** Index into the documents array — `FileKey::diff_file` identity. */
  index: number;
  doc: DiffDocument;
  /** Full path — mirrors `doc.path`. */
  path: string;
  /** Directory prefix with trailing slash, '' for root-level files. */
  dir: string;
  /** Basename. */
  name: string;
  added: number;
  deleted: number;
  /** Display index of the file-header entry. */
  headerDisplayIndex: number;
  /** Display index of the file's last entry. */
  endDisplayIndex: number;
  /** Display footprints, parallel to `doc.hunks`. */
  hunks: HunkView[];
}

export interface DiffDisplay {
  rows: DisplayRow[];
  docs: DocView[];
  /** displayIndex → entry: selection, scroll targeting, annotation slots. */
  byIndex: Map<number, DisplayRow>;
  /** anchor.ts `diffKey(path, side, line)` → displayIndex. */
  byEndpoint: Map<string, number>;
}

/**
 * Presentation text for a hunk header: `@@ -a,b +c,d @@ ctx`.
 * Git omits the count when it is 1: `-3` not `-3,1`.
 */
export function hunkHeaderText(hunk: HunkV2): string {
  const side = (sign: string, range: LineRange) => {
    const count = range.end - range.start;
    return count === 1 ? `${sign}${range.start}` : `${sign}${range.start},${count}`;
  };
  const marker = `@@ ${side('-', hunk.old_range)} ${side('+', hunk.new_range)} @@`;
  return hunk.function_context ? `${marker} ${hunk.function_context}` : marker;
}

/** Which side(s) a row lives on: old-only = deleted, new-only = added. */
export function rowKind(row: Row): RowKind {
  if (row.old_line === null) return 'added';
  if (row.new_line === null) return 'deleted';
  return 'context';
}


/** The single display-truth derivation for diff mode. */
export function deriveDisplay(docs: PseudoDoc[]): DiffDisplay {
  const rows: DisplayRow[] = [];
  const docViews: DocView[] = [];
  const byIndex = new Map<number, DisplayRow>();
  const byEndpoint = new Map<string, number>();
  let pos = 0;

  // Wire-space today; positional once real wire docs carry no wireIndex.
  const stamp = (wireIndex?: number): number => {
    pos += 1;
    return wireIndex ?? pos;
  };

  const push = (entry: DisplayRow) => {
    rows.push(entry);
    byIndex.set(entry.displayIndex, entry);
  };

  docs.forEach((doc, docIdx) => {
    const headerDisplayIndex = stamp(doc.wireIndex);
    push({ kind: 'file-header', docIdx, displayIndex: headerDisplayIndex });

    let added = 0;
    let deleted = 0;
    const hunkViews: HunkView[] = [];

    doc.hunks.forEach((hunk, hunkIdx) => {
      const headerIdx = stamp(hunk.wireIndex);
      push({ kind: 'hunk-header', docIdx, hunkIdx, displayIndex: headerIdx });

      let rowStart: number | null = null;
      let rowEnd = headerIdx;
      for (const row of hunk.rows) {
        const kind = rowKind(row);
        if (kind === 'added') added += 1;
        else if (kind === 'deleted') deleted += 1;

        const displayIndex = stamp((row as PseudoRow).wireIndex);
        push({ kind: 'row', docIdx, hunkIdx, row, rowKind: kind, displayIndex });
        rowStart ??= displayIndex;
        rowEnd = displayIndex;

        if (row.old_line !== null) byEndpoint.set(diffKey(doc.path, 'old', row.old_line), displayIndex);
        if (row.new_line !== null) byEndpoint.set(diffKey(doc.path, 'new', row.new_line), displayIndex);
      }
      hunkViews.push({ headerDisplayIndex: headerIdx, rowStart: rowStart ?? headerIdx + 1, rowEnd });
    });

    const slash = doc.path.lastIndexOf('/');
    docViews.push({
      index: docIdx,
      doc,
      path: doc.path,
      dir: doc.path.slice(0, slash + 1),
      name: doc.path.slice(slash + 1),
      added,
      deleted,
      headerDisplayIndex,
      endDisplayIndex: doc.endWireIndex ?? rows[rows.length - 1].displayIndex,
      hunks: hunkViews,
    });
  });

  return { rows, docs: docViews, byIndex, byEndpoint };
}

/** A row's anchor endpoint: new-side for added/context rows, old-side for deleted. */
function rowEndpoint(row: Row): Endpoint {
  return row.new_line !== null
    ? { side: 'new', line: row.new_line }
    : { side: 'old', line: row.old_line! };
}

/**
 * Convert a display selection into a diff anchor. Valid only when every
 * index in the range is a row of the same document — headers and
 * cross-document spans are not annotatable.
 */
export function selectionToDiffAnchor(range: Range, display: DiffDisplay): Anchor | null {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);

  const entries: Extract<DisplayRow, { kind: 'row' }>[] = [];
  for (let i = min; i <= max; i++) {
    const entry = display.byIndex.get(i);
    if (!entry || entry.kind !== 'row') return null;
    entries.push(entry);
  }
  if (entries.length === 0 || entries.some((e) => e.docIdx !== entries[0].docIdx)) return null;

  const startPoint = rowEndpoint(entries[0].row);
  const endPoint = rowEndpoint(entries[entries.length - 1].row);

  // Display order can number in reverse of source order within a hunk (old vs
  // new numbering) — swap the pair as a unit so each endpoint's line and side
  // stay paired together.
  const [lo, hi] = endPoint.line < startPoint.line ? [endPoint, startPoint] : [startPoint, endPoint];

  return { type: 'diff', path: display.docs[entries[0].docIdx].path, start: lo, end: hi };
}

/**
 * Transitional adapter: DocViews in the legacy FileEntry shape, for consumers
 * not yet reading the walk directly. Dies with its last consumer.
 */
export function toFileEntries(display: DiffDisplay): FileEntry[] {
  return display.docs.map((dv) => ({
    index: dv.index,
    path: dv.path,
    dir: dv.dir,
    name: dv.name,
    added: dv.added,
    deleted: dv.deleted,
    startLine: dv.headerDisplayIndex,
    endLine: dv.endDisplayIndex,
  }));
}
