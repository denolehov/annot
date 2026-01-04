import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from 'svelte-tiptap';
import Suggestion, { type SuggestionOptions } from '@tiptap/suggestion';
import { PluginKey } from '@tiptap/pm/state';
import RefChipView from '../nodeviews/RefChipView.svelte';
import type { RefSnapshot, Bookmark, AnnotationRefSnapshot, SectionInfo } from '$lib/types';

const RefSuggestionPluginKey = new PluginKey('refSuggestion');

/** Menu section header for grouped @ menu (visual separator, not selectable). */
export type RefSuggestionMenuHeader = { type: 'menu-header'; label: string };

/** Unified suggestion item for @ menu - annotation, bookmark, file, or heading section. */
export type RefSuggestionItem =
	| { type: 'annotation'; key: string; preview: string; content: import('$lib/types').ContentNode[] }
	| { type: 'bookmark'; bookmark: Bookmark }
	| { type: 'file'; path: string }
	| { type: 'heading'; section: SectionInfo }
	| RefSuggestionMenuHeader;

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
			refType: { default: null }, // 'annotation' | 'bookmark' | 'file' | 'heading'
			snapshot: { default: null }, // RefSnapshot (for annotation/bookmark)
			path: { default: null }, // string (for file refs)
			// Heading section attributes
			sectionLine: { default: null }, // number (source line of heading)
			sectionLevel: { default: null }, // number (1-6)
			sectionTitle: { default: null }, // string
		};
	},

	parseHTML() {
		return [
			{
				tag: 'span[data-ref-chip]',
				getAttrs: (dom) => {
					const element = dom as HTMLElement;
					const snapshotData = element.getAttribute('data-snapshot');
					const sectionLine = element.getAttribute('data-section-line');
					const sectionLevel = element.getAttribute('data-section-level');
					return {
						refType: element.getAttribute('data-ref-type') || null,
						snapshot: snapshotData ? JSON.parse(snapshotData) : null,
						path: element.getAttribute('data-path') || null,
						sectionLine: sectionLine ? parseInt(sectionLine, 10) : null,
						sectionLevel: sectionLevel ? parseInt(sectionLevel, 10) : null,
						sectionTitle: element.getAttribute('data-section-title') || null,
					};
				},
			},
		];
	},

	renderHTML({ node, HTMLAttributes }) {
		const refType = node.attrs.refType as string;
		const snapshot = node.attrs.snapshot as RefSnapshot | null;
		const path = node.attrs.path as string | null;
		const sectionLine = node.attrs.sectionLine as number | null;
		const sectionLevel = node.attrs.sectionLevel as number | null;
		const sectionTitle = node.attrs.sectionTitle as string | null;

		// Build display text based on type
		let displayText = '[@?]';
		if (refType === 'heading' && sectionLevel && sectionTitle) {
			displayText = `[H${sectionLevel} ${sectionTitle}]`;
		} else if (refType === 'file' && path) {
			const filename = path.split('/').pop() || path;
			displayText = `[@${filename}]`;
		} else if (snapshot) {
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
				'data-path': path || '',
				'data-section-line': sectionLine?.toString() || '',
				'data-section-level': sectionLevel?.toString() || '',
				'data-section-title': sectionTitle || '',
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
