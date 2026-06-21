// Browser-mode shim for @tauri-apps/api/core. Built-time aliased by
// vite.config.js when ANNOT_BROWSER=1. Replaces Tauri's IPC bridge with
// fetch() against the local axum server's /invoke/* routes.

export async function invoke<T = unknown>(
	cmd: string,
	args?: Record<string, unknown>,
): Promise<T> {
	const res = await fetch(`/invoke/${cmd}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(args ?? {}),
	});

	if (res.status === 501) {
		// Spike: command isn't wired. Don't throw — the bootstrap path fans
		// out into many commands; one unwired one shouldn't crash the page.
		const msg = await res.text();
		console.warn(`[browser-shim] ${msg}`);
		return null as T;
	}

	if (!res.ok) {
		const err = await res.text();
		throw new Error(err || `invoke ${cmd} failed: HTTP ${res.status}`);
	}

	const text = await res.text();
	if (text === '' || text === 'null') return null as T;
	return JSON.parse(text) as T;
}
