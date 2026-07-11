import { describe, it, expect } from 'vitest';
import { deriveDisplay, hunkHeaderText, rowKind, selectionToDiffAnchor } from './display-rows';
import { diffKey, anchorKeys } from './anchor';
import type { DiffDocument, Row } from './types';

// =============================================================================
// Wire-shaped fixture: per-file documents exactly as ContentView::Diff
// serializes them — raw row content, git-printed ranges, hunk-owned rows.
// Covers every producer shape: modified (two hunks + function context),
// deleted, renamed, binary (unavailable, no hunks), and added.
// =============================================================================

function row(old: number | null, new_: number | null, content: string): Row {
  return { old_line: old, new_line: new_, content, html: null };
}

const DOCS: DiffDocument[] = [
  {
    path: 'src/main.rs',
    old_path: null,
    status: 'modified',
    unavailable: false,
    new_len: null,
    language: 'rs',
    hunks: [
      {
        old_range: { start: 1, end: 4 },
        new_range: { start: 1, end: 5 },
        function_context: 'fn main()',
        function_context_html: '<span>fn main()</span>',
        rows: [
          row(1, 1, 'fn main() {'),
          row(2, null, '    old_call();'),
          row(null, 2, '    new_call();'),
          row(null, 3, '    extra();'),
          row(3, 4, '}'),
        ],
      },
      {
        old_range: { start: 10, end: 11 },
        new_range: { start: 11, end: 12 },
        function_context: null,
        function_context_html: null,
        rows: [row(10, null, 'const A: u8 = 1;'), row(null, 11, 'const A: u8 = 2;')],
      },
    ],
  },
  {
    path: 'src/gone.rs',
    old_path: null,
    status: 'deleted',
    unavailable: false,
    new_len: null,
    language: 'rs',
    hunks: [
      {
        old_range: { start: 1, end: 3 },
        new_range: { start: 0, end: 0 },
        function_context: null,
        function_context_html: null,
        rows: [row(1, null, 'pub fn gone() {}'), row(2, null, '')],
      },
    ],
  },
  {
    path: 'new/name.rs',
    old_path: 'old/name.rs',
    status: 'renamed',
    unavailable: false,
    new_len: null,
    language: 'rs',
    hunks: [
      {
        old_range: { start: 5, end: 6 },
        new_range: { start: 5, end: 6 },
        function_context: null,
        function_context_html: null,
        rows: [row(5, null, 'a'), row(null, 5, 'b')],
      },
    ],
  },
  {
    path: 'logo.png',
    old_path: null,
    status: 'modified',
    unavailable: true,
    new_len: null,
    language: 'png',
    hunks: [],
  },
  {
    path: 'docs/new.md',
    old_path: null,
    status: 'added',
    unavailable: false,
    new_len: null,
    language: 'md',
    hunks: [
      {
        old_range: { start: 0, end: 0 },
        new_range: { start: 1, end: 3 },
        function_context: null,
        function_context_html: null,
        rows: [row(null, 1, '# Title'), row(null, 2, 'Body')],
      },
    ],
  },
];

const display = deriveDisplay(DOCS);

// Display layout: src/main.rs header 1, hunk 2, rows 3–7, hunk 8, rows 9–10.
// src/gone.rs: header 11, hunk 12, rows 13–14. new/name.rs: header 15,
// hunk 16, rows 17–18. logo.png: header 19. docs/new.md: header 20, hunk 21,
// rows 22–23.

describe('deriveDisplay', () => {
  it('stamps dense 1..n display indexes over the total walk', () => {
    const headerCount = DOCS.length;
    const hunkCount = DOCS.reduce((sum, d) => sum + d.hunks.length, 0);
    const rowCount = DOCS.reduce(
      (sum, d) => sum + d.hunks.reduce((s, h) => s + h.rows.length, 0),
      0,
    );

    expect(display.rows).toHaveLength(headerCount + hunkCount + rowCount);
    expect(display.rows.map((r) => r.displayIndex)).toEqual(display.rows.map((_, i) => i + 1));
  });

  it('DocViews carry identity, counts, and contiguous spans', () => {
    const main = display.docs[0];
    expect(main.dir).toBe('src/');
    expect(main.name).toBe('main.rs');
    expect(main.added).toBe(3);
    expect(main.deleted).toBe(2);
    expect(main.headerDisplayIndex).toBe(1);
    expect(main.endDisplayIndex).toBe(10);

    display.docs.forEach((dv, i) => {
      const next = display.docs[i + 1];
      expect(dv.headerDisplayIndex).toBeLessThanOrEqual(dv.endDisplayIndex);
      if (next) expect(next.headerDisplayIndex).toBe(dv.endDisplayIndex + 1);
    });

    // Binary doc: header only, zero counts.
    const binary = display.docs[3];
    expect(binary.doc.unavailable).toBe(true);
    expect(binary.headerDisplayIndex).toBe(binary.endDisplayIndex);
    expect(binary.added + binary.deleted).toBe(0);
  });

  it('hunk footprints span exactly the hunk rows', () => {
    display.docs.forEach((dv) => {
      dv.hunks.forEach((hv, hi) => {
        expect(hv.rowStart).toBe(hv.headerDisplayIndex + 1);
        expect(hv.rowEnd).toBe(hv.rowStart + dv.doc.hunks[hi].rows.length - 1);
      });
    });
  });

  it('byIndex and byEndpoint resolve selection and anchor lookups', () => {
    // context row registers both sides
    expect(display.byEndpoint.get(diffKey('src/main.rs', 'old', 1))).toBe(
      display.byEndpoint.get(diffKey('src/main.rs', 'new', 1)),
    );
    // deleted-file rows resolve old-side under the display path
    const goneIdx = display.byEndpoint.get(diffKey('src/gone.rs', 'old', 1))!;
    const goneEntry = display.byIndex.get(goneIdx)!;
    expect(goneEntry.kind).toBe('row');
    if (goneEntry.kind === 'row') expect(goneEntry.rowKind).toBe('deleted');
  });
});

describe('gap derivation (S3 unfold)', () => {
  it('emits no gaps when new_len is null — the capability signal', () => {
    // The whole fixture is raw-patch shaped (new_len: null everywhere).
    display.docs.forEach((dv) => {
      dv.hunks.forEach((hv) => expect(hv.gapAbove).toBe(0));
      expect(dv.trailingGap).toBe(0);
    });
  });

  it('derives leading, interior, and trailing gaps from range arithmetic', () => {
    const doc: DiffDocument = {
      ...DOCS[0],
      new_len: 100,
      hunks: [
        { ...DOCS[0].hunks[0], old_range: { start: 7, end: 14 }, new_range: { start: 7, end: 14 } },
        { ...DOCS[0].hunks[1], old_range: { start: 57, end: 64 }, new_range: { start: 57, end: 64 } },
      ],
    };
    const [dv] = deriveDisplay([doc]).docs;
    expect(dv.hunks[0].gapAbove).toBe(6); // lines 1–6 folded
    expect(dv.hunks[1].gapAbove).toBe(43); // lines 14–56 folded
    expect(dv.trailingGap).toBe(37); // lines 64–100 folded
  });

  it('handles printed-empty ranges: a pure-deletion hunk sits at its insertion point', () => {
    // Deletion between new lines 10 and 11: printed new_range 10..10,
    // true position 11..11 — the gap after it starts at new line 11.
    const doc: DiffDocument = {
      ...DOCS[1],
      new_len: 30,
      hunks: [
        {
          ...DOCS[1].hunks[0],
          old_range: { start: 11, end: 13 },
          new_range: { start: 10, end: 10 },
        },
      ],
    };
    const [dv] = deriveDisplay([doc]).docs;
    expect(dv.hunks[0].gapAbove).toBe(10); // new lines 1–10 folded above
    expect(dv.trailingGap).toBe(20); // new lines 11–30 folded below
  });

  it('an added file whose hunk covers everything has no gaps', () => {
    const doc: DiffDocument = { ...DOCS[4], new_len: 2 };
    const [dv] = deriveDisplay([doc]).docs;
    expect(dv.hunks[0].gapAbove).toBe(0);
    expect(dv.trailingGap).toBe(0);
  });
});

describe('presentation helpers', () => {
  it('rowKind derives from the line-number pattern', () => {
    expect(rowKind(row(1, 1, ''))).toBe('context');
    expect(rowKind(row(1, null, ''))).toBe('deleted');
    expect(rowKind(row(null, 1, ''))).toBe('added');
  });

  it('hunkHeaderText reproduces git @@ headers from printed ranges', () => {
    expect(hunkHeaderText(DOCS[0].hunks[0])).toBe('@@ -1,3 +1,4 @@ fn main()');
    expect(hunkHeaderText(DOCS[0].hunks[1])).toBe('@@ -10 +11 @@');
    expect(hunkHeaderText(DOCS[4].hunks[0])).toBe('@@ -0,0 +1,2 @@');
  });
});

describe('selectionToDiffAnchor', () => {
  it('builds a mixed-side anchor over a replacement span', () => {
    expect(selectionToDiffAnchor({ start: 4, end: 6 }, display)).toEqual({
      type: 'diff',
      path: 'src/main.rs',
      start: { side: 'old', line: 2 },
      end: { side: 'new', line: 3 },
    });
  });

  it('anchors context spans new-side, mixing removed and added rows', () => {
    expect(selectionToDiffAnchor({ start: 3, end: 7 }, display)).toEqual({
      type: 'diff',
      path: 'src/main.rs',
      start: { side: 'new', line: 1 },
      end: { side: 'new', line: 4 },
    });
  });

  it('anchors deleted-file rows old-side under the display path', () => {
    expect(selectionToDiffAnchor({ start: 13, end: 14 }, display)).toEqual({
      type: 'diff',
      path: 'src/gone.rs',
      start: { side: 'old', line: 1 },
      end: { side: 'old', line: 2 },
    });
  });

  it('rejects spans touching headers or crossing documents', () => {
    expect(selectionToDiffAnchor({ start: 2, end: 3 }, display)).toBeNull(); // hunk header
    expect(selectionToDiffAnchor({ start: 10, end: 13 }, display)).toBeNull(); // crosses files
  });

  it('round-trips through byEndpoint with anchorKeys', () => {
    const anchor = selectionToDiffAnchor({ start: 4, end: 6 }, display)!;
    const [startKey, endKey] = anchorKeys(anchor);
    expect(display.byEndpoint.get(startKey)).toBe(4);
    expect(display.byEndpoint.get(endKey)).toBe(6);
  });
});
