import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import RegularLines from './RegularLines.svelte';
import { makeStore } from './regular-lines-test-store.svelte';
import type { DiffDocument, Row } from '$lib/types';

let store: ReturnType<typeof makeStore>;

// Shared across getAnnotContext() calls so tests can assert against the same
// fn instances LineRow wires up (vi.mock factories are hoisted).
const shared = vi.hoisted(() => ({
  interaction: {
    isCellHighlighted: () => false,
    handleLineEnter: vi.fn(),
    handleLineLeave: vi.fn(),
    handlePointerDown: vi.fn(),
    handleGutterClick: vi.fn(),
  },
  expandContext: vi.fn(() => Promise.resolve()),
  view: { mode: 'unified' as 'unified' | 'split' },
}));

vi.mock('$lib/context', () => ({
  getAnnotContext: () => ({
    get diffDisplay() {
      return store.display;
    },
    get diffView() {
      return shared.view.mode;
    },
    markdownMetadata: null,
    interaction: shared.interaction,
    annotations: { hasAnnotation: () => false },
    search: { matches: [], getCurrentMatch: () => null },
    mermaid: { getMermaidBlockAt: () => null, openMermaidWindow: vi.fn() },
    fileCollapse: { isCollapsed: () => false, toggle: vi.fn() },
    slotForRow: () => null,
    expandContext: shared.expandContext,
  }),
}));

function row(oldLine: number | null, newLine: number | null, content: string): Row {
  return {
    old_line: oldLine,
    new_line: newLine,
    content,
    html: content === '' ? null : { type: 'full', value: `<span class="x">${content}</span>` },
  };
}

function ctx(n: number): Row {
  return row(n, n, `line ${n}`);
}

function doc(hunks: DiffDocument['hunks']): DiffDocument {
  return {
    path: 'src/big.rs',
    old_path: null,
    status: 'modified',
    unavailable: false,
    conflicted: false,
    new_len: 100,
    language: 'rs',
    hunks,
  };
}

function hunk(rows: Row[]): DiffDocument['hunks'][number] {
  const olds = rows.map((r) => r.old_line).filter((n): n is number => n !== null);
  const news = rows.map((r) => r.new_line).filter((n): n is number => n !== null);
  return {
    old_range: { start: olds[0], end: olds[olds.length - 1] + 1 },
    new_range: { start: news[0], end: news[news.length - 1] + 1 },
    function_context: null,
    function_context_html: null,
    rows,
  };
}

/** Every row's rendered code text must match its `content`. */
function assertAllRowsRendered(container: HTMLElement) {
  const failures: string[] = [];
  for (const entry of store.display.rows) {
    if (entry.kind !== 'row') continue;
    const el = container.querySelector(`[data-display-idx="${entry.displayIndex}"] .code`);
    const text = el?.textContent ?? '<missing>';
    if (text !== entry.row.content) {
      failures.push(
        `idx ${entry.displayIndex}: expected ${JSON.stringify(entry.row.content)}, got ${JSON.stringify(text)}`,
      );
    }
  }
  expect(failures).toEqual([]);
}

describe('RegularLines unfold re-render', () => {
  // Regression: the color-swatch effect used to call container.normalize(),
  // which deleted the empty text nodes Svelte anchors {@html} on. The next
  // document replacement (unfold merging hunks) then re-rendered reused rows
  // into a detached anchor — the rows went blank while their gutters updated.
  // The microtask flush between renders is what arms the bug: the swatch
  // effect runs via queueMicrotask.
  it('re-renders every row after hunks merge, with DOM effects flushed between', async () => {
    // Before: hunk A (rows 1-3, line 2 changed), hunk B (rows 40-42).
    const before = doc([
      hunk([ctx(1), row(2, null, 'old 2'), row(null, 2, 'new 2'), ctx(3)]),
      hunk([ctx(40), row(41, null, 'old 41'), row(null, 41, 'new 41'), ctx(42)]),
    ]);
    // After: the single merged hunk covering 1..42 that expand-all produces.
    const rows: Row[] = [ctx(1), row(2, null, 'old 2'), row(null, 2, 'new 2')];
    for (let n = 3; n <= 40; n++) rows.push(ctx(n));
    rows.push(row(41, null, 'old 41'), row(null, 41, 'new 41'), ctx(42));
    const after = doc([hunk(rows)]);

    store = makeStore([before]);
    const { container } = render(RegularLines, {
      props: { annotationSlotProps: {} as never },
    });
    flushSync();
    await new Promise((r) => setTimeout(r, 0)); // run queueMicrotask effects

    store.replace(0, after);
    flushSync();
    await new Promise((r) => setTimeout(r, 0));

    assertAllRowsRendered(container);
  });
});

describe('RegularLines unfold affordance placement', () => {
  it('inlines chevrons into the @@ header gutter; only the trailing gap keeps a standalone row', () => {
    // hunk A starts at line 1 (no gap above); hunk B has a 36-line gap above;
    // 58 folded lines trail the last hunk (new_len 100).
    store = makeStore([
      doc([
        hunk([ctx(1), row(2, null, 'old 2'), row(null, 2, 'new 2'), ctx(3)]),
        hunk([ctx(40), row(41, null, 'old 41'), row(null, 41, 'new 41'), ctx(42)]),
      ]),
    ]);
    const { container } = render(RegularLines, {
      props: { annotationSlotProps: {} as never },
    });
    flushSync();

    // Trailing gap row also carries 'diff-header' (shared styling only, see
    // TrailingGapRow.svelte) — exclude it to count actual @@ rows.
    const headers = [...container.querySelectorAll('.line.diff-header:not(.gap-line)')];
    expect(headers).toHaveLength(2);
    // No gap above hunk A: empty slots. Gap above hunk B: chevrons in-gutter.
    expect(headers[0].querySelector('.unfold-controls')).toBeNull();
    expect(headers[1].querySelector('.gutter .unfold-controls')).not.toBeNull();

    // The only standalone gap row is the trailing one — non-interactive,
    // hosting its own chevron cluster.
    const gapRows = [...container.querySelectorAll('.line.gap-line')];
    expect(gapRows).toHaveLength(1);
    expect(gapRows[0].hasAttribute('data-display-idx')).toBe(false);
    expect(gapRows[0].querySelector('.unfold-controls')).not.toBeNull();
    expect(gapRows[0].compareDocumentPosition(headers[1]) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
  });

  it('unfolds on chevron click without selecting the @@ row', () => {
    shared.interaction.handleGutterClick.mockClear();
    shared.interaction.handlePointerDown.mockClear();
    shared.expandContext.mockClear();
    store = makeStore([
      doc([
        hunk([ctx(1), row(2, null, 'old 2'), row(null, 2, 'new 2'), ctx(3)]),
        hunk([ctx(40), row(41, null, 'old 41'), row(null, 41, 'new 41'), ctx(42)]),
      ]),
    ]);
    const { container } = render(RegularLines, {
      props: { annotationSlotProps: {} as never },
    });
    flushSync();

    // ▲ in hunk B's header gutter: grows hunk B (idx 1) upward, one step.
    const upBtn = container.querySelector('.line.diff-header .unfold-controls button[title^="Expand up"]')!;
    upBtn.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    upBtn.dispatchEvent(new MouseEvent('click', { bubbles: true }));

    expect(shared.expandContext).toHaveBeenCalledWith(0, 1, 'up', 'step');
    expect(shared.interaction.handleGutterClick).not.toHaveBeenCalled();
    expect(shared.interaction.handlePointerDown).not.toHaveBeenCalled();
  });
});

describe('RegularLines split view', () => {
  it('pairs change runs into two data-side columns, filler on the shorter side', () => {
    shared.view.mode = 'split';
    try {
      // ctx(1), del(2), add(2), add(3), ctx(4) → 4 pairs: ctx | del+add | filler+add | ctx.
      store = makeStore([
        doc([hunk([ctx(1), row(2, null, 'old 2'), row(null, 2, 'new 2'), row(null, 3, 'extra'), ctx(4)])]),
      ]);
      const { container } = render(RegularLines, {
        props: { annotationSlotProps: {} as never },
      });
      flushSync();

      const pairs = [...container.querySelectorAll('.split-pair')];
      expect(pairs).toHaveLength(4);
      const cellText = (pair: Element, side: string) =>
        pair.querySelector(`[data-side="${side}"] .code`)?.textContent ?? null;

      // Context renders in both columns under one display index.
      expect(cellText(pairs[0], 'old')).toBe('line 1');
      expect(cellText(pairs[0], 'new')).toBe('line 1');
      expect(pairs[0].querySelectorAll('[data-display-idx]')).toHaveLength(2);

      // The change run pairs by index; the uneven third add faces filler.
      expect(cellText(pairs[1], 'old')).toBe('old 2');
      expect(cellText(pairs[1], 'new')).toBe('new 2');
      expect(cellText(pairs[2], 'old')).toBeNull();
      expect(pairs[2].querySelector('[data-side="old"] .diff-filler')).not.toBeNull();
      expect(cellText(pairs[2], 'new')).toBe('extra');

      // Filler is presentational: no display index, not selectable.
      expect(pairs[2].querySelector('[data-side="old"] [data-display-idx]')).toBeNull();

      // The @@ header still spans full width outside any pair.
      expect(container.querySelector('.split-pair .diff-header')).toBeNull();
      expect(container.querySelector('.line.diff-header')).not.toBeNull();
    } finally {
      shared.view.mode = 'unified';
    }
  });
});
