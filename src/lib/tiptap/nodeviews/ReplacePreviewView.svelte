<script lang="ts">
	import { NodeViewWrapper } from 'svelte-tiptap';
	import type { NodeViewProps } from '@tiptap/core';

	let { node, getPos }: NodeViewProps = $props();

	interface DiffLine {
		type: 'equal' | 'insert' | 'delete';
		line: string;
	}

	function computeDiff(original: string[], replacement: string[]): DiffLine[] {
		const m = original.length;
		const n = replacement.length;

		// Build LCS table
		const lcs: number[][] = Array(m + 1)
			.fill(null)
			.map(() => Array(n + 1).fill(0));
		for (let i = 1; i <= m; i++) {
			for (let j = 1; j <= n; j++) {
				if (original[i - 1] === replacement[j - 1]) {
					lcs[i][j] = lcs[i - 1][j - 1] + 1;
				} else {
					lcs[i][j] = Math.max(lcs[i - 1][j], lcs[i][j - 1]);
				}
			}
		}

		// Backtrack to find diff
		const result: DiffLine[] = [];
		let i = m,
			j = n;

		while (i > 0 || j > 0) {
			if (i > 0 && j > 0 && original[i - 1] === replacement[j - 1]) {
				result.unshift({ type: 'equal', line: original[i - 1] });
				i--;
				j--;
			} else if (j > 0 && (i === 0 || lcs[i][j - 1] >= lcs[i - 1][j])) {
				result.unshift({ type: 'insert', line: replacement[j - 1] });
				j--;
			} else {
				result.unshift({ type: 'delete', line: original[i - 1] });
				i--;
			}
		}

		return result;
	}

	// Show header only if this is the first node (position 0)
	const isFirst = $derived.by(() => {
		const pos = typeof getPos === 'function' ? getPos() : null;
		return pos === 0;
	});

	const diffLines = $derived.by(() => {
		const originalLines = (node.attrs.original as string).split('\n');
		const replacementLines = (node.attrs.replacement as string).split('\n');
		return computeDiff(originalLines, replacementLines);
	});

	function getGutterChar(type: DiffLine['type']): string {
		return type === 'delete' ? '-' : type === 'insert' ? '+' : ' ';
	}

	function getLineClass(type: DiffLine['type']): string {
		return type === 'delete' ? 'removed' : type === 'insert' ? 'added' : 'context';
	}
</script>

<NodeViewWrapper as="div" class="replace-preview" data-replace-preview>
	{#if isFirst}
		<div class="replace-preview-header">Replace</div>
	{/if}
	{#each diffLines as { type, line }}
		<div class="replace-preview-line {getLineClass(type)}">
			<span class="replace-preview-gutter">{getGutterChar(type)}</span>
			<span class="replace-preview-content">{line}</span>
		</div>
	{/each}
</NodeViewWrapper>
