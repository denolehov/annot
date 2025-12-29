<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import Icon from '$lib/CommandPalette/Icon.svelte';

	interface Props {
		showToast: (message: string) => void;
	}

	let { showToast }: Props = $props();

	let open = $state(false);

	async function copyToClipboard(mode: 'content' | 'annotations' | 'all') {
		const labels = {
			content: 'Content',
			annotations: 'Annotations',
			all: 'Content + Annotations'
		};

		try {
			await invoke('copy_to_clipboard', { mode });
			showToast(`${labels[mode]} copied!`);
		} catch (e) {
			showToast(`Failed to copy: ${e}`);
		}
		open = false;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			open = false;
		}
	}

	function handleClickOutside(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (!target.closest('.copy-dropdown')) {
			open = false;
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} onclick={handleClickOutside} />

<div class="copy-dropdown">
	<button
		class="copy-btn"
		onclick={() => (open = !open)}
		aria-haspopup="true"
		aria-expanded={open}
		title="Copy"
	>
		<Icon name="copy-code" />
	</button>
	{#if open}
		<div class="copy-menu">
			<button class="copy-menu-item" onclick={() => copyToClipboard('content')}>Content</button>
			<button class="copy-menu-item" onclick={() => copyToClipboard('annotations')}
				>Annotations</button
			>
			<button class="copy-menu-item" onclick={() => copyToClipboard('all')}>Both</button>
		</div>
	{/if}
</div>

<style>
	.copy-dropdown {
		position: relative;
	}

	.copy-btn {
		display: inline-flex;
		align-items: center;
		gap: 0;
		padding: 4px 6px;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 6px;
		color: var(--text-secondary);
		cursor: pointer;
		font-family: var(--font-ui);
		font-size: 18px;
		font-weight: 500;
		transition: all 150ms ease;
		line-height: 1;
	}

	.copy-btn:hover {
		background: var(--bg-window);
		border-color: var(--border-subtle);
		color: var(--text-primary);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}

	.copy-btn:focus-visible {
		outline: none;
		border-color: var(--focus-ring);
	}

	.copy-menu {
		position: absolute;
		top: calc(100% + 4px);
		right: 0;
		background: var(--bg-window);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		padding: 4px;
		min-width: 140px;
		box-shadow:
			0 4px 12px rgba(0, 0, 0, 0.08),
			0 1px 3px rgba(0, 0, 0, 0.06);
		z-index: 1000;
		animation: dropdown-enter 150ms ease;
	}

	@keyframes dropdown-enter {
		from {
			opacity: 0;
			transform: translateY(-4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.copy-menu-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 8px 12px;
		background: transparent;
		border: none;
		border-radius: 4px;
		color: var(--text-secondary);
		cursor: pointer;
		font-family: var(--font-ui);
		font-size: 12px;
		font-weight: 500;
		text-align: left;
	}

	.copy-menu-item:hover {
		background: var(--bg-panel);
		color: var(--text-primary);
	}

	.copy-menu-item:focus-visible {
		outline: none;
		background: var(--bg-panel);
	}
</style>
