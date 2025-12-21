/**
 * Excalidraw Loader - Lazy loads React + Excalidraw and mounts into a container.
 * Uses React island pattern for Svelte integration.
 */

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
  onSave?: (elements: readonly ExcalidrawElement[], png: string) => void;
  onCancel?: () => void;
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

  const { Excalidraw, exportToBlob } = ExcalidrawModule;

  // Create root
  const root = createRoot(options.container);

  // Store ref to Excalidraw API
  let excalidrawAPI: ExcalidrawAPI = null;

  // Create wrapper component using createElement (no JSX needed)
  const ExcalidrawWrapper = () => {
    return React.createElement(
      React.Fragment,
      null,
      // Excalidraw - renders directly, will fill container
      React.createElement(Excalidraw, {
        initialData: {
          elements: options.initialElements || [],
          appState: { viewBackgroundColor: '#ffffff' },
        },
        excalidrawAPI: (api: ExcalidrawAPI) => {
          excalidrawAPI = api;
        },
      }),
      // Control buttons - positioned via CSS
      React.createElement(
        'div',
        { className: 'excalidraw-controls' },
        React.createElement(
          'button',
          {
            className: 'excalidraw-cancel',
            onClick: () => options.onCancel?.(),
          },
          'Cancel'
        ),
        React.createElement(
          'button',
          {
            className: 'excalidraw-save',
            onClick: async () => {
              if (excalidrawAPI && options.onSave) {
                const elements = excalidrawAPI.getSceneElements();
                try {
                  const blob = await exportToBlob({
                    elements,
                    mimeType: 'image/png',
                    appState: excalidrawAPI.getAppState(),
                    files: excalidrawAPI.getFiles(),
                  });
                  const reader = new FileReader();
                  reader.onloadend = () => {
                    options.onSave!(elements, reader.result as string);
                  };
                  reader.readAsDataURL(blob);
                } catch (e) {
                  console.error('Failed to export Excalidraw to PNG:', e);
                  // Still save elements even if PNG export fails
                  options.onSave!(elements, '');
                }
              }
            },
          },
          'Save'
        )
      )
    );
  };

  // Render
  root.render(React.createElement(ExcalidrawWrapper));

  return {
    unmount: () => root.unmount(),
    getElements: () => excalidrawAPI?.getSceneElements() || [],
    getAppState: () => excalidrawAPI?.getAppState() || ({} as AppState),
  };
}
