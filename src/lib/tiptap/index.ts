// Public API of the tiptap editor module.
//
// The old 1255-line `tiptap.ts` god-file is gone; its concerns now live in
// sibling files (and extensions/). This barrel is the import surface for the
// rest of the app — `import { ... } from './tiptap'` resolves here.
//
// See docs/tiptap-rebuild-spec.md, slice #4.
export * from './content';
export * from './serialize';
export * from './suggestions';
export * from './replace-fence';
export * from './extensions/EditorShortcuts';
export * from './extensions/AnnotBulletList';
export * from './extensions/PasteHandlers';
export * from './extensions/SlashCommands';
