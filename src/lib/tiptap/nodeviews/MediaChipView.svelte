<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';

	let { selected }: NodeViewProps = $props();
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip media-chip {selected ? 'selected' : ''}"
	data-media-chip
>
	<span class="tag-icon">&#x1F5BC;&#xFE0F;</span>
	<span class="tag-content">Image</span>
</NodeViewWrapper>
