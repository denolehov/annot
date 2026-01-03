import { parseMermaidToExcalidraw } from '@excalidraw/mermaid-to-excalidraw';
import { convertToExcalidrawElements } from '@excalidraw/excalidraw';

/**
 * Converts mermaid source to Excalidraw elements JSON string.
 * The library returns "skeleton" elements that must be converted to
 * fully qualified Excalidraw elements using convertToExcalidrawElements.
 *
 * @param source - The mermaid diagram source code
 * @returns JSON string of Excalidraw elements
 * @throws Error if conversion fails
 */
export async function convertMermaidToExcalidraw(source: string): Promise<string> {
  const { elements: skeletonElements } = await parseMermaidToExcalidraw(source, {
    // 16px produces well-proportioned text nodes in the generated Excalidraw elements.
    // This is not tied to editor font settings - Excalidraw elements are vectors that
    // scale with zoom, so the absolute size only affects internal proportions.
    themeVariables: { fontSize: '16px' },
  });
  // Convert skeleton elements to fully qualified Excalidraw elements
  const elements = convertToExcalidrawElements(skeletonElements);
  return JSON.stringify(elements);
}
