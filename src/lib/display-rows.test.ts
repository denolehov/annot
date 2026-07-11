import { describe, it, expect } from 'vitest';
import {
  synthesizeDocs,
  deriveDisplay,
  endpointKey,
  type PseudoDoc,
} from './display-rows';
import { deriveFileEntries } from './file-tree';
import { groupByFile } from './file-collapse';
import type { DisplayLine } from './composables/useLineSegments.svelte';
import type { DiffFileInfo, HunkInfo, Line } from './types';

// =============================================================================
// Wire-faithful fixture builder — mirrors the backend's flatten_file /
// flatten_hunk emission (diff.rs) and render_file (pipeline.rs): per file a
// FileHeader line, an optional binary Meta line, then per hunk a HunkHeader
// line followed by prefixed rows.
//
// The equivalence proof below pins the walk to the old projections
// (deriveFileEntries, groupByFile, wire display indexes) over this shape.
// It dies together with synthesizeDocs when the wire itself goes per-file.
// =============================================================================

interface FixtureRow {
  old: number | null;
  new: number | null;
  text: string;
}

interface FixtureHunk {
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
  ctx?: string;
  rows: FixtureRow[];
}

interface FixtureFile {
  oldName: string | null;
  newName: string | null;
  language?: string;
  binary?: boolean;
  hunks: FixtureHunk[];
}

/** Git omits the count when it is 1: `-3` not `-3,1`. */
function printedSide(sign: string, start: number, count: number): string {
  return count === 1 ? `${sign}${start}` : `${sign}${start},${count}`;
}

function rowPrefix(row: FixtureRow): string {
  if (row.old === null) return '+';
  if (row.new === null) return '-';
  return ' ';
}

function buildWire(files: FixtureFile[]): { lines: Line[]; files: DiffFileInfo[] } {
  const lines: Line[] = [];
  const infos: DiffFileInfo[] = [];

  for (const file of files) {
    const displayPath = file.newName ?? file.oldName ?? '';
    const startLine = lines.length + 1;
    const headerOrigin = { type: 'diff', path: displayPath, old_line: null, new_line: null } as const;

    lines.push({
      content: `diff --git a/${file.oldName ?? '/dev/null'} b/${file.newName ?? '/dev/null'}`,
      html: null,
      origin: headerOrigin,
      semantics: { type: 'diff', kind: 'file_header' },
    });

    if (file.binary) {
      lines.push({
        content: `Binary files a/${file.oldName} and b/${file.newName} differ`,
        html: null,
        origin: headerOrigin,
        semantics: { type: 'diff', kind: 'meta' },
      });
    }

    const hunks: HunkInfo[] = [];
    for (const hunk of file.hunks) {
      const marker = `@@ ${printedSide('-', hunk.oldStart, hunk.oldCount)} ${printedSide('+', hunk.newStart, hunk.newCount)} @@`;
      lines.push({
        content: hunk.ctx ? `${marker} ${hunk.ctx}` : marker,
        html: null,
        origin: headerOrigin,
        semantics: { type: 'diff', kind: 'hunk_header', context: hunk.ctx ?? null },
      });
      const displayLine = lines.length;

      for (const row of hunk.rows) {
        const prefix = rowPrefix(row);
        lines.push({
          content: `${prefix}${row.text}`,
          html: { type: 'full', value: `<span>${prefix}${row.text}</span>` },
          origin: { type: 'diff', path: displayPath, old_line: row.old, new_line: row.new },
          semantics: {
            type: 'diff',
            kind: row.old === null ? 'added' : row.new === null ? 'deleted' : 'context',
          },
        });
      }

      hunks.push({
        display_line: displayLine,
        old_start: hunk.oldStart,
        old_count: hunk.oldCount,
        new_start: hunk.newStart,
        new_count: hunk.newCount,
        function_context: hunk.ctx ?? null,
        function_context_html: hunk.ctx ? `<span>${hunk.ctx}</span>` : null,
      });
    }

    infos.push({
      old_name: file.oldName,
      new_name: file.newName,
      language: file.language ?? 'rs',
      start_line: startLine,
      end_line: lines.length,
      hunks,
    });
  }

  return { lines, files: infos };
}

function toDisplay(lines: Line[]): DisplayLine[] {
  return lines.map((line, i) => ({ line, displayIndex: i + 1 }));
}

/** Multi-file fixture: modified (two hunks + function context), deleted,
 *  renamed, binary, and added — every producer shape in one wire. */
const FIXTURE: FixtureFile[] = [
  {
    oldName: 'src/main.rs',
    newName: 'src/main.rs',
    hunks: [
      {
        oldStart: 1,
        oldCount: 3,
        newStart: 1,
        newCount: 4,
        ctx: 'fn main()',
        rows: [
          { old: 1, new: 1, text: 'fn main() {' },
          { old: 2, new: null, text: '    old_call();' },
          { old: null, new: 2, text: '    new_call();' },
          { old: null, new: 3, text: '    extra();' },
          { old: 3, new: 4, text: '}' },
        ],
      },
      {
        oldStart: 10,
        oldCount: 1,
        newStart: 11,
        newCount: 1,
        rows: [
          { old: 10, new: null, text: 'const A: u8 = 1;' },
          { old: null, new: 11, text: 'const A: u8 = 2;' },
        ],
      },
    ],
  },
  {
    oldName: 'src/gone.rs',
    newName: null,
    hunks: [
      {
        oldStart: 1,
        oldCount: 2,
        newStart: 0,
        newCount: 0,
        rows: [
          { old: 1, new: null, text: 'pub fn gone() {}' },
          { old: 2, new: null, text: '' },
        ],
      },
    ],
  },
  {
    oldName: 'old/name.rs',
    newName: 'new/name.rs',
    hunks: [
      {
        oldStart: 5,
        oldCount: 1,
        newStart: 5,
        newCount: 1,
        rows: [
          { old: 5, new: null, text: 'a' },
          { old: null, new: 5, text: 'b' },
        ],
      },
    ],
  },
  { oldName: 'logo.png', newName: 'logo.png', binary: true, hunks: [] },
  {
    oldName: null,
    newName: 'docs/new.md',
    language: 'md',
    hunks: [
      {
        oldStart: 0,
        oldCount: 0,
        newStart: 1,
        newCount: 2,
        rows: [
          { old: null, new: 1, text: '# Title' },
          { old: null, new: 2, text: 'Body' },
        ],
      },
    ],
  },
];

const wire = buildWire(FIXTURE);
const docs = synthesizeDocs(wire.lines, wire.files);
const display = deriveDisplay(docs);

describe('equivalence: walk ≡ old projections over the flat wire', () => {
  it('DocViews reproduce deriveFileEntries exactly', () => {
    const entries = deriveFileEntries(wire.lines, { files: wire.files });

    expect(
      display.docs.map((dv) => ({
        index: dv.index,
        path: dv.path,
        dir: dv.dir,
        name: dv.name,
        added: dv.added,
        deleted: dv.deleted,
        startLine: dv.headerDisplayIndex,
        endLine: dv.endDisplayIndex,
      })),
    ).toEqual(entries);
  });

  it('walk sections reproduce groupByFile (headers separated, meta excluded)', () => {
    const entries = deriveFileEntries(wire.lines, { files: wire.files });
    const grouped = groupByFile(toDisplay(wire.lines), entries)!;

    expect(grouped.leading).toEqual([]);
    expect(display.docs).toHaveLength(grouped.sections.length);

    display.docs.forEach((dv, i) => {
      const section = grouped.sections[i];
      expect(dv.headerDisplayIndex).toBe(section.header.displayIndex);

      const walkBody = display.rows
        .filter((r) => r.kind !== 'file-header' && r.docIdx === dv.index)
        .map((r) => r.displayIndex);
      expect(walkBody).toEqual(section.body.map((dl) => dl.displayIndex));
    });
  });

  it('every entry sits at its wire display index with matching identity', () => {
    for (const entry of display.rows) {
      const line = wire.lines[entry.displayIndex - 1];
      expect(line).toBeDefined();
      if (line.semantics.type !== 'diff') throw new Error('non-diff line in diff wire');

      switch (entry.kind) {
        case 'file-header':
          expect(line.semantics.kind).toBe('file_header');
          break;
        case 'hunk-header':
          expect(line.semantics.kind).toBe('hunk_header');
          expect(docs[entry.docIdx].hunks[entry.hunkIdx].function_context).toEqual(
            line.semantics.kind === 'hunk_header' ? line.semantics.context : null,
          );
          break;
        case 'row':
          expect(entry.row.content).toBe(line.content);
          expect(entry.row.html).toEqual(line.html);
          expect(entry.rowKind).toBe(line.semantics.kind);
          if (line.origin.type === 'diff') {
            expect(entry.row.old_line).toBe(line.origin.old_line);
            expect(entry.row.new_line).toBe(line.origin.new_line);
          }
          break;
      }
    }
  });

  it('covers every wire line except meta plumbing, exactly once', () => {
    const metaCount = wire.lines.filter(
      (l) => l.semantics.type === 'diff' && l.semantics.kind === 'meta',
    ).length;

    expect(display.rows).toHaveLength(wire.lines.length - metaCount);
    expect(new Set(display.rows.map((r) => r.displayIndex)).size).toBe(display.rows.length);
  });

  it('byIndex and byEndpoint resolve selection and anchor lookups', () => {
    // context row registers both sides
    expect(display.byEndpoint.get(endpointKey('src/main.rs', 'old', 1))).toBe(
      display.byEndpoint.get(endpointKey('src/main.rs', 'new', 1)),
    );
    // deleted-file rows resolve old-side under the display path
    const goneIdx = display.byEndpoint.get(endpointKey('src/gone.rs', 'old', 1))!;
    const goneEntry = display.byIndex.get(goneIdx)!;
    expect(goneEntry.kind).toBe('row');
    if (goneEntry.kind === 'row') expect(goneEntry.rowKind).toBe('deleted');
  });

  it('promotes documents with stubs and real ranges', () => {
    expect(docs[0].hunks[0].old_range).toEqual({ start: 1, end: 4 });
    expect(docs[0].hunks[0].new_range).toEqual({ start: 1, end: 5 });
    expect(docs[0].status).toBe('modified');
    expect(docs[0].old_path).toBeNull();
    expect(docs[2].old_path).toBe('old/name.rs');
    expect(docs[2].path).toBe('new/name.rs');
    expect(docs[3].hunks).toEqual([]);
    expect(docs[1].path).toBe('src/gone.rs');
  });
});

describe('positional fallback (the future per-file wire input)', () => {
  it('stamps dense 1..n display indexes when wireIndex is absent', () => {
    const bare: PseudoDoc[] = docs.map((doc) => ({
      ...doc,
      wireIndex: undefined,
      endWireIndex: undefined,
      hunks: doc.hunks.map((hunk) => ({
        ...hunk,
        wireIndex: undefined,
        rows: hunk.rows.map((row) => ({ ...row, wireIndex: undefined })),
      })),
    }));

    const positional = deriveDisplay(bare);

    expect(positional.rows.map((r) => r.displayIndex)).toEqual(
      positional.rows.map((_, i) => i + 1),
    );
    expect(positional.docs.at(-1)!.endDisplayIndex).toBe(positional.rows.length);
  });

  it('keeps DocView spans consistent in positional space', () => {
    const bare: PseudoDoc[] = docs.map((doc) => ({
      ...doc,
      wireIndex: undefined,
      endWireIndex: undefined,
      hunks: doc.hunks.map((hunk) => ({
        ...hunk,
        wireIndex: undefined,
        rows: hunk.rows.map((row) => ({ ...row, wireIndex: undefined })),
      })),
    }));

    const positional = deriveDisplay(bare);

    positional.docs.forEach((dv, i) => {
      const next = positional.docs[i + 1];
      expect(dv.headerDisplayIndex).toBeLessThanOrEqual(dv.endDisplayIndex);
      if (next) expect(next.headerDisplayIndex).toBe(dv.endDisplayIndex + 1);
    });
  });
});
