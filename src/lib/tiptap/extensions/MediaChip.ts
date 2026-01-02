import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import MediaChipView from '../nodeviews/MediaChipView.svelte';

export const MediaChip = Node.create({
	name: 'mediaChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			image: { default: '' },
			mimeType: { default: 'image/png' },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-media-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						image: element.getAttribute('data-image') || '',
						mimeType: element.getAttribute('data-mime-type') || 'image/png',
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-media-chip': '',
				'data-image': node.attrs.image,
				'data-mime-type': node.attrs.mimeType,
				class: 'tag-chip media-chip',
			}),
			'[Image]',
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(MediaChipView);
	},

	addKeyboardShortcuts() {
		return {
			Backspace: () =>
				this.editor.commands.command(({ tr, state }) => {
					let isMediaChip = false;
					const { selection } = state;
					const { empty, anchor } = selection;

					if (!empty) return false;

					state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
						if (node.type.name === this.name) {
							isMediaChip = true;
							tr.insertText('', pos, pos + node.nodeSize);
							return false;
						}
					});

					return isMediaChip;
				}),
		};
	},
});
