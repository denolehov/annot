import { describe, it, expect } from 'vitest';
import { injectColorSwatches, clearColorSwatches } from './color-preview';

function el(html: string): HTMLElement {
  const div = document.createElement('div');
  div.innerHTML = html;
  return div;
}

describe('color-preview', () => {
  it('injects a swatch before a hex value and clears it back to the original text', () => {
    const container = el('<span>color: #ff0000;</span>');
    injectColorSwatches(container);
    expect(container.querySelectorAll('.color-swatch')).toHaveLength(1);

    clearColorSwatches(container);
    expect(container.querySelectorAll('.color-swatch')).toHaveLength(0);
    expect(container.textContent).toBe('color: #ff0000;');
    // The split is re-joined into a single text node.
    expect(container.querySelector('span')!.childNodes).toHaveLength(1);
  });

  it('handles a hex value at the start of a text node without leaving empty nodes', () => {
    const container = el('<span>#00ff00 is green</span>');
    injectColorSwatches(container);
    expect(container.querySelectorAll('.color-swatch')).toHaveLength(1);

    clearColorSwatches(container);
    expect(container.textContent).toBe('#00ff00 is green');
    expect(container.querySelector('span')!.childNodes).toHaveLength(1);
  });

  // Svelte anchors {@html} and branch effects on empty text nodes; clearing
  // swatches must never delete them (container.normalize() did — reused diff
  // rows then re-rendered into a detached anchor and went blank).
  it('preserves empty text nodes when clearing', () => {
    const container = el('<span>#abc</span>');
    const anchor = document.createTextNode('');
    container.appendChild(anchor);

    injectColorSwatches(container);
    clearColorSwatches(container);

    expect(anchor.parentNode).toBe(container);
    expect(container.textContent).toBe('#abc');
  });
});
