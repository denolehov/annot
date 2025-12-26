# Frontend Composables Refactor Plan

## Status: Phase 1 Complete

### Completed (12 commits)

1. `composables/index.ts` - barrel exports
2. `useAnnotations.svelte.ts` - adapted with `getLines` callback
3. `useAnnotations.test.ts` - 13 tests
4. `useSelection.svelte.ts` - adapted without SourceLocation
5. `useSelection.test.ts` - 16 tests
6. `useExitModes.svelte.ts` - direct port
7. `useExitModes.test.ts` - 6 tests
8. `useKeyboard.svelte.ts` - direct port
9. `useKeyboard.test.ts` - 17 tests
10. `useContentTracking.svelte.ts` - direct port
11. `useContentTracking.test.ts` - 5 tests
12. `tiptap.ts` - added `SuggestionState<T>` and `createSuggestionRender<T>`

**Total: 57 passing tests**

---

## Remaining Work

### Phase 2: AnnotationEditor Refactor

1. **Port useAnnotationEditor.svelte.ts** (~237 lines)
   - Source: `git show new-portals:src/lib/composables/useAnnotationEditor.svelte.ts`
   - Extracts TipTap editor lifecycle, suggestion state, callbacks
   - Uses the new `SuggestionState<T>` and `createSuggestionRender<T>`

2. **Rewrite AnnotationEditor.svelte** (699 → ~360 lines)
   - Source: `git show new-portals:src/lib/AnnotationEditor.svelte`
   - Keep: Excalidraw window IPC, selection popover, scroll positioning
   - Move to composable: editor creation, suggestion handling

3. **Uncomment index.ts export**
   ```typescript
   export { useAnnotationEditor, type AnnotationEditorOptions } from './useAnnotationEditor.svelte';
   ```

### Phase 3: UI Components

4. **Create components/index.ts**
   ```typescript
   export { default as Header } from './Header.svelte';
   export { default as StatusBar } from './StatusBar.svelte';
   export { default as SessionEditor } from './SessionEditor.svelte';
   ```

5. **Port Header.svelte** (~136 lines)
   - Source: `git show new-portals:src/lib/components/Header.svelte`
   - Window header with breadcrumbs (diff/markdown/plain modes)

6. **Port StatusBar.svelte** (~35 lines)
   - Source: `git show new-portals:src/lib/components/StatusBar.svelte`
   - Exit mode button + keyboard hints

7. **Port SessionEditor.svelte** (~47 lines)
   - Source: `git show new-portals:src/lib/components/SessionEditor.svelte`
   - File-level comment wrapper

### Phase 4: Integration

8. **Wire composables into +page.svelte**
   - Replace inline state with composable calls
   - Replace inline header/footer with components
   - Target: 1332 → ~800 lines

### Phase 5: Documentation

9. **Document DisplayRange/SourceRange refactor opportunity**
   - Location: `docs/future-refactors.md` or similar
   - Issue: `AnnotationEntry.range` stores source lines but uses `Range` type (display indices)
   - Fix: Separate `DisplayRange` (UI) from `SourceRange` (backend)

---

## Key Adaptations Made

1. **useAnnotations**: Added `getLines` callback instead of `Range.source`
2. **useSelection**: Removed `SourceLocation`, uses display indices only
3. **useContentTracking.test.ts**: Removed `lines` from DiffMetadata, `portals` from MarkdownMetadata

---

## Commands

```bash
# Run all composable tests
pnpm test -- --run src/lib/composables/

# Type check
pnpm check

# Demo
pnpm demo

# View new-portals source
git show new-portals:src/lib/composables/useAnnotationEditor.svelte.ts
git show new-portals:src/lib/AnnotationEditor.svelte
git show new-portals:src/lib/components/Header.svelte
```

---

## jj Workflow

Each file gets its own commit:
```bash
jj new @ -m "description"
# make changes
# jj automatically tracks
```

Current chain: 12 commits off `main`
