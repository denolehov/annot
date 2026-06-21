// Browser-mode shim for @tauri-apps/api/event. listen/emit are no-ops for the
// spike — the events that use them (excalidraw-result, mermaid-open-excalidraw,
// codeblock-excalidraw-result, tauri://focus) are tied to features that are
// out of scope for browser mode v1.

type Unlisten = () => void;

export async function listen<T = unknown>(
	_event: string,
	_handler: (e: { payload: T }) => void,
): Promise<Unlisten> {
	return () => {};
}

export async function emit(_event: string, _payload?: unknown): Promise<void> {
	// no-op
}
