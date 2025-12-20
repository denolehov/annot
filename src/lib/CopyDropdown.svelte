<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

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
		<svg
			xmlns="http://www.w3.org/2000/svg"
			fill="none"
			viewBox="0 0 24 24"
			stroke-width="1.5"
			stroke="currentColor"
			width="18"
			height="18"
			aria-hidden="true"
		>
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184"
			/>
		</svg>
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
		font-size: 12px;
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

	.copy-btn svg {
		opacity: 0.7;
		display: block;
	}

	.copy-btn:hover svg {
		opacity: 1;
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
