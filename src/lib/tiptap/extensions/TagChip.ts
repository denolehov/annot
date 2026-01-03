import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import Suggestion, { type SuggestionOptions } from '@tiptap/suggestion';
import { PluginKey } from '@tiptap/pm/state';
import TagChipView from '../nodeviews/TagChipView.svelte';
import type { Tag } from '$lib/types';

const TagSuggestionPluginKey = new PluginKey('tagSuggestion');

export type TagChipOptions = {
	suggestion: Omit<SuggestionOptions<Tag>, 'editor' | 'pluginKey'>;
};

export const TagChip = Node.create<TagChipOptions>({
	name: 'tagChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			id: { default: null },
			name: { default: null },
			instruction: { default: null },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-tag-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					return {
						id: element.getAttribute('data-id') || null,
						name: element.getAttribute('data-name') || '',
						instruction: element.getAttribute('data-instruction') || '',
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-tag-chip': '',
				'data-id': node.attrs.id || '',
				'data-name': node.attrs.name,
				'data-instruction': node.attrs.instruction || '',
				class: 'tag-chip tag-tag',
			}),
			`[# ${node.attrs.name}]`,
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(TagChipView);
	},

	addKeyboardShortcuts() {
		return {
			Backspace: () =>
				this.editor.commands.command(({ tr, state }) => {
					let isTagChip = false;
					const { selection } = state;
					const { empty, anchor } = selection;

					if (!empty) return false;

					state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
						if (node.type.name === this.name) {
							isTagChip = true;
							tr.insertText('', pos, pos + node.nodeSize);
							return false;
						}
					});

					return isTagChip;
				}),
		};
	},

	addProseMirrorPlugins() {
		return [
			Suggestion({
				editor: this.editor,
				pluginKey: TagSuggestionPluginKey,
				...this.options.suggestion,
			}),
		];
	},
});
