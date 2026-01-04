# Spec: Session Timeline + Undo

## Goal

Give users visibility into their session, Cmd-Z undo, and make impossible UI states unrepresentable — all from a single unified infrastructure.

---

## Design

### Single Infrastructure: Immutable State Stack

```
┌─────────────────────────────────────────────────────────────┐
│                        AppState                             │
├─────────────────────────────────────────────────────────────┤
│  history: SessionData[]    ← source of truth (undoable)     │
│  historyIndex: number      ← current position               │
│  ui: UiState               ← ephemeral (not undoable)       │
│  overlay: OverlayState     ← ephemeral (not undoable)       │
│  narrative: NarrativeEntry[] ← debug log (not authoritative)│
└─────────────────────────────────────────────────────────────┘

Current state = history[historyIndex]
Timeline view = derived from current state
Undo = historyIndex--
Redo = historyIndex++
```

---

## Data Model

### SessionData (Undoable)

```typescript
interface SessionData {
  annotations: Map<string, AnnotationEntry>;  // key = "path:start-end"
  terraform: TerraformRegion[];
  sessionComment: ContentNode[] | null;
  selectedExitMode: string | null;
}

interface AnnotationEntry {
  path: string;
  range: Range;
  content: ContentNode[];
  sealed: boolean;
}

interface TerraformRegion {
  path: string;
  startLine: number;
  endLine: number;
  intent: TerraformIntent;
}
```

### UiState (Discriminated Union, Not Undoable)

```typescript
type UiState =
  | { phase: 'idle' }
  | { phase: 'hovering'; line: number }
  | { phase: 'selecting'; anchor: number; current: number }
  | { phase: 'committed'; range: Range; pendingChoice: boolean }
  | { phase: 'editing'; range: Range; annotationKey: string }
  | { phase: 'terraforming'; range: Range };
```

**Guarantees at compile time:**
- Can't hover while editing
- Can't have two selections
- Can't edit without a range

### OverlayState (Discriminated Union, Not Undoable)

```typescript
type OverlayState =
  | { type: 'NONE' }
  | { type: 'COMMAND_PALETTE'; state: CommandPaletteState }
  | { type: 'HELP' }
  | { type: 'TIMELINE'; selectedIndex: number; idBuffer: string };
```

**Guarantees:**
- Only one overlay open at a time
- Timeline and command palette are mutually exclusive

### NarrativeEntry (Debug Log, Not Authoritative)

```typescript
interface NarrativeEntry {
  id: string;              // homerow ID for display: "aa", "as"
  timestamp: number;
  label: string;           // "Created annotation at lib.rs:45-52"
  historyIndex: number;    // Which history state this corresponds to
}
```

**Purpose:** Human-readable "what happened" for debugging. NOT used for undo logic.

---

## Timeline View

Timeline is **derived from current state**, not from history or narrative.

```typescript
function deriveTimeline(state: SessionData): TimelineItem[] {
  const items: TimelineItem[] = [];
  
  // Annotations
  for (const [key, entry] of state.annotations) {
    items.push({
      id: generateStableId(key),
      type: 'annotation',
      location: { path: entry.path, range: entry.range },
      preview: extractPreview(entry.content),
    });
  }
  
  // Terraform regions
  for (const region of state.terraform) {
    items.push({
      id: generateStableId(`tf:${region.path}:${region.startLine}`),
      type: 'terraform',
      location: { path: region.path, range: { start: region.startLine, end: region.endLine } },
      preview: describeIntent(region.intent),
    });
  }
  
  // Session comment (if exists)
  if (state.sessionComment) {
    items.push({
      id: 'session-comment',
      type: 'session_comment',
      location: { type: 'session' },
      preview: extractPreview(state.sessionComment),
    });
  }
  
  return items.sort((a, b) => a.createdAt - b.createdAt);
}
```

### Timeline UI

```
┌─────────────────────────────────────────────────────────────┐
│  Session (4 items)                                  [Esc]   │
├─────────────────────────────────────────────────────────────┤
│  aa  ●  lib.rs:45-52    "UserAuth struct..."                │
│  as  ◆  lib.rs:60-80    "expand validation"                 │
│  ad  ●  main.rs:10-15   "TODO: handle err..."               │
│  af  §  session         "Focus on error ha..."              │
├─────────────────────────────────────────────────────────────┤
│  [Enter] edit   [d] delete   [Esc] close                    │
└─────────────────────────────────────────────────────────────┘
```

**Icons:** `●` annotation, `◆` terraform, `§` session comment

---

## Undo/Redo

### Core Operations

```typescript
function currentState(): SessionData {
  return history[historyIndex];
}

function pushState(newState: SessionData, label: string) {
  // Truncate any "future" states
  history = history.slice(0, historyIndex + 1);
  history.push(newState);
  historyIndex++;
  
  // Record in narrative (for debugging)
  narrative.push({
    id: generateNarrativeId(),
    timestamp: Date.now(),
    label,
    historyIndex,
  });
}

function undo(): boolean {
  if (historyIndex > 0) {
    historyIndex--;
    syncToBackend(currentState());
    return true;
  }
  return false;
}

function redo(): boolean {
  if (historyIndex < history.length - 1) {
    historyIndex++;
    syncToBackend(currentState());
    return true;
  }
  return false;
}
```

### Mutation Pattern

All mutations follow this pattern:

```typescript
function createAnnotation(path: string, range: Range, content: ContentNode[]) {
  const current = currentState();
  const key = `${path}:${range.start}-${range.end}`;
  
  // Create new state (immutable)
  const newState: SessionData = {
    ...current,
    annotations: new Map(current.annotations).set(key, {
      path,
      range,
      content,
      sealed: false,
    }),
  };
  
  pushState(newState, `Created annotation at ${path}:${range.start}-${range.end}`);
  
  // Sync to backend
  invoke('upsert_annotation', { path, startLine: range.start, endLine: range.end, content });
}

function deleteAnnotation(path: string, range: Range) {
  const current = currentState();
  const key = `${path}:${range.start}-${range.end}`;
  
  const newAnnotations = new Map(current.annotations);
  newAnnotations.delete(key);
  
  const newState: SessionData = {
    ...current,
    annotations: newAnnotations,
  };
  
  pushState(newState, `Deleted annotation at ${path}:${range.start}-${range.end}`);
  
  invoke('delete_annotation', { path, startLine: range.start, endLine: range.end });
}
```

### Backend Sync on Undo/Redo

When undo/redo changes state, sync entire state to backend:

```typescript
async function syncToBackend(state: SessionData) {
  // Option 1: Bulk restore (preferred if backend supports it)
  await invoke('restore_session_state', { state });
  
  // Option 2: Clear + replay (fallback)
  // await invoke('clear_session');
  // for (const [key, entry] of state.annotations) {
  //   await invoke('upsert_annotation', { ... });
  // }
  // for (const region of state.terraform) {
  //   await invoke('upsert_terraform', { ... });
  // }
}
```

---

## Keyboard Handling

### Global (always active)

| Key           | Action                  |
| ------------- | ----------------------- |
| `Cmd-Z`       | Undo (if not in editor) |
| `Cmd-Shift-Z` | Redo (if not in editor) |

### When no overlay

| Key   | Action               |
| ----- | -------------------- |
| `t`   | Open timeline        |
| `:`   | Open command palette |
| `?`   | Open help            |

### In timeline overlay

| Key     | Action                 |
| ------- | ---------------------- |
| `↑/k`   | Select previous        |
| `↓/j`   | Select next            |
| `[a-l]` | Type ID to jump        |
| `Enter` | Edit selected item     |
| `d`     | Delete selected item   |
| `Esc`   | Clear buffer, or close |

### Cmd-Z Context Awareness

```typescript
function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'z') {
    // Let editors handle their own undo
    const active = document.activeElement;
    const isInEditor = active?.closest('.ProseMirror') ||
                       active?.tagName === 'TEXTAREA' ||
                       active?.tagName === 'INPUT';
    
    if (isInEditor) return; // Let editor handle it
    
    e.preventDefault();
    if (e.shiftKey) {
      redo();
    } else {
      undo();
    }
  }
}
```

---

## UI State Transitions

### UiState Reducer

```typescript
function uiReducer(state: UiState, action: UiAction): UiState {
  switch (action.type) {
    case 'HOVER':
      if (state.phase !== 'idle') return state;
      return { phase: 'hovering', line: action.line };
      
    case 'HOVER_END':
      if (state.phase !== 'hovering') return state;
      return { phase: 'idle' };
      
    case 'START_SELECT':
      return { phase: 'selecting', anchor: action.line, current: action.line };
      
    case 'EXTEND_SELECT':
      if (state.phase !== 'selecting') return state;
      return { ...state, current: action.line };
      
    case 'COMMIT_SELECT':
      if (state.phase !== 'selecting') return state;
      return { phase: 'committed', range: normalize(state.anchor, state.current), pendingChoice: true };
      
    case 'OPEN_EDITOR':
      if (state.phase !== 'committed') return state;
      return { phase: 'editing', range: state.range, annotationKey: action.key };
      
    case 'CLOSE_EDITOR':
      if (state.phase !== 'editing') return state;
      return { phase: 'idle' };
      
    case 'OPEN_TERRAFORM':
      if (state.phase !== 'committed') return state;
      return { phase: 'terraforming', range: state.range };
      
    case 'CLOSE_TERRAFORM':
      if (state.phase !== 'terraforming') return state;
      return { phase: 'idle' };
      
    case 'RESET':
      return { phase: 'idle' };
      
    default:
      return state;
  }
}
```

### OverlayState Reducer

```typescript
function overlayReducer(state: OverlayState, action: OverlayAction): OverlayState {
  switch (action.type) {
    case 'OPEN_TIMELINE':
      if (state.type !== 'NONE') return state;
      return { type: 'TIMELINE', selectedIndex: 0, idBuffer: '' };
      
    case 'OPEN_COMMAND_PALETTE':
      if (state.type !== 'NONE') return state;
      return { type: 'COMMAND_PALETTE', state: initialCommandPaletteState() };
      
    case 'OPEN_HELP':
      if (state.type !== 'NONE') return state;
      return { type: 'HELP' };
      
    case 'CLOSE':
      return { type: 'NONE' };
      
    // Timeline-specific actions
    case 'TIMELINE_MOVE':
      if (state.type !== 'TIMELINE') return state;
      return { ...state, selectedIndex: action.index };
      
    case 'TIMELINE_TYPE':
      if (state.type !== 'TIMELINE') return state;
      return { ...state, idBuffer: state.idBuffer + action.char };
      
    case 'TIMELINE_CLEAR_BUFFER':
      if (state.type !== 'TIMELINE') return state;
      return { ...state, idBuffer: '' };
      
    default:
      return state;
  }
}
```

---

## ID Generation

### Stable IDs for Timeline Items

```typescript
// Items keep stable IDs based on their identity, not array position
function generateStableId(key: string): string {
  // Use a Map to track assigned IDs
  if (!idMap.has(key)) {
    idMap.set(key, nextHomerowId());
  }
  return idMap.get(key)!;
}

const HOMEROW = 'asdfjkl';
let idCounter = 0;

function nextHomerowId(): string {
  const idx = idCounter++;
  const first = HOMEROW[Math.floor(idx / HOMEROW.length) % HOMEROW.length];
  const second = HOMEROW[idx % HOMEROW.length];
  return `${first}${second}`;
}
```

IDs are **never recycled** within a session.

---

## Memory Management

### Estimates

| Session Type           | State Size   | 100-step History   |
| ---------------------- | ------------ | ------------------ |
| Light (5 annotations)  | ~1 KB        | ~100 KB            |
| Medium (8 annotations) | ~3 KB        | ~300 KB            |
| Heavy (10 annotations) | ~8 KB        | ~800 KB            |

### Media Handling

Large blobs (pasted images) should be stored separately:

```typescript
interface SessionData {
  annotations: Map<string, AnnotationEntry>;
  terraform: TerraformRegion[];
  sessionComment: ContentNode[] | null;
  selectedExitMode: string | null;
  mediaBlobs: Map<string, string>;  // blobId → dataUrl (deduplicated)
}

// ContentNode references blob by ID, not inline data
type ContentNode =
  | { type: 'text'; text: string }
  | { type: 'media'; blobId: string }  // Reference, not data
  | ...
```

### History Cap

```typescript
const MAX_HISTORY = 100;

function pushState(newState: SessionData, label: string) {
  history = history.slice(0, historyIndex + 1);
  history.push(newState);
  historyIndex++;
  
  // Cap history
  if (history.length > MAX_HISTORY) {
    const excess = history.length - MAX_HISTORY;
    history = history.slice(excess);
    historyIndex -= excess;
  }
  
  // ...
}
```

---

## New Backend IPC

### restore_session_state

```rust
#[tauri::command]
fn restore_session_state(
    state: State<'_, AppState>,
    session_data: SessionDataDto,
) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    
    // Clear and replace
    session.annotations.clear();
    for (key, entry) in session_data.annotations {
        session.annotations.insert(key.parse()?, entry.into());
    }
    
    session.terraform_regions.clear();
    session.terraform_regions.extend(session_data.terraform.into_iter().map(Into::into));
    
    session.comment = session_data.session_comment.map(Into::into);
    session.selected_exit_mode_id = session_data.selected_exit_mode;
    
    Ok(())
}
```

---

## Decisions

| Decision                                                 | Rationale                                         |
| -------------------------------------------------------- | ------------------------------------------------- |
| Immutable state stack (Option C)                         | Simplest mental model; undo = index movement      |
| Separate undoable (SessionData) from ephemeral (UiState) | Cmd-Z undoes data changes, not "I opened a modal" |
| Discriminated unions for UI state                        | Impossible states unrepresentable at compile time |
| Timeline derived from current state                      | No drift between timeline and actual state        |
| Narrative log for debugging only                         | Audit trail without coupling to undo logic        |
| Stable IDs never recycled                                | Users can memorize "as = the database annotation" |
| Media stored as blob references                          | Prevents history bloat from pasted images         |
| Backend bulk restore                                     | Simpler than diffing; idempotent sync             |

---

## Scope

### In

- `SessionData` type with annotations, terraform, comment, exit mode
- `UiState` discriminated union with reducer
- `OverlayState` discriminated union with reducer
- History stack with push/undo/redo
- Timeline view derived from current state
- Timeline UI component
- Cmd-Z/Cmd-Shift-Z global handling
- `restore_session_state` backend IPC
- Media blob deduplication
- History cap (100 entries)

### Out

- Cross-session persistence of history
- Collaborative undo
- Branching history (git-style)
- Per-keystroke undo (editor handles that)
- Undo for bookmarks (they persist to disk separately)