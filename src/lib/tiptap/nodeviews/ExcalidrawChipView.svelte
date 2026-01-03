<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { onMount } from 'svelte';
	import { ExcalidrawIcon } from '$lib/icons';

	let { node, getPos, selected }: NodeViewProps = $props();

	let chipEl: HTMLSpanElement;

	function handleClick() {
		const pos = typeof getPos === 'function' ? getPos() : null;
		if (pos !== null && chipEl) {
			const event = new CustomEvent('excalidraw-edit', {
				bubbles: true,
				detail: {
					pos,
					elements: node.attrs.elements,
					nodeId: node.attrs.nodeId,
				},
			});
			chipEl.dispatchEvent(event);
		}
	}
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip-wrapper"
	data-excalidraw-chip
><span class="tag-chip excalidraw-chip {selected ? 'selected' : ''}"><!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions --><span class="chip-hover-target" bind:this={chipEl} onclick={handleClick}><span class="tag-icon"><ExcalidrawIcon /></span><span class="tag-content">Excalidraw</span></span></span></NodeViewWrapper>
