<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';
	import { invoke } from '@tauri-apps/api/core';

	let { node, getPos }: NodeViewProps = $props();

	interface DiffSpan {
		text: string;
		emphasized: boolean;
	}

	type ReplaceDiffLine =
		| { type: 'equal'; spans: DiffSpan[] }
		| { type: 'insert'; spans: DiffSpan[] }
		| { type: 'delete'; spans: DiffSpan[] };

	// Show header only if this is the first node (position 0)
	const isFirst = $derived.by(() => {
		const pos = typeof getPos === 'function' ? getPos() : null;
		return pos === 0;
	});

	// Async state for diff computation
	let diffLines = $state<ReplaceDiffLine[]>([]);

	// Compute diff via backend when node attrs change
	$effect(() => {
		const original = node.attrs.original as string;
		const replacement = node.attrs.replacement as string;

		invoke<ReplaceDiffLine[]>('compute_replace_diff', { original, replacement }).then(
			(result) => {
				diffLines = result;
			}
		);
	});

	function getGutterChar(type: ReplaceDiffLine['type']): string {
		return type === 'delete' ? '-' : type === 'insert' ? '+' : ' ';
	}

	function getLineClass(type: ReplaceDiffLine['type']): string {
		return type === 'delete' ? 'removed' : type === 'insert' ? 'added' : 'context';
	}
</script>

<NodeViewWrapper as="div" class="replace-preview" data-replace-preview>
	{#if isFirst}
		<div class="replace-preview-header">Replace</div>
	{/if}
	{#each diffLines as { type, spans }}
		<div class="replace-preview-line {getLineClass(type)}">
			<span class="replace-preview-gutter">{getGutterChar(type)}</span>
			<span class="replace-preview-content"
				>{#each spans as span}<span class:emphasized={span.emphasized}>{span.text}</span
					>{/each}</span
			>
		</div>
	{/each}
</NodeViewWrapper>
