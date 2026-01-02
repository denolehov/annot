import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import ReplacePreviewView from '../nodeviews/ReplacePreviewView.svelte';

export const ReplacePreview = Node.create({
	name: 'replacePreview',
	group: 'block',
	atom: true,

	addAttributes() {
		return {
			blockId: { default: null },
			original: { default: '' },
			replacement: { default: '' },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'div[data-replace-preview]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						blockId: element.getAttribute('data-block-id') || null,
						original: element.getAttribute('data-original') || '',
						replacement: element.getAttribute('data-replacement') || '',
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'div',
			mergeAttributes(HTMLAttributes, {
				'data-replace-preview': '',
				'data-block-id': node.attrs.blockId,
				'data-original': node.attrs.original,
				'data-replacement': node.attrs.replacement,
				class: 'replace-preview',
			}),
			'[REPLACE]',
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(ReplacePreviewView);
	},
});
