// Browser-mode shim for @tauri-apps/api/window.
// Returns a stub object whose methods all resolve to no-ops. The real window
// management is the user's browser tab.
//
// onCloseRequested is intentionally a no-op for Day 3 — Day 4 wires the
// pagehide-based shutdown contract separately rather than going through this
// API (the Tauri close-event semantics can't be cleanly polyfilled).

type Unlisten = () => void;

interface CloseEvent {
	preventDefault: () => void;
}

interface StubWindow {
	close: () => Promise<void>;
	destroy: () => Promise<void>;
	show: () => Promise<void>;
	hide: () => Promise<void>;
	setFocus: () => Promise<void>;
	startResizeDragging: (direction: unknown) => Promise<void>;
	startDragging: () => Promise<void>;
	onCloseRequested: (
		handler: (event: CloseEvent) => void | Promise<void>,
	) => Promise<Unlisten>;
	label: string;
}

const STUB: StubWindow = {
	close: async () => {
		// In browser mode, request the tab to close. window.close() only works
		// on tabs opened by script, so this is best-effort.
		window.close();
	},
	destroy: async () => {
		window.close();
	},
	show: async () => {},
	hide: async () => {},
	setFocus: async () => {},
	startResizeDragging: async () => {},
	startDragging: async () => {},
	onCloseRequested: async () => () => {},
	label: 'main',
};

export function getCurrentWindow(): StubWindow {
	return STUB;
}
