<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import type { RefSnapshot } from '$lib/types';
	import AnnotationRefChip from './AnnotationRefChip.svelte';
	import BookmarkRefChip from './BookmarkRefChip.svelte';
	import FileRefChip from './FileRefChip.svelte';
	import HeadingRefChip from './HeadingRefChip.svelte';

	let { node, selected }: NodeViewProps = $props();

	type RefVariant =
		| { kind: 'heading'; level: number; title: string; line: number }
		| { kind: 'file'; path: string }
		| { kind: 'annotation'; snapshot: RefSnapshot & { type: 'annotation' } }
		| { kind: 'bookmark'; snapshot: RefSnapshot & { type: 'bookmark' } }
		| { kind: 'unknown' };

	const variant = $derived.by((): RefVariant => {
		const { refType, snapshot, path, sectionLine, sectionLevel, sectionTitle } = node.attrs;

		if (refType === 'heading' && sectionLevel && sectionTitle && sectionLine != null) {
			return { kind: 'heading', level: sectionLevel, title: sectionTitle, line: sectionLine };
		}
		if (refType === 'file' && path) {
			return { kind: 'file', path };
		}
		if (refType === 'annotation' && snapshot?.type === 'annotation') {
			return { kind: 'annotation', snapshot };
		}
		if (refType === 'bookmark' && snapshot?.type === 'bookmark') {
			return { kind: 'bookmark', snapshot };
		}
		return { kind: 'unknown' };
	});
</script>

<!--
  NodeViewWrapper must have no whitespace between tags for proper inline rendering.
  The span inside handles styling; variant.kind drives which chip component renders.
-->
<NodeViewWrapper as="span" class="tag-chip-wrapper" data-ref-chip
><span class="tag-chip ref-chip ref-{variant.kind} {selected ? 'selected' : ''}"
		>{#if variant.kind === 'heading'}<HeadingRefChip
				level={variant.level}
				title={variant.title}
				line={variant.line}
			/>{:else if variant.kind === 'file'}<FileRefChip
				path={variant.path}
			/>{:else if variant.kind === 'annotation'}<AnnotationRefChip
				snapshot={variant.snapshot}
			/>{:else if variant.kind === 'bookmark'}<BookmarkRefChip
				snapshot={variant.snapshot.bookmark}
			/>{:else}<span class="ref-icon">@</span><span class="ref-content">?</span>{/if}</span
	></NodeViewWrapper>
