# Woven Annotation System: Refined Specification

## Philosophy

> The user didn't "reply" to the AI. They reached into the content, physically altered it, expanded it with new logic, and left rejected options as faint historical records.

Annotations are ==modifications woven into the document fabric==, not comments stuck on the side.

---

## Core Concept: The Rift

When a user selects lines, a **rift** opens — an indented region that grows from the selection point.

### Visual Language

| Element      | Old (Popover)            | New (Rift)                    |
| ------------ | ------------------------ | ----------------------------- |
| Container    | Box with border + arrow  | Left border only (3px accent) |
| Background   | Distinct cream color     | Transparent (same as doc)     |
| Position     | Floating after selection | In document flow              |
| Relationship | Points AT content        | IS content                    |

The left border says: =="This indented region is human-added, belonging to the lines above."==

---

## States

### Active State (Editing)

```
   43 │   let result = dangerous_call(input);        ░░░░  ← tinted (red, ~20%)
   44 │   process(result);                           ░░░░
      │
      ┃   [# SECURITY]
      ┃
      ┃   ╭─ replacement ─────────────────────────────────╮
      ┃   │  let sanitized = sanitize(input);            │
      ┃   │  let result = safe_call(sanitized)?;         │
      ┃   ╰───────────────────────────────────────────────╯
      ┃
      ┃   This prevents injection attacks.
      ┃   █
      ┃
   45 │   fn next_function() { ... }
```

**Elements:**
- Original lines get subtle red tint (only while rift is active)
- `┃` left border in accent color
- Replacement box with green accent (the only "boxed" element — it's code)
- Prose flows naturally
- No duplication of original lines

### Sealed State

```
   43 │   let result = dangerous_call(input);
   44 │   process(result);
      ┃── 🔄 SECURITY · 2→3 lines · prevents injection...
   45 │   fn next_function() { ... }
```

**Elements:**
- ==Tint fades away== — clean look
- Single summary line with left border
- Shows: icon + primary tag + line delta (for replace) + preview
- Click to expand

---

## Range Indication

For multi-line selections, a bracket in the margin:

```
   42 │   fn validate(input: &str) -> Result<()> {       ┐
   43 │       let parsed = parse_input(input)?;          │
   44 │       validate_bounds(parsed)?;                  │
   45 │   }                                              ┘
      ┃── 🏷 SECURITY · bounds checking needed...
```

The bracket (`┐│┘`) shows the annotated range. Persists in both active and sealed states.

---

## Replace Blocks: No Duplication

### The Change

**Old**: ReplaceBlock shows original + replacement (duplicates code above)
**New**: ReplaceBlock shows ==replacement only== (original visible in tinted lines)

### Active Replace

```
   43 │   let result = dangerous_call(input);        ░░░░  ← tinted
   44 │   process(result);                           ░░░░
      │
      ┃   ╭─ replacement ─────────────────────────────────╮
      ┃   │  let sanitized = sanitize(input);            │  ← green tint
      ┃   │  let result = safe_call(sanitized)?;         │
      ┃   │  process(result);                            │
      ┃   ╰───────────────────────────────────────────────╯
      ┃
      ┃   Additional context here...
```

### Sealed Replace

```
      ┃── 🔄 SECURITY · 2→3 lines · prevents injection...
```

Line delta (`2→3`) indicates replacement exists. Click to see full replacement.

---

## Polymorphic Content Types

The rift adapts presentation based on content:

| Content Type   | Active Display              | Sealed Display                    |
| -------------- | --------------------------- | --------------------------------- |
| Text only      | Prose with tags             | `🏷 TAG · preview...`              |
| Replace        | Replacement box + prose     | `🔄 TAG · N→M lines · preview...` |
| Diagram        | Chip reference + prose      | `📐 TAG · [diagram] · preview...` |
| Mixed          | All elements flow naturally | Primary indicator + `+N more`     |

---

## Keyboard Flow

| Key          | Action                            |
| ------------ | --------------------------------- |
| Click lines  | Select range, anchor point        |
| Start typing | Rift opens                        |
| `#`          | Tag autocomplete                  |
| `/replace`   | Insert replacement block          |
| `/diagram`   | Open Excalidraw (separate window) |
| `Escape`     | Seal rift                         |
| `⌘⏎`         | Seal and focus next               |
| `Tab`        | Jump to next rift                 |
| `Shift+Tab`  | Previous rift                     |
| Click sealed | Expand rift                       |

---

## Edge Case Matrix

| Case                               | Detection                     | Handling                                              |
| ---------------------------------- | ----------------------------- | ----------------------------------------------------- |
| Overlapping ranges (40-50 & 45-55) | Selection intersects existing | Merge dialog: "Extend existing or create separate?"   |
| Identical ranges                   | Exact match on select         | Append to existing annotation (new section)           |
| Adjacent rifts (no code between)   | Consecutive sealed lines      | Visual separator between rifts                        |
| Long replacement (>15 lines)       | Line count check              | Rift expands; add sticky context header if very long  |
| Scroll drift on seal/unseal        | Viewport anchor changes       | Restore scroll position relative to focused element   |
| Rapid open/close                   | Multiple operations <100ms    | Debounce + CSS transitions                            |
| 20+ annotations in file            | Count threshold               | Offer "collapse all" / overview mode                  |
| Diagram too large for inline       | Excalidraw dimensions         | Diagrams always open in separate window; chip in rift |
| Empty rift on blur                 | No content after cancel       | Auto-delete rift                                      |
| Replace validation fail            | Original === replacement      | Shake + refocus (existing behavior)                   |
| Multi-tag annotation               | Multiple `[# TAG]` chips      | Summary shows primary tag + count                     |

---

## Output Format Compatibility

The output format remains unchanged:

```
LEGEND:
  [# SECURITY] Review for injection vulnerabilities

SESSION:
  Exit Mode Name (instruction)

---

file.rs:43-44:
    42 | fn validate(input: &str) {
>  43 |     let result = dangerous_call(input);
>  44 |     process(result);
         └──> [# SECURITY] 
         [REPLACE]
         ```diff
         - let result = dangerous_call(input);
         - process(result);
         + let sanitized = sanitize(input);
         + let result = safe_call(sanitized)?;
         + process(result);
         ```
         This prevents injection attacks.
```

The woven UI is a ==presentation layer== — the structured output Claude receives is identical.

---

## Implementation Phases

### Phase 1: Visual Rift (CSS only)
- Remove box border + arrow from `.annotation-editor`
- Add 3px left border accent
- Adjust padding/margins for indent effect
- **Effort**: 1-2 hours

### Phase 2: Line Tinting
- Add `hasReplaceNode(json)` helper
- Add `linesWithReplace` derived state in +page.svelte
- Add `.has-replace` class with red tint (~20% opacity)
- Tint only when rift is active (not sealed)
- **Effort**: 2-3 hours

### Phase 3: Replacement-Only Display
- Modify ReplaceBlock to show replacement section only
- Modify ReplacePreview for sealed summary
- Update styling (green accent for replacement)
- **Effort**: 3-4 hours

### Phase 4: Sealed Compaction
- Add `summarizeAnnotation(content)` helper
- Render compact summary when sealed
- Preserve click-to-expand behavior
- **Effort**: 2-3 hours

### Phase 5: Range Brackets
- Add bracket indicators in gutter for multi-line ranges
- Persist in both active and sealed states
- **Effort**: 2-3 hours

**Total**: ~12-15 hours

---

## Files to Modify

| File                                          | Changes                                           |
| --------------------------------------------- | ------------------------------------------------- |
| `src/styles/components/annotation-editor.css` | Remove box/arrow, add left-border, sealed summary |
| `src/styles/components/code-viewer.css`       | Add `.has-replace` tint, range brackets           |
| `src/styles/components/chips.css`             | Update replace-block for replacement-only display |
| `src/lib/tiptap.ts`                           | Modify ReplaceBlock/ReplacePreview NodeViews      |
| `src/routes/+page.svelte`                     | Add `linesWithReplace` state, bracket rendering   |
| `src/lib/AnnotationEditor.svelte`             | Add sealed summary mode                           |

---

## Open Items

1. **"Show original" toggle**: Should active replace have a way to peek at original lines? Or is the tint + position sufficient context?

2. **Overlap policy**: Strictly forbid, or allow with merge prompt?

3. **Animation**: Should rift open/close animate, or be instant?