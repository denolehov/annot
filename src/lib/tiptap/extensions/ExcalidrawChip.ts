import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import ExcalidrawChipView from '../nodeviews/ExcalidrawChipView.svelte';

export const ExcalidrawChip = Node.create({
	name: 'excalidrawChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			nodeId: {
				default: () => crypto.randomUUID(),
				parseHTML: (element) => element.getAttribute('data-node-id') || crypto.randomUUID(),
			},
			elements: { default: '[]' },
			image: { default: null },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-excalidraw-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						nodeId: element.getAttribute('data-node-id') || crypto.randomUUID(),
						elements: element.getAttribute('data-elements') || '[]',
						image: element.getAttribute('data-image') || null,
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-excalidraw-chip': '',
				'data-node-id': node.attrs.nodeId,
				'data-elements': node.attrs.elements,
				'data-image': node.attrs.image || '',
				class: 'tag-chip excalidraw-chip',
			}),
			'[Diagram]',
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(ExcalidrawChipView);
	},

	addKeyboardShortcuts() {
		return {
			Backspace: () =>
				this.editor.commands.command(({ tr, state }) => {
					let isChip = false;
					const { selection } = state;
					const { empty, anchor } = selection;

					if (!empty) return false;

					state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
						if (node.type.name === this.name) {
							isChip = true;
							tr.insertText('', pos, pos + node.nodeSize);
							return false;
						}
					});

					return isChip;
				}),
		};
	},
});
