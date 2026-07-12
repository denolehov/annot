/**
 * Word-level diff highlights via the CSS Custom Highlight API.
 *
 * Paints the wire's `Row.word_ranges` (half-open UTF-16 code-unit offsets
 * into the row's text) as `::highlight(word-diff-add|del)` ranges over the
 * mounted row's text nodes — never by mutating the DOM (see dom-text.ts for
 * why post-hoc mutation is banned). Painting is provenance-blind: it works
 * the same over syntect HTML and plain-content rows, so it survives the
 * windowed-rendering campaign's raw-first styling unchanged.
 */
import type { Action } from 'svelte/action';

export type WordDiffSide = 'add' | 'del';

export interface WordDiffParams {
  /** UTF-16 code-unit offsets into the row's text; absent = nothing to paint. */
  ranges: { start: number; end: number }[] | undefined;
  side: WordDiffSide;
}

/** Lazy: the API is absent under jsdom (tests shim it) and old webviews. */
const supported = () =>
  typeof Highlight !== 'undefined' && typeof CSS !== 'undefined' && 'highlights' in CSS;

/**
 * `::highlight()` rules live here, not in code-viewer.css: lightningcss
 * (vite's CSS minifier) doesn't parse the pseudo-element yet and warns on
 * every build. Colors are the diff row palette (code-viewer.css
 * `diff-added`/`diff-deleted`, 0.1 row wash) at a stronger alpha.
 */
const STYLE_ID = 'word-diff-highlight-styles';
const STYLES = `
::highlight(word-diff-add) { background-color: rgba(34, 197, 94, 0.28); }
::highlight(word-diff-del) { background-color: rgba(239, 68, 68, 0.28); }
`;

function installStyles(): void {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = STYLES;
  document.head.append(style);
}

/** The per-side `Highlight` registered as `word-diff-add` / `word-diff-del`. */
function registryFor(side: WordDiffSide): Highlight {
  const name = `word-diff-${side}`;
  let highlight = CSS.highlights.get(name);
  if (!highlight) {
    installStyles();
    highlight = new Highlight();
    CSS.highlights.set(name, highlight);
  }
  return highlight;
}

interface TextNodeInfo {
  node: Text;
  start: number; // cumulative UTF-16 offset in container
  end: number;
}

function getTextNodes(container: HTMLElement): TextNodeInfo[] {
  const nodes: TextNodeInfo[] = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  let offset = 0;
  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    nodes.push({ node, start: offset, end: offset + node.data.length });
    offset += node.data.length;
  }
  return nodes;
}

/** Boundary point for a container-wide offset; null when out of range. */
function locate(
  nodes: TextNodeInfo[],
  offset: number,
  kind: 'start' | 'end',
): { node: Text; offset: number } | null {
  for (const info of nodes) {
    if (info.start === info.end) continue; // empty Svelte anchor node
    // A start on a node boundary belongs to the next node, an end to the
    // previous — keeps both endpoints inside non-empty nodes.
    const within = kind === 'start' ? offset < info.end : offset <= info.end;
    if (within && offset >= info.start) {
      return { node: info.node, offset: offset - info.start };
    }
  }
  return null;
}

function buildRanges(
  container: HTMLElement,
  spans: { start: number; end: number }[],
): Range[] {
  const nodes = getTextNodes(container);
  const ranges: Range[] = [];
  for (const span of spans) {
    if (span.end <= span.start) continue;
    const start = locate(nodes, span.start, 'start');
    const end = locate(nodes, span.end, 'end');
    if (!start || !end) continue; // stale offsets for this DOM — skip, don't guess
    const range = document.createRange();
    range.setStart(start.node, start.offset);
    range.setEnd(end.node, end.offset);
    ranges.push(range);
  }
  return ranges;
}

/**
 * `use:paintWordDiff={{ ranges, side }}` on the row's `.code` element.
 * Registers ranges on mount/update, withdraws them on destroy — rows that
 * scroll out or get spliced away never leak highlights.
 */
export const paintWordDiff: Action<HTMLElement, WordDiffParams> = (el, params) => {
  if (!supported()) return;

  let owned: { range: Range; highlight: Highlight }[] = [];

  const clear = () => {
    for (const { range, highlight } of owned) {
      highlight.delete(range);
    }
    owned = [];
  };

  const apply = ({ ranges, side }: WordDiffParams) => {
    clear();
    if (!ranges?.length) return;
    const highlight = registryFor(side);
    owned = buildRanges(el, ranges).map((range) => {
      highlight.add(range);
      return { range, highlight };
    });
  };

  apply(params);
  return { update: apply, destroy: clear };
};
