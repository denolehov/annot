<script lang="ts">
	import { tooltip } from '$lib/actions/tooltip';
	import type { AnnotationRefSnapshot } from '$lib/types';

	interface Props {
		snapshot: AnnotationRefSnapshot;
	}

	let { snapshot }: Props = $props();

	function escapeHtml(text: string): string {
		const div = document.createElement('div');
		div.textContent = text;
		return div.innerHTML;
	}

	const displayKey = $derived(`L${snapshot.source_key}`);
	const displayPreview = $derived.by(() => {
		const preview = snapshot.preview || '';
		return preview.length > 25 ? preview.slice(0, 25) + '...' : preview;
	});

	const tooltipHtml = $derived.by(() => {
		const file = snapshot.source_file || 'Current file';
		const preview = escapeHtml(snapshot.preview || '');
		return `<strong>${escapeHtml(file)}:${escapeHtml(snapshot.source_key)}</strong><br>${preview}`;
	});
</script>

<span class="chip-hover-target" use:tooltip={{ content: tooltipHtml, html: true }}>
	<span class="ref-icon annotation-icon">
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" width="12" height="12">
			<path fill-rule="evenodd" d="M4.848 2.771A49.144 49.144 0 0 1 12 2.25c2.43 0 4.817.178 7.152.52 1.978.292 3.348 2.024 3.348 3.97v6.02c0 1.946-1.37 3.678-3.348 3.97a48.901 48.901 0 0 1-3.476.383.39.39 0 0 0-.297.17l-2.755 4.133a.75.75 0 0 1-1.248 0l-2.755-4.133a.39.39 0 0 0-.297-.17 48.9 48.9 0 0 1-3.476-.384c-1.978-.29-3.348-2.024-3.348-3.97V6.741c0-1.946 1.37-3.68 3.348-3.97ZM6.75 8.25a.75.75 0 0 1 .75-.75h9a.75.75 0 0 1 0 1.5h-9a.75.75 0 0 1-.75-.75Zm.75 2.25a.75.75 0 0 0 0 1.5H12a.75.75 0 0 0 0-1.5H7.5Z" clip-rule="evenodd" />
		</svg>
	</span>
	<span class="ref-key">{displayKey}</span>
	{#if displayPreview}
		<span class="ref-divider">·</span>
		<span class="ref-preview">{displayPreview}</span>
	{/if}
</span>
