<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { onMount } from 'svelte';
	import { ExcalidrawIcon } from '$lib/icons';

	let { node, getPos }: NodeViewProps = $props();

	let chipEl: HTMLSpanElement;

	onMount(() => {
		// Dispatch event to open window immediately with placeholderId
		requestAnimationFrame(() => {
			const pos = typeof getPos === 'function' ? getPos() : null;
			if (pos !== null && chipEl) {
				const event = new CustomEvent('excalidraw-create', {
					bubbles: true,
					detail: { pos, placeholderId: node.attrs.placeholderId },
				});
				chipEl.dispatchEvent(event);
			}
		});
	});
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip excalidraw-placeholder"
	data-excalidraw-placeholder
>
	<span class="chip-hover-target" bind:this={chipEl}>
		<span class="tag-icon"><ExcalidrawIcon /></span>
		<span class="tag-content">Drawing...</span>
	</span>
</NodeViewWrapper>
