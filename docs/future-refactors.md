# Future Refactoring Opportunities

## DisplayRange vs SourceRange

**Issue**: `AnnotationEntry.range` stores source lines but uses the `Range` type which represents display indices. This conflation leads to confusion when working with diffs (where display and source indices differ).

**Current state**:
- `Range` type has `start` and `end` (display indices)
- Annotations store source lines using same type
- Conversion happens implicitly via `getLines` callback

**Proposed fix**:
- Create `SourceRange { start_line: number, end_line: number }` for backend
- Keep `DisplayRange { start: number, end: number }` for UI
- Make conversion explicit: `displayToSource(range)` / `sourceToDisplay(range)`

**Affected files**:
- `src/lib/composables/useAnnotations.svelte.ts`
- `src/lib/composables/useSelection.svelte.ts`
- `src/lib/types.ts`
