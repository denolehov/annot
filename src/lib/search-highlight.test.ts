import { describe, it, expect } from 'vitest';
import { highlightMatches, clearHighlights } from './search-highlight';

function el(html: string): HTMLElement {
  const div = document.createElement('div');
  div.innerHTML = html;
  return div;
}

describe('search-highlight', () => {
  it('wraps a match in <mark> and clears it back to the original text', () => {
    const container = el('<span>hello world</span>');
    highlightMatches(container, [{ start: 6, end: 11 }], null);
    expect(container.querySelector('mark.search-match')?.textContent).toBe('world');

    clearHighlights(container);
    expect(container.querySelector('mark')).toBeNull();
    expect(container.textContent).toBe('hello world');
    // The splits the range created are re-joined (jsdom may leave an empty
    // split artifact behind; only non-empty nodes matter for offsets).
    const nonEmpty = Array.from(container.querySelector('span')!.childNodes).filter(
      (n) => n.textContent !== '',
    );
    expect(nonEmpty).toHaveLength(1);
  });

  // Svelte anchors {@html} and branch effects on empty text nodes; clearing
  // highlights must never delete them (parent.normalize() did).
  it('preserves empty text nodes when clearing', () => {
    const container = el('<span>hello world</span>');
    const anchor = document.createTextNode('');
    container.appendChild(anchor);

    highlightMatches(container, [{ start: 0, end: 5 }], 0);
    expect(container.querySelector('mark.search-current')).not.toBeNull();

    clearHighlights(container);
    expect(anchor.parentNode).toBe(container);
    expect(container.textContent).toBe('hello world');
  });
});
