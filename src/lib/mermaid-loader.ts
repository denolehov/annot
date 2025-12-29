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
 * Render a mermaid diagram.
 * @param source - The mermaid source code
 * @param theme - The effective theme ('light' or 'dark')
 */
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
