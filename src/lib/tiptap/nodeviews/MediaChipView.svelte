<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { ImageIcon } from '$lib/icons';

	let { selected }: NodeViewProps = $props();
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip media-chip {selected ? 'selected' : ''}"
	data-media-chip
>
	<span class="tag-icon"><ImageIcon /></span>
	<span class="tag-content">Image</span>
</NodeViewWrapper>
