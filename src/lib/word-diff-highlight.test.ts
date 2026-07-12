import { describe, it, expect, beforeEach, vi } from 'vitest';
import { paintWordDiff } from './word-diff-highlight';

// jsdom has no CSS Custom Highlight API — shim the registry surface the
// painter touches (add/delete + the named-highlight map).
class HighlightShim {
  ranges = new Set<Range>();
  add(range: Range) {
    this.ranges.add(range);
  }
  delete(range: Range) {
    this.ranges.delete(range);
  }
}

vi.stubGlobal('Highlight', HighlightShim);

function registry(): Map<string, HighlightShim> {
  return (CSS as unknown as { highlights: Map<string, HighlightShim> }).highlights;
}

function rangesOf(name: string): Range[] {
  return [...(registry().get(name)?.ranges ?? [])];
}

function el(html: string): HTMLElement {
  const span = document.createElement('span');
  span.innerHTML = html;
  return span;
}

beforeEach(() => {
  (CSS as unknown as { highlights: Map<string, HighlightShim> }).highlights = new Map();
});

describe('paintWordDiff', () => {
  it('paints a range inside a single syntect span', () => {
    const code = el('<span class="k">let</span> <span class="w">foo</span>');
    paintWordDiff(code, { ranges: [{ start: 4, end: 7 }], side: 'add' });

    const [range] = rangesOf('word-diff-add');
    expect(range.toString()).toBe('foo');
    // First paint installs the runtime ::highlight() stylesheet (kept out of
    // code-viewer.css — lightningcss warns on the pseudo-element).
    expect(document.getElementById('word-diff-highlight-styles')).not.toBeNull();
  });

  it('spans syntect span boundaries in one range', () => {
    const code = el('<span class="k">let</span> <span class="w">foo</span> = 1');
    paintWordDiff(code, { ranges: [{ start: 2, end: 9 }], side: 'del' });

    const [range] = rangesOf('word-diff-del');
    expect(range.toString()).toBe('t foo =');
  });

  it('addresses UTF-16 code units, matching the wire encoding', () => {
    // 😀 is one char, two UTF-16 code units — offsets past it must not drift.
    const code = el('a😀<span class="w">bc</span>');
    paintWordDiff(code, { ranges: [{ start: 3, end: 5 }], side: 'add' });

    const [range] = rangesOf('word-diff-add');
    expect(range.toString()).toBe('bc');
  });

  it('paints plain-content rows (no spans at all)', () => {
    const code = el('plain changed text');
    paintWordDiff(code, { ranges: [{ start: 6, end: 13 }], side: 'add' });

    expect(rangesOf('word-diff-add').map(String)).toEqual(['changed']);
  });

  it('skips ranges that fall outside the mounted text', () => {
    const code = el('short');
    paintWordDiff(code, {
      ranges: [
        { start: 0, end: 5 },
        { start: 10, end: 20 },
      ],
      side: 'del',
    });

    expect(rangesOf('word-diff-del').map(String)).toEqual(['short']);
  });

  it('withdraws its ranges on update and destroy', () => {
    const code = el('one two');
    const action = paintWordDiff(code, { ranges: [{ start: 0, end: 3 }], side: 'add' });

    action!.update!({ ranges: [{ start: 4, end: 7 }], side: 'del' });
    expect(rangesOf('word-diff-add')).toEqual([]);
    expect(rangesOf('word-diff-del').map(String)).toEqual(['two']);

    action!.destroy!();
    expect(rangesOf('word-diff-del')).toEqual([]);
  });

  it('no-ops on context rows (no ranges)', () => {
    const code = el('unchanged');
    const action = paintWordDiff(code, { ranges: undefined, side: 'add' });
    expect(registry().get('word-diff-add')?.ranges.size ?? 0).toBe(0);
    action?.destroy?.();
  });
});
