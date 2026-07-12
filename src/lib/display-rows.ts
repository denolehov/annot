/**
 * The DisplayRow spine — one derived walk over per-file diff documents is the
 * single source of display truth for diff mode. Documents arrive on the wire
 * (`ContentView::Diff`); headers are structure the walk synthesizes.
 *
 * The walk is total: every doc, every row, always. Collapse is a render-time
 * visibility skip, never a walk concern — display indexes are stable under
 * toggle.
 */

import type { DiffDocument, HunkV2, LineRange, Row } from './types';
import type { Range } from './range';
import { diffKey, type Anchor, type Endpoint, type Side } from './anchor';

/** Context lines revealed per directional unfold click — mirrors `pipeline::EXPAND_STEP`. */
export const EXPAND_STEP = 20;

/** Diff rendering projection: one column or two. Session-scoped, not persisted. */
export type DiffViewMode = 'unified' | 'split';

export type RowKind = 'added' | 'deleted' | 'context';

/**
 * A row's place in a materialized merge conflict (jj only — see
 * `FileSource::is_conflicted`).
 *
 * - `marker` — a conflict marker line: `<<<<<<<`, `%%%%%%%`, `+++++++`,
 *   `-------`, `>>>>>>>`. The text after the marker is the side label jj
 *   wrote ("Contents of side #2"), so the line is its own caption.
 * - `body` — a line between markers: one side's actual content.
 * - `null` — an ordinary line.
 */
export type ConflictPart = 'marker' | 'body' | null;

/**
 * Conflict markers are 7-or-more repeats of a marker char; jj lengthens them
 * when the content itself contains marker-looking lines, so the count is not
 * fixed at 7.
 *
 * Only ever applied within a file the backend flagged `conflicted` — otherwise
 * a markdown rule (`-------`) or a doc *about* conflicts would light up.
 */
const CONFLICT_MARKER = /^(<{7,}|>{7,}|%{7,}|\+{7,}|-{7,})(\s|$)/;

export function isConflictMarker(content: string): boolean {
  return CONFLICT_MARKER.test(content);
}

export type DisplayRow =
  | { kind: 'file-header'; docIdx: number; displayIndex: number }
  | { kind: 'hunk-header'; docIdx: number; hunkIdx: number; displayIndex: number }
  | {
      kind: 'row';
      docIdx: number;
      hunkIdx: number;
      row: Row;
      rowKind: RowKind;
      /**
       * True when this row should draw a top/bottom run border — the edge of
       * a contiguous added/deleted run against context or a hunk boundary,
       * but not against an adjacent run of the opposite kind. Unused for
       * context rows.
       */
      runStart: boolean;
      runEnd: boolean;
      /** Place in a conflict region; `null` outside one (and always in git). */
      conflict: ConflictPart;
      displayIndex: number;
    };

/** A hunk's footprint in display space. */
export interface HunkView {
  headerDisplayIndex: number;
  /** Display-index span of the hunk's rows — the selectable region. */
  rowStart: number;
  rowEnd: number;
  /**
   * Folded context lines between this hunk and the previous one (or the
   * file top). 0 when nothing is folded or the document can't unfold —
   * a gap bar renders exactly when this is positive.
   */
  gapAbove: number;
}

/** Per-document view: file identity, counts, and display footprint. */
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
  /** Folded context lines after the last hunk; 0 when none or can't unfold. */
  trailingGap: number;
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
 * Git omits the count when it is 1: `-3` not `-3,1`. Ranges arrive in
 * git-printed convention, so the numbers read off verbatim.
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

/**
 * Ranges arrive in git-printed convention — an empty side starts at the line
 * *before* the position. Gap arithmetic needs the true half-open range,
 * where an empty side sits at its insertion point.
 */
function trueRange(range: LineRange): LineRange {
  return range.start === range.end ? { start: range.start + 1, end: range.start + 1 } : range;
}

/**
 * Folded context lines between hunk `idx` and its upper neighbor (or the
 * file top). Pure new-side range arithmetic — expansion state is nothing
 * but the ranges themselves.
 */
function gapAbove(doc: DiffDocument, idx: number): number {
  if (doc.new_len === null) return 0;
  const start = trueRange(doc.hunks[idx].new_range).start;
  const bound = idx === 0 ? 1 : trueRange(doc.hunks[idx - 1].new_range).end;
  return start - bound;
}

/** Folded context lines after the last hunk. */
function trailingGap(doc: DiffDocument): number {
  if (doc.new_len === null || doc.hunks.length === 0) return 0;
  const end = trueRange(doc.hunks[doc.hunks.length - 1].new_range).end;
  return doc.new_len + 1 - end;
}

/** The single display-truth derivation for diff mode. */
export function deriveDisplay(docs: DiffDocument[]): DiffDisplay {
  const rows: DisplayRow[] = [];
  const docViews: DocView[] = [];
  const byIndex = new Map<number, DisplayRow>();
  const byEndpoint = new Map<string, number>();
  let pos = 0;

  const stamp = (): number => {
    pos += 1;
    return pos;
  };

  const push = (entry: DisplayRow) => {
    rows.push(entry);
    byIndex.set(entry.displayIndex, entry);
  };

  docs.forEach((doc, docIdx) => {
    const headerDisplayIndex = stamp();
    push({ kind: 'file-header', docIdx, displayIndex: headerDisplayIndex });

    let added = 0;
    let deleted = 0;
    const hunkViews: HunkView[] = [];

    doc.hunks.forEach((hunk, hunkIdx) => {
      const headerIdx = stamp();
      push({ kind: 'hunk-header', docIdx, hunkIdx, displayIndex: headerIdx });

      let rowStart: number | null = null;
      let rowEnd = headerIdx;
      // Depth, not a boolean: jj emits one region per conflicted hunk, and a
      // file can hold several. Reset per hunk — an unfold never straddles one.
      let insideConflict = false;
      hunk.rows.forEach((row, i) => {
        const kind = rowKind(row);
        if (kind === 'added') added += 1;
        else if (kind === 'deleted') deleted += 1;

        const prevKind = i > 0 ? rowKind(hunk.rows[i - 1]) : null;
        const nextKind = i < hunk.rows.length - 1 ? rowKind(hunk.rows[i + 1]) : null;

        let conflict: ConflictPart = null;
        if (doc.conflicted && isConflictMarker(row.content)) {
          conflict = 'marker';
          if (row.content.startsWith('<')) insideConflict = true;
          else if (row.content.startsWith('>')) insideConflict = false;
        } else if (doc.conflicted && insideConflict) {
          conflict = 'body';
        }

        const displayIndex = stamp();
        push({
          kind: 'row',
          docIdx,
          hunkIdx,
          row,
          rowKind: kind,
          // Only border where a run meets context or a hunk edge — not where
          // a deleted run and an added run "kiss" directly (the common
          // replace-a-line case), which would double up on a shared edge.
          runStart: prevKind === null || prevKind === 'context',
          runEnd: nextKind === null || nextKind === 'context',
          conflict,
          displayIndex,
        });
        rowStart ??= displayIndex;
        rowEnd = displayIndex;

        if (row.old_line !== null) byEndpoint.set(diffKey(doc.path, 'old', row.old_line), displayIndex);
        if (row.new_line !== null) byEndpoint.set(diffKey(doc.path, 'new', row.new_line), displayIndex);
      });
      hunkViews.push({
        headerDisplayIndex: headerIdx,
        rowStart: rowStart ?? headerIdx + 1,
        rowEnd,
        gapAbove: gapAbove(doc, hunkIdx),
      });
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
      endDisplayIndex: pos,
      hunks: hunkViews,
      trailingGap: trailingGap(doc),
    });
  });

  return { rows, docs: docViews, byIndex, byEndpoint };
}

/** A split-view cell: one walk row rendered in one column. */
export type SplitCell = Extract<DisplayRow, { kind: 'row' }>;

/**
 * One entry of the split-view render sequence: headers pass through
 * full-width; rows become column pairs. A null cell is filler — the shorter
 * side of an uneven change run.
 */
export type SplitEntry =
  | Exclude<DisplayRow, { kind: 'row' }>
  | { kind: 'pair'; old: SplitCell | null; new: SplitCell | null };

/**
 * Project a document's walk entries into split-view pairs — a pure
 * re-arrangement of the same DisplayRows, no new index space. Context rows
 * span both columns (one displayIndex, two cells); a change run pairs its
 * deletions and additions by index. Runs never cross context or headers, so
 * pairing is hunk-local by construction.
 */
export function pairHunkRows(entries: DisplayRow[]): SplitEntry[] {
  const out: SplitEntry[] = [];
  let dels: SplitCell[] = [];
  let adds: SplitCell[] = [];

  const flush = () => {
    const n = Math.max(dels.length, adds.length);
    for (let i = 0; i < n; i++) {
      out.push({ kind: 'pair', old: dels[i] ?? null, new: adds[i] ?? null });
    }
    dels = [];
    adds = [];
  };

  for (const entry of entries) {
    if (entry.kind !== 'row') {
      flush();
      out.push(entry);
    } else if (entry.rowKind === 'context') {
      flush();
      out.push({ kind: 'pair', old: entry, new: entry });
    } else if (entry.rowKind === 'deleted') {
      dels.push(entry);
    } else {
      adds.push(entry);
    }
  }
  flush();
  return out;
}

/**
 * A row's anchor endpoint. Side-scoped selections (split view) anchor on the
 * scoped side — the filter guarantees the row has a line there. Otherwise:
 * new-side for added/context rows, old-side for deleted.
 */
function rowEndpoint(row: Row, side: Side | null): Endpoint {
  if (side === 'old' && row.old_line !== null) return { side: 'old', line: row.old_line };
  if (side === 'new' && row.new_line !== null) return { side: 'new', line: row.new_line };
  return row.new_line !== null
    ? { side: 'new', line: row.new_line }
    : { side: 'old', line: row.old_line! };
}

/**
 * Convert a display selection into a diff anchor. Valid only when every
 * index in the range is a row of the same document — headers and
 * cross-document spans are not annotatable.
 *
 * `side` scopes the selection to one split-view column: rows absent from
 * that side (the opposite column's half of a change run) are skipped, not
 * rejected — a column drag is non-contiguous in display space. Both
 * endpoints then anchor on the scoped side, so split drags always produce
 * single-side anchors; mixed-side ranges stay a unified-view gesture.
 */
export function selectionToDiffAnchor(range: Range, display: DiffDisplay, side: Side | null = null): Anchor | null {
  const min = Math.min(range.start, range.end);
  const max = Math.max(range.start, range.end);

  const entries: Extract<DisplayRow, { kind: 'row' }>[] = [];
  for (let i = min; i <= max; i++) {
    const entry = display.byIndex.get(i);
    if (!entry || entry.kind !== 'row') return null;
    if (side === 'old' && entry.row.old_line === null) continue;
    if (side === 'new' && entry.row.new_line === null) continue;
    entries.push(entry);
  }
  if (entries.length === 0 || entries.some((e) => e.docIdx !== entries[0].docIdx)) return null;

  const startPoint = rowEndpoint(entries[0].row, side);
  const endPoint = rowEndpoint(entries[entries.length - 1].row, side);

  // Display order can number in reverse of source order within a hunk (old vs
  // new numbering) — swap the pair as a unit so each endpoint's line and side
  // stay paired together.
  const [lo, hi] = endPoint.line < startPoint.line ? [endPoint, startPoint] : [startPoint, endPoint];

  return { type: 'diff', path: display.docs[entries[0].docIdx].path, start: lo, end: hi };
}
