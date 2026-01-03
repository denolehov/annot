/**
 * DOM Range-based search highlighting.
 *
 * Uses browser's Range API to inject <mark> elements while preserving
 * existing HTML structure (syntax highlighting, etc).
 */

interface TextNodeInfo {
  node: Text;
  start: number; // cumulative char offset in container
  end: number;
}

/**
 * Collects all text nodes within a container, with their cumulative character offsets.
 * Skips elements with class "gutter-cell" or "gutter" to avoid offset misalignment in tables.
 */
function getTextNodes(container: HTMLElement): TextNodeInfo[] {
  const nodes: TextNodeInfo[] = [];
  const walker = document.createTreeWalker(
    container,
    NodeFilter.SHOW_TEXT,
    {
      acceptNode(node: Node): number {
        // Skip text nodes inside gutter elements
        let parent = node.parentElement;
        while (parent && parent !== container) {
          if (parent.classList.contains('gutter-cell') || parent.classList.contains('gutter')) {
            return NodeFilter.FILTER_REJECT;
          }
          parent = parent.parentElement;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    }
  );

  let offset = 0;
  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    const length = node.textContent?.length ?? 0;
    nodes.push({
      node,
      start: offset,
      end: offset + length,
    });
    offset += length;
  }

  return nodes;
}

/**
 * Maps a character offset to a specific text node and local offset within that node.
 */
function charOffsetToNode(
  nodes: TextNodeInfo[],
  charOffset: number
): { node: Text; offset: number } | null {
  for (const info of nodes) {
    if (charOffset >= info.start && charOffset <= info.end) {
      return {
        node: info.node,
        offset: charOffset - info.start,
      };
    }
  }
  return null;
}

/**
 * Highlights text ranges within a container element.
 * Uses Range.surroundContents for simple cases, falls back to manual DOM manipulation
 * for ranges that cross node boundaries.
 *
 * @param container - The element containing text to highlight
 * @param ranges - Character offset ranges to highlight
 * @param currentRangeIndex - Index of the "current" match (for different styling), or null
 */
export function highlightMatches(
  container: HTMLElement,
  ranges: Array<{ start: number; end: number }>,
  currentRangeIndex: number | null
): void {
  const textNodes = getTextNodes(container);
  if (textNodes.length === 0) return;

  // Process ranges in reverse order to avoid offset shifts affecting subsequent ranges
  const sortedRanges = [...ranges]
    .map((r, i) => ({ ...r, originalIndex: i }))
    .sort((a, b) => b.start - a.start);

  for (const range of sortedRanges) {
    const startPos = charOffsetToNode(textNodes, range.start);
    const endPos = charOffsetToNode(textNodes, range.end);

    if (!startPos || !endPos) continue;

    const isCurrent = range.originalIndex === currentRangeIndex;
    const className = isCurrent ? 'search-current' : 'search-match';

    try {
      const domRange = document.createRange();
      domRange.setStart(startPos.node, startPos.offset);
      domRange.setEnd(endPos.node, endPos.offset);

      // Check if range is within a single text node (simple case)
      if (startPos.node === endPos.node) {
        const mark = document.createElement('mark');
        mark.className = className;
        domRange.surroundContents(mark);
      } else {
        // Range spans multiple nodes - use extractContents + wrap
        const mark = document.createElement('mark');
        mark.className = className;
        const contents = domRange.extractContents();
        mark.appendChild(contents);
        domRange.insertNode(mark);
      }
    } catch {
      // surroundContents can fail if range partially selects a non-text node
      // Fall back to simpler highlighting approach
      console.warn('Failed to highlight range, skipping');
    }
  }
}

/**
 * Removes all search highlight marks from a container.
 */
export function clearHighlights(container: HTMLElement): void {
  const marks = container.querySelectorAll('mark.search-match, mark.search-current');
  marks.forEach((mark) => {
    const parent = mark.parentNode;
    if (parent) {
      // Replace mark with its contents
      while (mark.firstChild) {
        parent.insertBefore(mark.firstChild, mark);
      }
      parent.removeChild(mark);
      // Normalize to merge adjacent text nodes
      parent.normalize();
    }
  });
}

/**
 * Checks if a container has any search highlights.
 */
export function hasHighlights(container: HTMLElement): boolean {
  return container.querySelector('mark.search-match, mark.search-current') !== null;
}
