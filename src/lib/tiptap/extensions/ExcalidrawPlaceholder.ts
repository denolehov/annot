import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import ExcalidrawPlaceholderView from '../nodeviews/ExcalidrawPlaceholderView.svelte';

export const ExcalidrawPlaceholder = Node.create({
	name: 'excalidrawPlaceholder',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			placeholderId: { default: () => crypto.randomUUID() },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-excalidraw-placeholder]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						placeholderId:
							element.getAttribute('data-placeholder-id') || crypto.randomUUID(),
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-excalidraw-placeholder': '',
				'data-placeholder-id': node.attrs.placeholderId,
				class: 'tag-chip excalidraw-placeholder',
			}),
			'[Drawing...]',
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(ExcalidrawPlaceholderView);
	},
});
