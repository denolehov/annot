<script lang="ts">
	import { tooltip } from '$lib/actions/tooltip';
	import type { Bookmark } from '$lib/types';

	interface Props {
		snapshot: Bookmark;
	}

	let { snapshot }: Props = $props();

	function escapeHtml(text: string): string {
		const div = document.createElement('div');
		div.textContent = text;
		return div.innerHTML;
	}

	const shortId = $derived(snapshot.id?.slice(0, 3) || '');
	const displayLabel = $derived.by(() => {
		const label = snapshot.label || snapshot.snapshot.source_title || '';
		return label.length > 25 ? label.slice(0, 25) + '...' : label;
	});

	const tooltipHtml = $derived(
		`<strong>${escapeHtml(snapshot.id || '')}</strong><br>${escapeHtml(snapshot.label || snapshot.snapshot.source_title || '')}`
	);
</script>

<span class="chip-hover-target" use:tooltip={{ content: tooltipHtml, html: true }}>
	<span class="ref-icon bookmark-icon">
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" width="12" height="12">
			<path fill-rule="evenodd" d="M6.32 2.577a49.255 49.255 0 0 1 11.36 0c1.497.174 2.57 1.46 2.57 2.93V21a.75.75 0 0 1-1.085.67L12 18.089l-7.165 3.583A.75.75 0 0 1 3.75 21V5.507c0-1.47 1.073-2.756 2.57-2.93Z" clip-rule="evenodd" />
		</svg>
	</span>
	<span class="ref-key">{shortId}</span>
	{#if displayLabel}
		<span class="ref-divider">·</span>
		<span class="ref-label">{displayLabel}</span>
	{/if}
</span>
