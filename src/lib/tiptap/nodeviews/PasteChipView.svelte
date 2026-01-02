<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { tooltip } from '$lib/actions/tooltip';

	let { node, selected }: NodeViewProps = $props();

	function escapeHtml(text: string): string {
		const div = document.createElement('div');
		div.textContent = text;
		return div.innerHTML;
	}

	const label = $derived(
		node.attrs.lineCount > 1 ? `Pasted (${node.attrs.lineCount} lines)` : 'Pasted text'
	);

	const tooltipHtml = $derived.by(() => {
		const content = node.attrs.content as string;
		const lines = content.split('\n');
		const maxPreviewLines = 10;
		const previewLines = lines.slice(0, maxPreviewLines);
		const hasMore = lines.length > maxPreviewLines;

		let html = `<pre class="paste-preview-content">${escapeHtml(previewLines.join('\n'))}</pre>`;
		if (hasMore) {
			html += `<div class="paste-preview-more">+${lines.length - maxPreviewLines} more lines</div>`;
		}
		return html;
	});
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip paste-chip {selected ? 'selected' : ''}"
	data-paste-chip
>
	<span class="chip-hover-target" use:tooltip={{ content: tooltipHtml, variant: 'paste-tooltip', html: true }}>
		<span class="tag-icon">&#x1F4CB;</span>
		<span class="tag-content">{label}</span>
	</span>
</NodeViewWrapper>
