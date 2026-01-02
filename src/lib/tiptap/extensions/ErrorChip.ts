import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import ErrorChipView from '../nodeviews/ErrorChipView.svelte';

export const ErrorChip = Node.create({
	name: 'errorChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			source: { default: '' },
			message: { default: '' },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-error-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						source: element.getAttribute('data-source') || '',
						message: element.getAttribute('data-message') || '',
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-error-chip': '',
				'data-source': node.attrs.source,
				'data-message': node.attrs.message,
				class: 'tag-chip error-chip',
			}),
			`[${node.attrs.source} error]`,
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(ErrorChipView);
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
