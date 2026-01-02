<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { tooltip } from '$lib/actions/tooltip';

	let { node, selected }: NodeViewProps = $props();
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip tag-tag {selected ? 'selected' : ''}"
	data-tag-chip
>
	{#if node.attrs.instruction}
		<span class="chip-hover-target" use:tooltip={{ content: node.attrs.instruction }}>
			<span class="tag-icon">#</span>
			<span class="tag-content">{node.attrs.name}</span>
		</span>
	{:else}
		<span class="tag-icon">#</span>
		<span class="tag-content">{node.attrs.name}</span>
	{/if}
</NodeViewWrapper>
