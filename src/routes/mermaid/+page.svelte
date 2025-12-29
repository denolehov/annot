<script lang="ts">
	import { onMount, onDestroy, tick } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
	import { renderMermaid } from '$lib/mermaid-loader';
	import { initTheme, type EffectiveTheme } from '$lib/theme';
	import panzoom from 'panzoom';
	import type { PanZoom } from 'panzoom';

	interface MermaidContext {
		source: string;
		file_path: string;
		start_line: number;
		end_line: number;
	}

	let svg = $state('');
	let loading = $state(true);
	let error = $state<string | null>(null);
	let context = $state<MermaidContext | null>(null);
	let canvasEl: HTMLDivElement | null = $state(null);
	let panzoomInstance: PanZoom | null = null;
	let currentScale = $state(1);

	// Layout constants
	const TITLE_BAR_HEIGHT = 40;
	const TOOLBAR_HEIGHT = 60;
	const DIAGRAM_PADDING = 40;

	onMount(async () => {
		try {
			// Initialize theme before rendering
			const effectiveTheme = await initTheme();

			// Get mermaid source from backend
			context = await invoke<MermaidContext>('get_mermaid_source');
			svg = await renderMermaid(context.source, effectiveTheme);
			loading = false;

			// Wait for DOM update
			await tick();

			// Initialize pan/zoom and show window
			await initPanZoom();
		} catch (e) {
			error = String(e);
			loading = false;
			// Still show window on error
			const win = getCurrentWindow();
			await win.show();
		}
	});

	onDestroy(() => {
		if (panzoomInstance) {
			panzoomInstance.dispose();
		}
	});

	async function initPanZoom() {
		if (!canvasEl) return;

		const svgEl = canvasEl.querySelector('svg');
		if (!svgEl) return;

		// Get intrinsic size from viewBox or attributes
		const viewBox = svgEl.getAttribute('viewBox');
		let diagramWidth: number;
		let diagramHeight: number;

		if (viewBox) {
			const parts = viewBox.split(/\s+|,/).map(Number);
			diagramWidth = parts[2] || 600;
			diagramHeight = parts[3] || 400;
		} else {
			diagramWidth = parseFloat(svgEl.getAttribute('width') || '600');
			diagramHeight = parseFloat(svgEl.getAttribute('height') || '400');
		}

		// Remove forced dimensions so SVG renders at native size
		svgEl.style.width = `${diagramWidth}px`;
		svgEl.style.height = `${diagramHeight}px`;

		// Resize window to fit diagram content
		const win = getCurrentWindow();

		// Calculate window size to fit diagram at 100%
		const windowWidth = Math.max(300, diagramWidth + DIAGRAM_PADDING);
		const windowHeight = Math.max(200, diagramHeight + DIAGRAM_PADDING + TOOLBAR_HEIGHT + TITLE_BAR_HEIGHT);

		// Cap to screen size
		const maxWidth = window.screen.availWidth * 0.9;
		const maxHeight = window.screen.availHeight * 0.9;
		const finalWidth = Math.min(windowWidth, maxWidth);
		const finalHeight = Math.min(windowHeight, maxHeight);

		await win.setSize(new LogicalSize(finalWidth, finalHeight));
		await win.center();

		// Initialize panzoom at 1:1
		panzoomInstance = panzoom(svgEl, {
			maxZoom: 5,
			minZoom: 0.1,
			initialZoom: 1,
			bounds: false,
			boundsPadding: 0.1,
			smoothScroll: false,
		});

		// Position diagram in canvas (canvas already offset by title bar via CSS margin-top)
		const canvasHeight = finalHeight - TITLE_BAR_HEIGHT;
		const availableHeight = canvasHeight - TOOLBAR_HEIGHT;
		const offsetX = (finalWidth - diagramWidth) / 2;
		// For tall diagrams, show from top; otherwise center vertically
		const offsetY = diagramHeight > availableHeight
			? DIAGRAM_PADDING / 2
			: (availableHeight - diagramHeight) / 2;
		panzoomInstance.zoomAbs(0, 0, 1);
		panzoomInstance.moveTo(offsetX, offsetY);

		// Track scale changes
		currentScale = 1;
		panzoomInstance.on('zoom', () => {
			if (panzoomInstance) {
				currentScale = panzoomInstance.getTransform().scale;
			}
		});

		// Show the window
		await win.show();
	}

	function zoomIn() {
		if (panzoomInstance) {
			const transform = panzoomInstance.getTransform();
			panzoomInstance.smoothZoom(
				window.innerWidth / 2,
				window.innerHeight / 2,
				1.25
			);
		}
	}

	function zoomOut() {
		if (panzoomInstance) {
			panzoomInstance.smoothZoom(
				window.innerWidth / 2,
				window.innerHeight / 2,
				0.8
			);
		}
	}

	function smartFit() {
		if (!canvasEl || !panzoomInstance) return;

		const svgEl = canvasEl.querySelector('svg');
		if (!svgEl) return;

		// Get native dimensions from style (set during init)
		const nativeWidth = parseFloat(svgEl.style.width) || svgEl.clientWidth;
		const nativeHeight = parseFloat(svgEl.style.height) || svgEl.clientHeight;

		// Canvas is already offset by title bar (CSS margin-top), just account for toolbar
		const availWidth = window.innerWidth - DIAGRAM_PADDING;
		const canvasHeight = window.innerHeight - TITLE_BAR_HEIGHT;
		const availHeight = canvasHeight - TOOLBAR_HEIGHT - DIAGRAM_PADDING;

		// Smart fit: compare aspect ratios
		const diagramAspect = nativeWidth / nativeHeight;
		const windowAspect = availWidth / availHeight;

		let fitScale: number;
		let offsetX: number;
		let offsetY: number;

		if (diagramAspect > windowAspect) {
			// Diagram is wider → fit to width, center vertically
			fitScale = availWidth / nativeWidth;
			offsetX = DIAGRAM_PADDING / 2;
			// Center in available vertical space (above toolbar)
			offsetY = (availHeight - nativeHeight * fitScale) / 2 + DIAGRAM_PADDING / 2;
		} else {
			// Diagram is taller → fit to height, center horizontally
			fitScale = availHeight / nativeHeight;
			offsetX = (window.innerWidth - nativeWidth * fitScale) / 2;
			offsetY = DIAGRAM_PADDING / 2;
		}

		panzoomInstance.zoomAbs(0, 0, fitScale);
		panzoomInstance.moveTo(offsetX, offsetY);
	}

	function actualSize() {
		if (!canvasEl || !panzoomInstance) return;

		const svgEl = canvasEl.querySelector('svg');
		if (!svgEl) return;

		// Get native dimensions from style (set during init)
		const nativeWidth = parseFloat(svgEl.style.width) || svgEl.clientWidth;
		const nativeHeight = parseFloat(svgEl.style.height) || svgEl.clientHeight;

		// Set to 100% and position (canvas already offset by title bar, account for toolbar)
		panzoomInstance.zoomAbs(0, 0, 1);
		const offsetX = (window.innerWidth - nativeWidth) / 2;
		const canvasHeight = window.innerHeight - TITLE_BAR_HEIGHT;
		const availableHeight = canvasHeight - TOOLBAR_HEIGHT;
		// For tall diagrams, show from top; otherwise center vertically
		const offsetY = nativeHeight > availableHeight
			? DIAGRAM_PADDING / 2
			: (availableHeight - nativeHeight) / 2;
		panzoomInstance.moveTo(offsetX, offsetY);
	}

	function centerDiagram() {
		// Center = smart fit (scales to fit window + centers)
		smartFit();
	}
</script>

<div class="mermaid-window">
	<header class="window-header" data-tauri-drag-region>
		<span class="window-title">Mermaid</span>
	</header>
	{#if loading}
		<div class="mermaid-loading">Rendering diagram...</div>
	{:else if error}
		<div class="mermaid-error">{error}</div>
	{:else}
		<div class="mermaid-canvas" bind:this={canvasEl}>
			{@html svg}
		</div>
		<div class="zoom-toolbar">
			<button onclick={zoomOut} title="Zoom out">−</button>
			<span class="zoom-level">{Math.round(currentScale * 100)}%</span>
			<button onclick={zoomIn} title="Zoom in">+</button>
			<button onclick={centerDiagram} title="Fit to window">Fit</button>
			<button onclick={actualSize} title="Actual size (100%)">1:1</button>
		</div>
	{/if}
</div>

<style>
	.mermaid-window {
		width: 100vw;
		height: 100vh;
		background: var(--bg-window);
		overflow: hidden;
		position: relative;
	}

	.window-header {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 40px;
		-webkit-app-region: drag;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		/* Match main window header styling */
		border-bottom: 1px solid rgba(0, 0, 0, 0.06);
		background: color-mix(in srgb, var(--bg-panel) 85%, transparent);
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
	}

	.window-title {
		font-family: var(--font-ui);
		font-size: 13px;
		font-weight: 500;
		color: var(--text-secondary);
	}

	.mermaid-canvas {
		width: 100%;
		height: calc(100% - 40px);
		margin-top: 40px;
		cursor: grab;
	}

	.mermaid-canvas:active {
		cursor: grabbing;
	}

	.zoom-toolbar {
		position: fixed;
		bottom: 16px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 4px;
		background: var(--bg-panel);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		padding: 6px 10px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
		font-family: var(--font-ui);
		font-size: 13px;
		z-index: 100;
	}

	.zoom-toolbar button {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 13px;
		font-weight: 500;
	}

	.zoom-toolbar button:hover {
		background: var(--bg-window);
		color: var(--text-primary);
	}

	.zoom-level {
		min-width: 48px;
		text-align: center;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}

	.mermaid-loading,
	.mermaid-error {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		font-family: var(--font-ui);
		font-size: 14px;
		color: var(--text-secondary);
	}

	.mermaid-error {
		color: var(--error-text);
	}
</style>
