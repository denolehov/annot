import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import Suggestion, { type SuggestionOptions } from '@tiptap/suggestion';
import { PluginKey } from '@tiptap/pm/state';
import RefChipView from '../nodeviews/RefChipView.svelte';
import type { RefSnapshot, Bookmark, AnnotationRefSnapshot } from '$lib/types';

const RefSuggestionPluginKey = new PluginKey('refSuggestion');

/** Unified suggestion item for @ menu - either an annotation or a bookmark. */
export type RefSuggestionItem =
	| { type: 'annotation'; key: string; preview: string; content: import('$lib/types').ContentNode[] }
	| { type: 'bookmark'; bookmark: Bookmark };

export type RefChipOptions = {
	suggestion: Omit<SuggestionOptions<RefSuggestionItem>, 'editor' | 'pluginKey'>;
};

export const RefChip = Node.create<RefChipOptions>({
	name: 'refChip',
	group: 'inline',
	inline: true,
	atom: true,

	addAttributes() {
		return {
			refType: { default: null }, // 'annotation' | 'bookmark'
			snapshot: { default: null }, // RefSnapshot
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-ref-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					const snapshotData = element.getAttribute('data-snapshot');
					return {
						refType: element.getAttribute('data-ref-type') || null,
						snapshot: snapshotData ? JSON.parse(snapshotData) : null,
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		const refType = node.attrs.refType as string;
		const snapshot = node.attrs.snapshot as RefSnapshot | null;

		// Build display text based on type
		let displayText = '[@?]';
		if (snapshot) {
			if (refType === 'annotation' && snapshot.type === 'annotation') {
				const annSnap = snapshot as AnnotationRefSnapshot;
				const preview = annSnap.preview?.slice(0, 20) || '';
				displayText = `[@L${annSnap.source_key}${preview ? ' · ' + preview : ''}...]`;
			} else if (refType === 'bookmark' && snapshot.type === 'bookmark') {
				const shortId = snapshot.bookmark.id?.slice(0, 3) || '';
				const label = snapshot.bookmark.label || snapshot.bookmark.snapshot.source_title || '';
				const truncLabel = label.length > 20 ? label.slice(0, 20) + '...' : label;
				displayText = `[@${shortId}${truncLabel ? ' · ' + truncLabel : ''}]`;
			}
		}

		return [
			'span',
			mergeAttributes(HTMLAttributes, {
				'data-ref-chip': '',
				'data-ref-type': refType || '',
				'data-snapshot': snapshot ? JSON.stringify(snapshot) : '',
				class: `tag-chip ref-chip ref-${refType || 'unknown'}`,
			}),
			displayText,
		];
	},

	addNodeView() {
		return SvelteNodeViewRenderer(RefChipView);
	},

	addKeyboardShortcuts() {
		return {
			Backspace: () =>
				this.editor.commands.command(({ tr, state }) => {
					let isRefChip = false;
					const { selection } = state;
					const { empty, anchor } = selection;

					if (!empty) return false;

					state.doc.nodesBetween(anchor - 1, anchor, (node, pos) => {
						if (node.type.name === this.name) {
							isRefChip = true;
							tr.insertText('', pos, pos + node.nodeSize);
							return false;
						}
					});

					return isRefChip;
				}),
		};
	},

	addProseMirrorPlugins() {
		return [
			Suggestion({
				editor: this.editor,
				pluginKey: RefSuggestionPluginKey,
				...this.options.suggestion,
			}),
		];
	},
});
