import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import PasteChipView from '../nodeviews/PasteChipView.svelte';

export const PasteChip = Node.create({
	name: 'pasteChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			content: { default: '' },
			lineCount: { default: 1 },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-paste-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						content: element.getAttribute('data-content') || '',
						lineCount: parseInt(element.getAttribute('data-line-count') || '1', 10),
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		const label = node.attrs.lineCount > 1 ? `Pasted (${node.attrs.lineCount} lines)` : 'Pasted text';
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-paste-chip': '',
				'data-content': node.attrs.content,
				'data-line-count': node.attrs.lineCount,
				class: 'tag-chip paste-chip',
			}),
			`[${label}]`,
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(PasteChipView);
	},

	addKeyboardShortcuts() {
		return {
			Backspace: () =>
				this.editor.commands.command(({ tr, state }) => {
					let isPasteChip = false;
					const { selection } = state;
					const { empty, anchor } = selection;

					if (!empty) return false;

					state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
						if (node.type.name === this.name) {
							isPasteChip = true;
							tr.insertText('', pos, pos + node.nodeSize);
							return false;
						}
					});

					return isPasteChip;
				}),
		};
	},
});
