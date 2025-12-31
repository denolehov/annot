# Spec: Context-Based State Management

## Goal

Eliminate prop drilling by introducing Svelte context, reducing `+page.svelte` from ~1000 lines to ~400 lines while preserving all existing functionality.

## Design

### Context Definition

`src/lib/context/annot-context.svelte.ts`:

```typescript
export interface AnnotContext {
  // Core state (read-only access via getters)
  readonly lines: Line[];
  readonly metadata: ContentMetadata;
  readonly tags: Tag[];

  // Composable instances
  interaction: ReturnType<typeof useInteraction>;
  annotations: ReturnType<typeof useAnnotations>;
  exitModes: ReturnType<typeof useExitModes>;
  search: ReturnType<typeof useSearch>;
  mermaid: ReturnType<typeof useMermaid>;

  // Shared actions
  showToast: (message: string) => void;
}

const ANNOT_CONTEXT = Symbol('annot');

export function setAnnotContext(ctx: AnnotContext) {
  setContext(ANNOT_CONTEXT, ctx);
}

export function getAnnotContext(): AnnotContext {
  return getContext<AnnotContext>(ANNOT_CONTEXT);
}
```

### Provider Component

`src/lib/context/AnnotProvider.svelte`:

```svelte
<script lang="ts">
  import { setAnnotContext } from './annot-context.svelte';

  interface Props {
    lines: Line[];
    metadata: ContentMetadata;
    tags: Tag[];
    children: Snippet;
  }

  let { lines, metadata, tags, children }: Props = $props();

  // Instantiate composables once
  const interaction = useInteraction({ ... });
  const annotations = useAnnotations({ getLines: () => lines });
  const exitModes = useExitModes();
  const search = useSearch(() => lines, scrollToDisplayIndex);
  const mermaid = useMermaid({ ... });

  setAnnotContext({
    get lines() { return lines; },
    get metadata() { return metadata; },
    get tags() { return tags; },
    interaction,
    annotations,
    exitModes,
    search,
    mermaid,
    showToast,
  });
</script>

{@render children()}
```

### Component Tree (After)

```
+page.svelte (~400 lines)
├── UI state (modals, zoom, scroll)
├── Data loading (onMount → invoke)
├── useKeyboard (page-level event router)
└── <AnnotProvider {lines} {metadata} {tags}>
      ├── <Header />           ← ctx.metadata, ctx.exitModes
      ├── <SearchBar />        ← ctx.search
      ├── <Portal />           ← ctx.interaction, ctx.annotations
      ├── <CodeBlock />        ← ctx.interaction, ctx.annotations
      ├── <RegularLines />     ← ctx.interaction, ctx.annotations
      │     └── <AnnotationSlot rangeKey="..." />
      ├── <StatusBar />        ← ctx.exitModes
      └── <SessionEditor />    ← ctx.tags
    </AnnotProvider>
```

### Consuming Context (Example)

Before (AnnotationSlot with 10+ props):
```svelte
<script lang="ts">
  interface Props {
    annotationState, interaction, tags, allowsImagePaste,
    pendingTagInsertion, onUpdate, onDismiss, ...
  }
</script>
```

After (AnnotationSlot with 1 prop):
```svelte
<script lang="ts">
  import { getAnnotContext } from '$lib/context/annot-context.svelte';

  interface Props { rangeKey: string; }
  let { rangeKey }: Props = $props();

  const ctx = getAnnotContext();
  // Access: ctx.annotations, ctx.interaction, ctx.tags
</script>
```

### What Stays in +page.svelte

```typescript
// UI-local state (not shared, no context needed)
let commandPaletteOpen = $state(false);
let sessionEditorOpen = $state(false);
let saveModalOpen = $state(false);
let toastMessage = $state<string | null>(null);
let contentZoom = $state(1.0);
let scrollTop = $state(0);

// Keyboard stays at page level (stateless event router)
const keyboard = useKeyboard(handlers, stateQueries);

// Tag creation flow stays at page level (cross-cutting modal coordination)
let pendingTagCreation = $state<...>(null);
let pendingTagInsertion = $state<...>(null);
```

## Decisions

| Decision | Rationale |
|----------|-----------|
| Single context | One `getAnnotContext()` call vs multiple context lookups |
| Direct context reads | No intermediate orchestrator needed; virtualization is simple |
| `useKeyboard` at page level | Stateless event router, inherently page-scoped |
| Tag creation flow at page level | Cross-cutting modal coordination, context doesn't simplify |
| Getters for reactive state | Ensures consumers see reactive updates |
| Props for identity only | `rangeKey` needed to identify *which* annotation |

## Open Questions

None — all resolved during design.

## Scope

### In

- `annot-context.svelte.ts` — context type + accessors
- `AnnotProvider.svelte` — provider component
- Migrate: AnnotationSlot, Portal, CodeBlock, RegularLines, Header, StatusBar
- Slim `+page.svelte` to ~400 lines

### Out

- Composable internals unchanged
- Tag creation flow unchanged (stays in page)
- No new features

## Migration Path

| Phase | Changes | Risk |
|-------|---------|------|
| 1. Create context | Add `annot-context.svelte.ts`, `AnnotProvider.svelte` | None — additive |
| 2. Wrap page | Insert `<AnnotProvider>` in `+page.svelte` | Low |
| 3. Migrate AnnotationSlot | Remove props, use context | Medium — most nested |
| 4. Migrate Portal/CodeBlock/RegularLines | Remove cascaded props | Medium |
| 5. Migrate Header/StatusBar | Remove exit mode props | Low |
| 6. Cleanup | Remove unused props, slim page | Low |
