# Spec: Bookmarks

> **Status**: Approved — ready for implementation

## Goal

Users can capture moments of attention during annot sessions and reference them in future annotations. Agents can discover and retrieve bookmarks via MCP.

---

## Data Model

```rust
pub struct Bookmark {
    pub id: String,                    // "k3u3daxdd2wp" (12-char, generated)
    pub label: Option<String>,         // User-provided, or auto-derived
    pub created_at: DateTime<Utc>,
    pub project_path: Option<PathBuf>, // Project context (cwd at creation)
    pub snapshot: BookmarkSnapshot,
}

pub enum BookmarkSnapshot {
    /// Entire session content
    Session {
        source_type: SessionType,      // File | Diff | Content
        source_title: String,          // "auth.rs" or "cache-spec.md"
        context: String,               // Full document snapshot
    },
    /// Inline selection within session
    Selection {
        source_type: SessionType,
        source_title: String,
        context: String,               // Full document snapshot
        selected_text: String,         // What user selected
    },
}

pub enum SessionType {
    File,
    Diff,
    Content,
}
```

### Label Derivation

If user doesn't provide a label:

| Bookmark Type | Derivation |
|---------------|------------|
| Selection | First ~50 chars of `selected_text` |
| Session (markdown) | First `# heading` in content |
| Session (code/other) | `source_title` (filename) |

### ID Generation

12-character base32 (jj-style):
- Alphabet: `kpqrstvwxyz` + digits (no vowels, lowercase only)
- Example: `k3u3daxdd2wp`
- Prefix-matchable: `k3u` resolves if unambiguous

### ContentNode Extension

```rust
pub enum ContentNode {
    Text { text: String },
    Tag { id: String, name: String, instruction: String },
    Excalidraw { elements: Value, png: Option<String> },
    Media { data_url: String },
    // NEW:
    BookmarkRef {
        id: String,                    // Resolved bookmark ID
        label: String,                 // Cached for display
    },
}
```

---

## Storage

```
~/.config/annot/bookmarks.json
```

```json
[
  {
    "id": "k3u3daxdd2wp",
    "label": "SQL injection concern",
    "created_at": "2024-12-28T14:22:00Z",
    "project_path": "/Users/user/projects/auth-service",
    "snapshot": {
      "type": "selection",
      "source_type": "file",
      "source_title": "auth.rs",
      "context": "// Full file content here...\nfn authenticate(input: &str) {\n    let query = format!(\"SELECT * FROM users WHERE id={}\", input);\n    ...\n}\n// Rest of file...",
      "selected_text": "let query = format!(\"SELECT * FROM users WHERE id={}\", input);"
    }
  }
]
```

File locking: `fs4::FileExt::lock_exclusive()` pattern.

---

## UX: Creating Bookmarks

### Flow: Immediate Create → Optional Edit

All methods share the same pattern:
1. **Action** → Bookmark created immediately with auto-derived label
2. **Toast**: "Bookmarked as `k3u`" with `[e] edit` hint
3. **Press `e`** → Opens command palette to edit label (triggers last created bookmark)

### Method 1: Header Icon (Session Bookmark)

```
┌────────────────────────────────────────────┐
│  cache-spec.md                   [📌 icon] │
└────────────────────────────────────────────┘
```

- **Click**: Creates session bookmark (full document snapshot)
- **Label**: Auto-derived from heading or filename

### Method 2: Line Hover (Inline Bookmark)

User hovers over a line:
- **`b`**: Creates bookmark of that single line
- **`e`**: Opens palette to edit label

### Method 3: Multi-line Selection

User shift-drags to select multiple lines:
- **Release + `b`**: Creates bookmark of selection
- **`e`**: Opens palette to edit label

### Method 4: Command Palette

`: → boo → [n]ew`

Opens create form with fields:
- Label (required for this method)
- Creates session bookmark of current content

---

## UX: Referencing Bookmarks

### Trigger: `@` in Annotation Editor

User types `@` → Autocomplete popup:

```
┌────────────────────────────────────────────┐
│ @cach                                      │
├────────────────────────────────────────────┤
│ k3u  Cache strategy decision               │
│      cache-spec.md • Dec 30                │
├────────────────────────────────────────────┤
│ r2d  Auth caching layer                    │
│      auth-service/ • Dec 28                │
└────────────────────────────────────────────┘
```

**Display format**: `[ID]  [label]` with source/project hint below.

**Fuzzy match priority**:
1. ID prefix (`k3u` → `k3u3daxdd2wp`)
2. Label text
3. Selected text (for selection bookmarks)
4. Context content

**Project filtering**: Show all bookmarks, but sort current project first.

### After Selection

```
Same issue as @k3u
               └── chip: [📌 k3u · Cache strategy]
```

### Chip Rendering

TipTap `bookmarkChip` node:
- Display: `📌` + short ID (3-4 chars) + truncated label
- Hover: Full preview tooltip

### Preview Tooltip

```
┌─────────────────────────────────────────────┐
│ 📌 k3u3daxdd2wp                             │
│ Cache strategy decision                     │
│                                             │
│ Source: cache-spec.md                       │
│ Project: /Users/.../auth-service            │
│ Created: Dec 30, 2024                       │
├─────────────────────────────────────────────┤
│ ## Decision                                 │
│ We will use Redis for session caching       │
│ because:                                    │
│ - Built-in TTL support                      │
│ - Cluster mode for scaling                  │
│ ...                                         │
└─────────────────────────────────────────────┘
```

For selection bookmarks, show the selected text.

---

## UX: Managing Bookmarks

### Command Palette

`: → bookmarks` or `: → boo`

```
┌─────────────────────────────────────────────┐
│ : bookmarks                                 │
├─────────────────────────────────────────────┤
│ k3u  Cache strategy decision                │
│      cache-spec.md • Dec 30                 │
├─────────────────────────────────────────────┤
│ r2d  Auth caching layer                     │
│      auth-service/ • Dec 28                 │
├─────────────────────────────────────────────┤
│ t7x  let query = format!("SELECT..."        │
│      auth.rs • Dec 28                       │
└─────────────────────────────────────────────┘
  [n] new  [dd] delete  [e] edit  [Enter] view
```

**View (Enter)**: Modal with full snapshot
**Edit (e)**: Edit label (snapshot immutable)
**Delete (dd)**: Remove bookmark

### Delete Behavior

Same as tags: if referenced in current annotations, output includes it "one last time" on session close.

### CLI

```bash
annot bookmarks list                    # List all
annot bookmarks show k3u                # Full snapshot
annot bookmarks delete k3u              # Delete
annot bookmarks export k3u              # Markdown output
```

---

## Output Format

```
LEGEND:
  [# SECURITY] Review for security vulnerabilities
  [@ k3u] Cache strategy decision

SESSION:
  Exit: Apply

BOOKMARKS REFERENCED:
  [@ k3u] Cache strategy decision
    Source: cache-spec.md
    Project: /Users/.../auth-service
    Created: 2024-12-30
    ────────────────────────────────────
    ## Decision
    We will use Redis for session caching
    because:
    - Built-in TTL support
    - Cluster mode for scaling
    ────────────────────────────────────

---

current-file.rs:15-20:
    15 | fn get_session(id: &str) -> Session {
    16 |     cache.get(id)
     > [# SECURITY] Ensure TTL is set per @k3u decision
```

---

## MCP Interface

### get_bookmark

```typescript
{
  name: "get_bookmark",
  inputSchema: {
    properties: {
      id: { type: "string", description: "Full or prefix ID" }
    },
    required: ["id"]
  }
}

// Success:
{
  id: "k3u3daxdd2wp",
  label: "Cache strategy decision",
  created_at: "2024-12-30T14:22:00Z",
  project_path: "/Users/.../auth-service",
  snapshot: { type: "session", source_title: "cache-spec.md", context: "..." }
}

// Ambiguous:
{
  error: "ambiguous_id",
  candidates: [{ id: "...", label: "...", preview: "..." }, ...]
}
```

### list_bookmarks

```typescript
{
  name: "list_bookmarks",
  inputSchema: {
    properties: {
      limit: { type: "integer", default: 20 },
      search: { type: "string" },
      project: { type: "string", description: "Filter by project path" }
    }
  }
}
```

---

## Keyboard Shortcuts Summary

| Context | Key | Action |
|---------|-----|--------|
| Header | Click 📌 | Bookmark entire session |
| Line hover | `b` | Bookmark single line |
| After selection | `b` | Bookmark selection |
| After bookmark | `e` | Edit last created bookmark |
| Annotation editor | `@` | Reference bookmark autocomplete |
| Command palette | `: boo` | Bookmark management |

---

## Implementation Phases

### Phase 1: Foundation (3-4 days)
- Bookmark struct + storage
- ID generation
- Rust IPC commands
- Command palette namespace (list, view, delete)

### Phase 2: Creation UX (2-3 days)
- Header bookmark icon → immediate create
- Toast with `[e] edit` flow
- Command palette edit form

### Phase 3: References (4-5 days)
- TipTap bookmarkChip node
- `@` autocomplete with fuzzy search
- Hover preview
- Output expansion

### Phase 4: Line/Selection Bookmarks (2-3 days)
- `b` on line hover (`Cmd-b` to bookmark the entire session)
- `b` after selection
- Selection snapshot variant

### Phase 5: MCP + CLI (2-3 days)
- MCP tools
- CLI subcommand

---

## Decisions

| Decision | Choice |
|----------|--------|
| Identifier | ID only, prefix-matchable, lowercase |
| Label | Optional, auto-derived |
| Context | Always full document |
| Storage | Global user config |
| Project | Stored for context, not filtering |
| Creation UX | Immediate create → `e` to edit |
| Delete | Output "one last time" if referenced |

## Out of Scope (Future)

- Team/project-scoped bookmarks
- Bookmark versioning
- Staleness detection
- Auto-archival
- Cross-bookmark linking
