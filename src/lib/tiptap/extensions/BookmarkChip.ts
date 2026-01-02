import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import Suggestion, { type SuggestionOptions } from '@tiptap/suggestion';
import { PluginKey } from '@tiptap/pm/state';
import BookmarkChipView from '../nodeviews/BookmarkChipView.svelte';
import type { Bookmark } from '$lib/types';

const BookmarkSuggestionPluginKey = new PluginKey('bookmarkSuggestion');

export type BookmarkChipOptions = {
	suggestion: Omit<SuggestionOptions<Bookmark>, 'editor' | 'pluginKey'>;
};

export const BookmarkChip = Node.create<BookmarkChipOptions>({
	name: 'bookmarkChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			id: { default: null },
			label: { default: null },
			bookmark: { default: null },
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-bookmark-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					const bookmarkData = element.getAttribute('data-bookmark');
					return {
						id: element.getAttribute('data-id') || null,
						label: element.getAttribute('data-label') || '',
						bookmark: bookmarkData ? JSON.parse(bookmarkData) : null,
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		const shortId = node.attrs.id?.slice(0, 3) || '';
		const label = node.attrs.label || '';
		const displayLabel = label.length > 30 ? label.slice(0, 30) + '...' : label;
		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-bookmark-chip': '',
				'data-id': node.attrs.id || '',
				'data-label': node.attrs.label || '',
				'data-bookmark': node.attrs.bookmark ? JSON.stringify(node.attrs.bookmark) : '',
				class: 'tag-chip bookmark-chip',
			}),
			`[@ ${shortId} · ${displayLabel}]`,
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(BookmarkChipView);
	},

	addKeyboardShortcuts() {
		return {
			Backspace: () =>
				this.editor.commands.command(({ tr, state }) => {
					let isBookmarkChip = false;
					const { selection } = state;
					const { empty, anchor } = selection;

					if (!empty) return false;

					state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
						if (node.type.name === this.name) {
							isBookmarkChip = true;
							tr.insertText('', pos, pos + node.nodeSize);
							return false;
						}
					});

					return isBookmarkChip;
				}),
		};
	},

	addProseMirrorPlugins() {
		return [
			Suggestion({
				editor: this.editor,
				pluginKey: BookmarkSuggestionPluginKey,
				...this.options.suggestion,
			}),
		];
	},
});
