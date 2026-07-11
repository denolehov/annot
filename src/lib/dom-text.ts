/**
 * Text-node surgery that is safe on Svelte-managed DOM.
 *
 * `Node.normalize()` is banned in this codebase (test-setup.ts enforces it):
 * Svelte anchors `{@html}` and branch effects on empty text nodes, and
 * normalize() deletes empty text nodes wholesale — the anchor detaches and
 * the next update renders into nothing (the line goes blank, gutter intact).
 *
 * When code splits a text node (splitText, Range.surroundContents), undo
 * that specific split with `mergeTextSplit` instead of normalizing the
 * whole subtree.
 */

/**
 * Merges `a` with its following sibling when both are non-empty text nodes.
 * Real splits are non-empty on both sides, so this re-joins them while
 * never touching an empty anchor node.
 */
export function mergeTextSplit(a: Node | null | undefined): void {
  const b = a?.nextSibling;
  if (a instanceof Text && b instanceof Text && a.data !== '' && b.data !== '') {
    a.data += b.data;
    b.remove();
  }
}
