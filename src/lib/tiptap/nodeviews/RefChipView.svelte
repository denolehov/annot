<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import type { RefSnapshot } from '$lib/types';
	import AnnotationRefChip from './AnnotationRefChip.svelte';
	import BookmarkRefChip from './BookmarkRefChip.svelte';
	import FileRefChip from './FileRefChip.svelte';

	let { node, selected }: NodeViewProps = $props();

	const refType = $derived(node.attrs.refType as 'annotation' | 'bookmark' | 'file' | null);
	const snapshot = $derived(node.attrs.snapshot as RefSnapshot | null);
	const path = $derived(node.attrs.path as string | null);
</script>

<NodeViewWrapper
	as="span"
	class="tag-chip ref-chip ref-{refType || 'unknown'} {selected ? 'selected' : ''}"
	data-ref-chip
>
	{#if refType === 'file' && path}
		<FileRefChip {path} />
	{:else if refType === 'annotation' && snapshot?.type === 'annotation'}
		<AnnotationRefChip {snapshot} />
	{:else if refType === 'bookmark' && snapshot?.type === 'bookmark'}
		<BookmarkRefChip snapshot={snapshot.bookmark} />
	{:else}
		<span class="ref-icon">@</span>
		<span class="ref-content">?</span>
	{/if}
</NodeViewWrapper>
