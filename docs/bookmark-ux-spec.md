# Spec: Line/Selection Bookmarks UX

## Goal

Allow users to bookmark single lines or multi-line selections with minimal friction, keeping bookmarks and annotations as orthogonal actions.

---

## Design

### Single-line (Hover)

| Action             | Result                         |
| ------------------ | ------------------------------ |
| Hover + `b`        | Bookmark line, toast appears   |
| Hover + `c`        | Open annotation editor on line |
| `e` after bookmark | Edit bookmark label            |

==Unchanged from current implementation.==

### Multi-line (Shift-drag)

**On release (no modifier held):**
```
Selection committed → show two buttons:
  [📝] Annotate    [📌] Bookmark
  
Press 'c' → open annotation editor
Press 'b' → bookmark range, show toast
```

**With modifier held during drag:**

| Gesture                      | Result                                     |
| ---------------------------- | ------------------------------------------ |
| `Shift + drag + release`     | Show choice buttons (new)                  |
| `Shift + c + drag + release` | Immediately open editor (current behavior) |
| `Shift + b + drag + release` | Immediately bookmark range                 |

### Follow-up: `e` to edit

After bookmark created:
- Press `e` → command palette opens to edit bookmark label

---

## Visual Design

### Choice Buttons (after plain shift-drag-release)

```
     10 │ fn validate(input: &str) {       ┐
     11 │     let query = format!(...);    │ selected
     12 │     db.execute(&query);          │
     13 │ }                                ┘
              [📝 c]  [📌 b]  ◀── replaces [+] button
```

Or inline text hints:
```
     [c] annotate   [b] bookmark
```

### Bookmarked Lines Indicator

Lines that are bookmarked show visual indicator (you've already added `isLineBookmarked`).

---

## State Transitions

```
           Shift+drag
               │
    ┌──────────┼──────────┐
    │          │          │
  c held    b held    neither
    │          │          │
    ▼          ▼          ▼
 Editor    Bookmark    Choice
 Opens     Created     Buttons
              │          │
              │    ┌─────┼─────┐
              │    │     │     │
              │   'c'   'b'   Esc/click
              │    │     │     │
              │    ▼     ▼     ▼
              │  Editor  Bkmk  Clear
              │  Opens   Made
              │          │
              └────┬─────┘
                   │
                   ▼
              [e] = edit last
```

---

## Keyboard Detection During Drag

Need to track if `c` or `b` is held during the drag:

```typescript
// In useInteraction or parent
let dragModifier: 'c' | 'b' | null = null;

function handleKeyDown(e: KeyboardEvent) {
  if (phase === 'selecting') {
    if (e.key === 'c') dragModifier = 'c';
    if (e.key === 'b') dragModifier = 'b';
  }
}

function handlePointerUp() {
  if (dragModifier === 'c') {
    openEditor();
  } else if (dragModifier === 'b') {
    createBookmark();
  } else {
    showChoiceButtons();
  }
  dragModifier = null;
}
```

---

## Decisions

| Decision                      | Choice                                     |
| ----------------------------- | ------------------------------------------ |
| Default on shift-drag-release | Show choice buttons (not auto-open editor) |
| Shortcut during drag          | Hold `c` or `b` to pre-select action       |
| Visual indicator              | Two buttons replace [+] button             |
| Follow-up                     | `e` edits last bookmark label              |

---

## Scope

**In:**
- Choice buttons after selection
- `c`/`b` modifier during drag
- `e` follow-up for bookmark label

**Out:**
- Tab to switch modes in editor (dropped)
- Active selection persistence after action (can revisit)