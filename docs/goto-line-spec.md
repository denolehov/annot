# Spec: Go-to-Line Command

## Goal

Enable vim-style `:n` and `:n-m` navigation in the command palette to jump to source line numbers and select ranges, with diff-aware and file-qualified variants.

---

## Design

### Syntax

| Pattern                 | Example         | Meaning                             |
| ----------------------- | --------------- | ----------------------------------- |
| `<line>`                | `:42`           | Go to source line 42, select it     |
| `<start>-<end>`         | `:42-50`        | Select source lines 42-50           |
| `n<line>`               | `:n42`          | Diff: go to **new** file line 42    |
| `o<line>`               | `:o42`          | Diff: go to **old** file line 42    |
| `<file>:<line>`         | `:config.rs:42` | Go to line 42 in file (fuzzy match) |
| `<file>:<prefix><line>` | `:lib.rs:n42`   | Combined file + diff qualifier      |

### State Machine

New `GOTO` state added to command palette reducer:

```
NAMESPACE_FILTER → GOTO   (when first char is digit, 'n', 'o', or letter followed by ':')
GOTO → IDLE               (Enter with valid target → execute, close)
GOTO → NAMESPACE_FILTER   (Backspace on empty → return to namespaces)
GOTO → IDLE               (Escape → close)
```

### State Type

```typescript
type GotoState = {
  kind: 'GOTO';
  query: string;
  parsed: GotoCommand | null;
  validation: GotoValidation;
};

type GotoCommand = 
  | { type: 'goto'; target: LineTarget }
  | { type: 'select'; start: LineTarget; end: LineTarget };

type LineTarget = {
  line: number;
  file?: string;
  version?: 'old' | 'new';
};

type GotoValidation = 
  | { valid: true; displayIdx: number; endDisplayIdx?: number; preview: string }
  | { valid: false; error: string };
```

### Resolution Logic

1. Parse query into `GotoCommand`
2. For each `LineTarget`:
   - Find matching line in `lines[]` array by source coordinates
   - If file qualifier: fuzzy match against `line.origin.path`
   - If diff mode + version: match against `oldLine` or `newLine`
   - Default diff version: `new`
3. Validate:
   - Line exists in view
   - Line is selectable (or snap to nearest selectable)
   - Range is within selection bounds
4. On Enter: call `setRange()`, scroll into view, close palette

### UI

**Valid state:** Show preview of target line(s)
**Invalid state:** Show error message (line doesn't exist, file not in view)
**Footer:** "Enter to select • Esc to cancel"

### Edge Case Handling

| Case                  | Behavior                                         |
| --------------------- | ------------------------------------------------ |
| Line out of bounds    | Error: "Line N doesn't exist (file has M lines)" |
| Non-selectable target | Snap to nearest selectable line                  |
| Range crosses bounds  | Constrain to valid selection bounds              |
| File not found        | Error: "file.rs not in current view"             |
| Invalid syntax        | Remain in GOTO, show parse hint                  |

---

## Decisions

- **`:42` selects, not hovers** — consistent with range behavior, survives mouse movement
- **Diff defaults to new file** — matches VSCode/GitHub, annotations are about proposed changes
- **Non-selectable snaps** — better UX than strict error
- **File match is fuzzy** — `:conf:42` matches `src/config.rs:42`

---

## Open Questions

- Multi-file diff without qualifier: default to focused file or require qualifier? (Defer: start with require)
- Tab-completion for file names: defer to future enhancement

---

## Scope

### In
- `:n` single line navigation + selection
- `:n-m` range selection
- `:nN` / `:oN` diff version qualifiers  
- `:file:n` file-qualified navigation
- Real-time validation + preview
- Scroll into view on execute

### Out
- Pattern search (`:/regex`)
- Jump history / bookmarks
- Tab-completion for files