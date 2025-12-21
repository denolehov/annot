<script lang="ts">
	import { onMount } from 'svelte';
	import { renderMermaid } from './mermaid-loader';

	interface Props {
		source: string;
		onClose: () => void;
	}

	let { source, onClose }: Props = $props();

	let svg = $state('');
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			svg = await renderMermaid(source);
			loading = false;
		} catch (e) {
			error = String(e);
			loading = false;
		}
	});

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onClose();
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="mermaid-backdrop" onclick={handleBackdropClick} role="presentation">
	<div class="mermaid-modal" role="dialog" aria-modal="true">
		<button class="mermaid-close" onclick={onClose} aria-label="Close" title="Close">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				width="20"
				height="20"
			>
				<path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
			</svg>
		</button>
		<div class="mermaid-body">
			{#if loading}
				<div class="mermaid-loading">Rendering diagram...</div>
			{:else if error}
				<div class="mermaid-error">{error}</div>
			{:else}
				<div class="mermaid-container">
					{@html svg}
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.mermaid-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.75);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 50;
	}

	.mermaid-modal {
		position: relative;
		background: var(--bg-window);
		border-radius: 8px;
		box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
		width: 95vw;
		max-height: 90vh;
		margin: 5vh auto;
		display: flex;
		flex-direction: column;
	}

	.mermaid-close {
		position: absolute;
		top: 16px;
		right: 16px;
		z-index: 10;
		color: var(--text-muted);
		background: none;
		border: none;
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
		transition: all 0.15s ease;
	}

	.mermaid-close:hover {
		color: var(--text-primary);
		background: var(--bg-panel);
	}

	.mermaid-close:focus-visible {
		outline: none;
		background: var(--bg-panel);
		color: var(--text-primary);
	}

	.mermaid-body {
		padding: 2rem 0;
		background: var(--bg-panel);
		border-radius: 8px;
		overflow: auto;
		flex: 1;
		min-height: 0;
	}

	.mermaid-container :global(svg) {
		display: block;
		margin: 0 auto;
		max-width: 100%;
		height: auto;
	}

	.mermaid-loading,
	.mermaid-error {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 200px;
		font-family: var(--font-ui);
		font-size: 14px;
		color: var(--text-secondary);
	}

	.mermaid-error {
		color: var(--error-text, #dc2626);
	}
</style>
