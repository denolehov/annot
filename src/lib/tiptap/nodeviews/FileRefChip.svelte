<script lang="ts">
	interface Props {
		path: string;
	}

	let { path }: Props = $props();

	const filename = $derived(path.split('/').pop() || path);
	const parent = $derived(() => {
		const parts = path.split('/');
		return parts.length > 1 ? parts[parts.length - 2] : null;
	});

	async function copyPath() {
		await navigator.clipboard.writeText(path);
		// TODO: show toast via event
	}
</script>

<span
	class="file-ref-chip"
	title={path}
	onclick={copyPath}
	role="button"
	tabindex="0"
	onkeydown={(e) => e.key === 'Enter' && copyPath()}
>
	<span class="file-icon">@</span>
	<span class="file-name">{filename}</span>
	{#if parent()}
		<span class="file-parent">({parent()})</span>
	{/if}
</span>

<style>
	.file-ref-chip {
		display: inline-flex;
		align-items: baseline;
		gap: 2px;
		cursor: pointer;
	}

	.file-ref-chip:hover {
		text-decoration: underline;
	}

	.file-icon {
		opacity: 0.6;
	}

	.file-name {
		font-size: 12px;
		font-weight: 500;
	}

	.file-parent {
		opacity: 0.5;
		font-size: 0.9em;
	}
</style>
