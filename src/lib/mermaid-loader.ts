let initialized = false;
let loadPromise: Promise<typeof import('mermaid')> | null = null;

/**
 * Preloads Mermaid module in the background.
 * Call this early (e.g., on app init) so diagram rendering feels instant.
 */
export function preloadMermaid(): void {
	if (loadPromise) return;
	loadPromise = import('mermaid');
}

export async function renderMermaid(source: string): Promise<string> {
	if (!loadPromise) {
		loadPromise = import('mermaid');
	}
	const mermaid = (await loadPromise).default;

	if (!initialized) {
		mermaid.initialize({
			startOnLoad: false,
			theme: 'default'
		});
		initialized = true;
	}

	const id = `mermaid-${Date.now()}`;
	const { svg } = await mermaid.render(id, source);
	return svg;
}
