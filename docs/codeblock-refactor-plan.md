# CodeBlock Refactor: Implementation Plan

## Context

The `CodeBlock.svelte` component was added to render fenced code blocks with specialized styling. However, it duplicates line-rendering logic from `+page.svelte` and `Portal.svelte`. This document captures the agreed design decisions for a future refactor.

---

## Design Decisions

### 1. EditorLine Component

**Decision:** Create `EditorLine.svelte` that handles the shared line structure.

**What it handles:**
- `.line` wrapper with `selected`, `annotated` classes
- `.add-btn` (conditionally rendered via prop, not CSS)
- `.gutter` with selection events
- `.code` content via slot
- Data attributes for styling hooks

**Props interface:**
```typescript
interface EditorLineProps {
  line: Line;
  displayIndex: number;
  isSelected: boolean;
  isAnnotated: boolean;
  rangeKey: string | null;
  showAddButton?: boolean;
  variant?: 'regular' | 'portal' | 'codeblock';
  semantic?: string;  // 'header' | 'content' | 'footer' | 'fence'
  
  // Event callbacks (pre-bound to displayIndex)
  onGutterMouseDown: (e: MouseEvent) => void;
  onGutterClick: () => void;
  onAddMouseDown: (e: MouseEvent) => void;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  
  // Slots
  gutterContent?: Snippet;
  children: Snippet;  // code content
  annotationSlot: Snippet<[rangeKey: string | null]>;
}
```

---

### 2. Styling Approach

**Decision:** Use `variant` prop + data attributes for theming.

```svelte
<div 
  class="line"
  class:selected
  class:annotated
  data-variant={variant}
  data-semantic={semantic}
>
```

CSS uses attribute selectors:
```css
.line[data-variant="portal"][data-semantic="header"] { ... }
.line[data-variant="codeblock"][data-semantic="fence"] { ... }
```

**Rationale:** Clean API without class prop explosion. New variants = new CSS rules.

---

### 3. Shared Logic Extraction

**Decision:** Create `useLineGroup` composable for shared behavior.

```typescript
// src/lib/composables/useLineGroup.svelte.ts
export function useLineGroup(options: {
  selection: Range | null;
  annotations: Map<string, JSONContent>;
  lastSelectedLine: number | null;
  isDragging: boolean;
}) {
  function isSelected(idx: number): boolean { ... }
  function hasAnnotation(idx: number): boolean { ... }
  function getAnnotationAtLine(idx: number): { key: string; content: JSONContent } | null { ... }
  function computeRangeKey(idx: number): string | null { ... }
  
  return { isSelected, hasAnnotation, getAnnotationAtLine, computeRangeKey };
}
```

Both `Portal.svelte` and `CodeBlock.svelte` use this composable.

---

### 4. Annotation Slot

**Decision:** Keep slot pattern, but extract shared rendering to `AnnotationSlot.svelte`.

```svelte
<!-- AnnotationSlot.svelte -->
<script lang="ts">
  interface Props {
    rangeKey: string | null;
    annotationState: ...;
    selectionState: ...;
    // ... other shared props
  }
</script>

{#if rangeKey}
  {#key rangeKey}
    <AnnotationEditor ... />
  {/key}
{/if}
```

Consumer usage:
```svelte
{#snippet annotationSlot(rangeKey)}
  <AnnotationSlot {rangeKey} {annotationState} {selectionState} {tags} ... />
{/snippet}
```

---

### 5. Keep Portal and CodeBlock Separate

**Decision:** Do NOT unify into a generic `LineGroup` component.

**Rationale:**
- Portal = embedded file reference (header shows path, label)
- CodeBlock = fenced code (header shows language, mermaid button)
- Semantic purposes are distinct
- Forcing unification would create over-abstraction

---

## Migration Steps

1. Create `src/lib/composables/useLineGroup.svelte.ts`
2. Create `src/lib/components/EditorLine.svelte`
3. Create `src/lib/components/AnnotationSlot.svelte`
4. Update `+page.svelte` regular line loop
5. Update `Portal.svelte` to use EditorLine + useLineGroup
6. Update `CodeBlock.svelte` to use EditorLine + useLineGroup
7. Remove duplicated CSS from Portal/CodeBlock (keep only variant-specific styles)
8. Test all three rendering paths

---

## Files Changed (Current Session)

### New files:
- `src/lib/components/embedded/CodeBlock.svelte`

### Modified files:
- `src/styles/tokens.css` — added `--bg-code-block`, `--border-code`, `--codeblock-pattern-bg`
- `src/lib/line-utils.ts` — added `isCodeBlockFence`, `isCodeBlockContent`, `isCodeBlockLine`
- `src/routes/+page.svelte` — added codeblock segment type, CodeBlock rendering

---

## Current State

The CodeBlock component is **functional** but contains duplicated logic. The refactor described above is **deferred** — pick up in a future session.