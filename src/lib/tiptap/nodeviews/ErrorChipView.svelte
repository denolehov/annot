<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { tooltip } from '$lib/actions/tooltip';

	let { node, selected }: NodeViewProps = $props();
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip error-chip {selected ? 'selected' : ''}"
	data-error-chip
>
	<span class="chip-hover-target" use:tooltip={{ content: node.attrs.message, variant: 'error-tooltip' }}>
		<span class="tag-icon">&#x26A0;&#xFE0F;</span>
		<span class="tag-content">{node.attrs.source} syntax error</span>
	</span>
</NodeViewWrapper>
