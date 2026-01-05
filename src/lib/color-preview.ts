/**
 * HEX color preview - inline swatch injection.
 *
 * Uses browser's Range API to inject color swatches before HEX values
 * while preserving existing HTML structure (syntax highlighting, etc).
 */

// Matches #RGB, #RGBA, #RRGGBB, #RRGGBBAA (word boundary to avoid partial matches)
const HEX_PATTERN = /#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/g;

interface TextNodeMatch {
  node: Text;
  index: number;
  hex: string;
}

/**
 * Collects all text nodes within a container, skipping gutter elements.
 */
function getTextNodes(container: HTMLElement): Text[] {
  const nodes: Text[] = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node: Node): number {
      // Skip text nodes inside gutter elements
      let parent = node.parentElement;
      while (parent && parent !== container) {
        if (
          parent.classList.contains('gutter-cell') ||
          parent.classList.contains('gutter')
        ) {
          return NodeFilter.FILTER_REJECT;
        }
        parent = parent.parentElement;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    nodes.push(node);
  }

  return nodes;
}

/**
 * Checks if a hex color has an alpha component.
 */
function hasAlpha(hex: string): boolean {
  const len = hex.length - 1; // exclude #
  return len === 4 || len === 8;
}

/**
 * Injects color swatches before HEX color values in a container.
 *
 * @param container - The element to search for HEX values
 */
export function injectColorSwatches(container: HTMLElement): void {
  const textNodes = getTextNodes(container);
  const matches: TextNodeMatch[] = [];

  // Collect all HEX matches (don't mutate while iterating)
  for (const node of textNodes) {
    const text = node.textContent;
    if (!text) continue;

    HEX_PATTERN.lastIndex = 0; // Reset regex state
    let match: RegExpExecArray | null;
    while ((match = HEX_PATTERN.exec(text)) !== null) {
      matches.push({
        node,
        index: match.index,
        hex: match[0],
      });
    }
  }

  // Process in reverse order to preserve text node positions
  for (let i = matches.length - 1; i >= 0; i--) {
    const { node, index, hex } = matches[i];

    try {
      // Split text node at the match position
      const beforeNode = node.splitText(index);

      // Create swatch element
      const swatch = document.createElement('span');
      swatch.className = 'color-swatch';
      swatch.style.setProperty('--swatch-color', hex);
      if (hasAlpha(hex)) {
        swatch.dataset.hasAlpha = 'true';
      }

      // Insert swatch before the hex value
      beforeNode.parentNode?.insertBefore(swatch, beforeNode);
    } catch {
      // Node manipulation can fail in edge cases, skip silently
    }
  }
}

/**
 * Removes all color swatches from a container.
 */
export function clearColorSwatches(container: HTMLElement): void {
  const swatches = container.querySelectorAll('span.color-swatch');
  swatches.forEach((swatch) => {
    swatch.remove();
  });
  // Normalize to merge adjacent text nodes
  container.normalize();
}
