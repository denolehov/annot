import type { EffectiveTheme } from './theme';

let loadPromise: Promise<typeof import('mermaid')> | null = null;
let currentTheme: string | null = null;

/**
 * Preloads Mermaid module in the background.
 * Call this early (e.g., on app init) so diagram rendering feels instant.
 */
export function preloadMermaid(): void {
	if (loadPromise) return;
	loadPromise = import('mermaid');
}

/**
 * Supported mermaid diagram types for Excalidraw conversion.
 * The @excalidraw/mermaid-to-excalidraw library only supports:
 * - Flowchart (flowchart, graph)
 * - Sequence (sequenceDiagram)
 * - Class (classDiagram)
 *
 * Pattern skips leading whitespace, comments (%% ...), and directives (%%{...}%%)
 * before matching the diagram type.
 */
const EXCALIDRAW_SUPPORTED_TYPES =
	/^\s*(?:%%.*\n\s*)*(flowchart|graph|sequenceDiagram|classDiagram)\b/;

/**
 * Check if a mermaid diagram can be converted to Excalidraw.
 * Only Flowchart, Sequence, and Class diagrams are supported.
 * Handles diagrams with leading comments or directives.
 * @param source - The mermaid source code
 */
export function isMermaidExcalidrawSupported(source: string): boolean {
	return EXCALIDRAW_SUPPORTED_TYPES.test(source);
}

export async function renderMermaid(source: string, theme: EffectiveTheme = 'light'): Promise<string> {
	if (!loadPromise) {
		loadPromise = import('mermaid');
	}
	const mermaid = (await loadPromise).default;

	// Map our theme to mermaid's theme
	const mermaidTheme = theme === 'dark' ? 'dark' : 'default';

	// Re-initialize if theme changed or first time
	if (currentTheme !== mermaidTheme) {
		mermaid.initialize({
			startOnLoad: false,
			theme: mermaidTheme
		});
		currentTheme = mermaidTheme;
	}

	const id = `mermaid-${Date.now()}`;
	const { svg } = await mermaid.render(id, source);
	return svg;
}
