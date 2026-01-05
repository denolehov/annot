/**
 * Excalidraw Loader - Lazy loads React + Excalidraw and mounts into a container.
 * Uses React island pattern for Svelte integration.
 */

import type { EffectiveTheme } from './theme';

// Use generic types to avoid import issues with @excalidraw/excalidraw internals
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ExcalidrawElement = any;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AppState = any;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ExcalidrawAPI = any;

export interface ExcalidrawHandle {
  unmount: () => void;
  getElements: () => readonly ExcalidrawElement[];
  getAppState: () => AppState;
}

export interface ExcalidrawLoaderOptions {
  container: HTMLElement;
  initialElements?: ExcalidrawElement[];
  theme?: EffectiveTheme;
}

let loadPromise: Promise<typeof import('@excalidraw/excalidraw')> | null = null;

/**
 * Preloads React + Excalidraw modules in the background.
 * Call this early (e.g., on app init) so /excalidraw feels instant.
 */
export function preloadExcalidraw(): void {
  if (loadPromise) return; // Already loading/loaded

  loadPromise = import('@excalidraw/excalidraw');

  // Fire-and-forget: preload React, ReactDOM, CSS in parallel
  Promise.all([
    import('react'),
    import('react-dom/client'),
    loadPromise,
    import('@excalidraw/excalidraw/index.css'),
  ]);
}

/**
 * Lazy-loads React + Excalidraw and mounts into the given container.
 * Returns a handle for controlling the instance.
 */
export async function mountExcalidraw(
  options: ExcalidrawLoaderOptions
): Promise<ExcalidrawHandle> {
  // Lazy load React, Excalidraw, and CSS
  if (!loadPromise) {
    loadPromise = import('@excalidraw/excalidraw');
  }

  const [React, { createRoot }, ExcalidrawModule] = await Promise.all([
    import('react'),
    import('react-dom/client'),
    loadPromise,
    // Load Excalidraw CSS
    import('@excalidraw/excalidraw/index.css'),
  ]);

  const { Excalidraw } = ExcalidrawModule;

  // Create root
  const root = createRoot(options.container);

  // Store ref to Excalidraw API + readiness promise
  let excalidrawAPI: ExcalidrawAPI = null;
  let resolveReady: () => void;
  const apiReady = new Promise<void>((resolve) => {
    resolveReady = resolve;
  });

  // Create wrapper component using createElement (no JSX needed)
  const ExcalidrawWrapper = () => {
    return React.createElement(Excalidraw, {
      theme: options.theme === 'dark' ? 'dark' : 'light',
      initialData: {
        elements: options.initialElements || [],
      },
      excalidrawAPI: (api: ExcalidrawAPI) => {
        excalidrawAPI = api;
        // Center diagram in viewport after canvas renders
        if (options.initialElements?.length) {
          requestAnimationFrame(() => {
            api.scrollToContent(options.initialElements!, {
              fitToViewport: true,
              viewportZoomFactor: 0.9,
            });
          });
        }
        // Signal that API is ready
        resolveReady();
      },
    });
  };

  // Render and wait for API to be ready
  root.render(React.createElement(ExcalidrawWrapper));
  await apiReady;

  return {
    unmount: () => root.unmount(),
    getElements: () => excalidrawAPI?.getSceneElements() || [],
    getAppState: () => excalidrawAPI?.getAppState() || ({} as AppState),
  };
}
