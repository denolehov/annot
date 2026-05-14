import { wrappingInputRule } from '@tiptap/core';
import { BulletList } from '@tiptap/extension-list';

/**
 * BulletList with a narrowed input rule.
 *
 * Stock tiptap matches `-`, `+`, AND `*` followed by a space (`/^\s*([-+*])\s$/`).
 * We keep only `- ` — `+` surprises users (nobody expects `+` to mean "bullet"),
 * and `*` cognitively collides with `*italic*`. Everything else about BulletList
 * (the node, toggle command, `Mod-Shift-8`) is unchanged.
 */
export const AnnotBulletList = BulletList.extend({
  addInputRules() {
    return [
      wrappingInputRule({
        find: /^\s*-\s$/,
        type: this.type,
      }),
    ];
  },
});
