<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { ImageIcon } from '$lib/icons';

	let { selected }: NodeViewProps = $props();
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip-wrapper"
	data-media-chip
><span class="tag-chip media-chip {selected ? 'selected' : ''}"><span class="tag-icon"><ImageIcon /></span><span class="tag-content">Image</span></span></NodeViewWrapper>
