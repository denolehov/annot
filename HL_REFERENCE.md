# HL Reference (Original Go Implementation)

This document captures the complete feature set of the original [hl](https://github.com/denolehov/hl) implementation. Use this as reference when building annot instead of re-exploring the Go codebase.

**Source**: `/Users/denolehov/_p/golang/hl`

---

## 1. CORE USER-FACING FEATURES

### A. Annotation Workflow

- **Line Selection**: Click line numbers to select single lines or ranges
- **Inline Editor**: Click on a selected range to open a rich text editor
- **Range Support**: Annotate from line X to line Y (e.g., "45-52")
- **Multi-line Annotations**: Each annotation can contain multiple content nodes (see below)

### B. Content Node Types (Composable Annotation Building Blocks)

Annotations are built from these discrete node types:

1. **Text Node** (`type: "text"`)
   - Raw markdown or plain prose
   - Rendered as-is in output
   - Example: "Token validation is fragile..."

2. **Tag Chip** (`type: "chip"`)
   - Composable mini-prompts: `[# TAG_NAME]`
   - User-defined with persistent instructions
   - Examples: `[# SECURITY]`, `[# TODO]`, `[# ELABORATE]`
   - Stored globally in `~/.config/hl/tags.json`
   - Can be nested in prose: "Please [# ELABORATE] on this error handling"
   - ID: 12-character stable alphanumeric ID (for persistence)

3. **Excalidraw Chip** (`type: "excalidraw"`)
   - Embedded diagrams with full Excalidraw drawing capabilities
   - Persisted as JSON elements + base64 PNG image
   - Rendered as `[EXCALIDRAW]` with JSON in CLI, as `{{MEDIA:N}}` placeholders in output
   - MCP can receive as base64 image

4. **Media Chip** (`type: "media"`)
   - Pasted images (clipboard paste)
   - Stored as base64 data URL
   - Rendered as `[IMAGE]` in CLI, `{{MEDIA:N}}` in structured output
   - Supports arbitrary MIME types (image/png, image/jpeg, etc.)

### C. Tag System

**Tag Definition** (persistent, global):
```go
type Tag struct {
    ID          string    // 12-char stable ID
    Name        string    // Display name (e.g., "SECURITY")
    Instruction string    // LLM-facing instruction (e.g., "Security-sensitive code...")
    CreatedAt   time.Time
}
```

**Features**:
- `/` key triggers tag autocomplete menu
- Suggestions based on usage frequency
- Smart ordering: most-used tags first
- Usage tracking per file type (.go, .ts, etc.) in `tag-usage.json`
- Tags appear in LEGEND section of output

### D. Exit Modes

**Exit Mode** (signals intent when closing session):
```go
type ExitMode struct {
    ID          string         // Stable ID or "ephemeral-0", "ephemeral-1"
    Name        string         // "Apply", "Reject", "Discuss", etc.
    Color       color.RGBA     // Auto-assigned from palette
    ColorCSS    string         // "#22c55e" (green, yellow, red, blue, purple, orange)
    Instruction string         // LLM-facing guidance
    Order       int            // Display/cycling order
    IsEphemeral bool           // true if from MCP (session-scoped)
}
```

**Features**:
- **Tab / Shift+Tab**: Cycle through modes, colored border indicates selection
- **User-defined modes**: Persistent, stored per-file-type in `~/.config/hl/exit-modes.{ext}.json`
- **Agent-defined modes**: Ephemeral, prepended before persistent modes (MCP only)
- Modes appear in SESSION block of output
- Color auto-assignment from palette if not specified

### E. Session Context

- **`g` key**: Opens editor for session-wide comment (not tied to specific lines)
- Rendered in SESSION block before annotations
- Supports all ContentNode types (text, tags, diagrams, images)

### F. Keyboard Shortcuts

| Shortcut           | Function                                   |
|--------------------|--------------------------------------------|
| Click line numbers | Select/deselect lines                      |
| Shift+Click        | Select range                               |
| `/` (in editor)    | Tag autocomplete menu                      |
| Tab                | Cycle exit mode forward                    |
| Shift+Tab          | Cycle exit mode backward                   |
| g                  | Open session context editor                |
| Cmd+S / Ctrl+S     | Flush pending saves                        |
| Ctrl+K             | Open poly-editor (tags/exit modes manager) |

---

## 2. OUTPUT FORMAT SPECIFICATION

**Structure** (when annotations exist):

```
LEGEND:
  [# TAG_NAME] Tag instruction text
  [# SECURITY] Security-sensitive code requiring review

SESSION:
  Context: [user's high-level session comment, may include {{MEDIA:N}}]
  Exit: Apply
    Apply all changes exactly as annotated

---

file.rs:45-52:
    44 | context line (non-empty line before selection)
    >  45 | first selected line
    >  46 | second selected line
    > [content node 1] text annotation
    > [content node 2] [# TAG] embedded tag
    > [content node 3] {{MEDIA:1}} diagram reference

---

file.rs:78-82:
    77 | context
    >  78 | line
    > [annotation content]
```

### Rendering Details

1. **LEGEND** (optional)
   - Alphabetically sorted by tag name
   - Collects unique tags from session + all annotations
   - Format: `[# NAME] instruction`

2. **SESSION** (optional, if context or exit mode selected)
   - `Context:` line with session-wide comment (can span multiple lines)
   - `Exit:` line with selected mode name + instruction (indented)

3. **Annotation Blocks**
   - File header: `file.rs:45-52` (or `:45` for single line)
   - Context line (1 line before start, if exists and non-empty)
   - Selected lines with `>` prefix
   - Annotation content with `>` prefix + arrow indent
   - Separated by `---\n\n` between blocks

4. **Media Resolution** (differs by output mode)
   - CLI: `{{MEDIA:N}}` placeholders resolved to `[Figure N]` with image data in responses
   - MCP: `{{MEDIA:N}}` placeholders + binary image attachments

---

## 3. FRONTEND ARCHITECTURE (Alpine + HTMX + TipTap)

### Tech Stack

- **Alpine.js**: Reactive state management for templates
- **HTMX**: Server-driven updates (annotation save/sync)
- **TipTap**: Rich text editor (with custom extensions)
- **TypeScript**: Type-safe client logic
- **Excalidraw**: Drawing library (lazy-loaded)

### Key Components

1. **Selection Manager** (`features/selection/`)
   - Tracks selected line ranges
   - Manages visual highlighting
   - Updates annotation store on save

2. **Editor** (`features/editor/`)
   - `hl-editor` custom element (TipTap-based)
   - Extensions:
     - `tag-command`: `/` command for tag insertion
     - `image-paste-handler`: Paste images as MediaChips
     - `excalidraw-chip`: Render excalidraw diagrams
     - `tag-chip`: Render tag chips `[# NAME]`
     - `media-chip`: Render pasted images
     - `backspace-handler`: Smart deletion of nodes
   - ContentNode extraction: Converts TipTap state → ContentNode[]
   - SaveQueue: Batches saves with debounce (prevents network spam)

3. **Poly-Editor** (`features/poly-editor/`)
   - Command palette for tag/exit-mode CRUD
   - Namespaced item management (tags, exit modes)
   - Engine + Reducer pattern (state machine)
   - Keyboard navigation with filtering

4. **Exit Mode Management** (`features/exitmode/`)
   - Cycle through modes with Tab/Shift+Tab
   - Visual feedback: selected mode colors window border
   - Persist selection to backend

5. **Tag System** (`tags/`)
   - Store: Persistent tag definitions
   - Menu: Autocomplete suggestions during editing
   - Suggest: Smart filtering based on context
   - Usage tracking: Per-file-type metrics

6. **Stores** (reactive state containers):
   - `modeStore`: Exit mode state + cycling
   - `selectionStore`: Line selection state
   - `sessionStore`: Session context editor state
   - `tagsStore`: Tag definitions + usage
   - `uiStore`: Modal states (Mermaid viewer, etc.)
   - `annotationsStore`: Current annotations map

---

## 4. BACKEND API ROUTES

Token-authenticated endpoints (Bearer token or query param):

### Content & Annotations

- `GET /` - Render main page with syntax highlighting
- `POST /annotations/upsert` - Create/update annotation
  - Body: `{ startLine: number, endLine: number, content: ContentNode[] }`
- `POST /annotations/session` - Set session-wide context
  - Body: `{ content: ContentNode[] }`

### Tags Management

- `GET /tags` - List all tags
- `POST /tags` - Create new tag
- `PUT /tags/{id}` - Update tag
- `DELETE /tags/{id}` - Delete tag

### Exit Modes

- `GET /exit-mode` - Get current state (modes + selected ID)
- `POST /exit-mode` - Set selected mode
  - Body: `{ modeId: string }` (empty string to clear)
- `POST /exit-modes` - Create new mode
- `DELETE /exit-modes/{id}` - Delete mode
- `POST /exit-modes/reorder` - Reorder modes

### Export & Copy

- `GET /copy/content` - Copy raw content to clipboard
- `GET /copy/annotations` - Copy formatted annotations
- `GET /copy/all` - Copy everything
- `POST /export/gist/content` - Export to GitHub Gist
- `POST /export/gist/annotations` - Export annotations to Gist
- `POST /export/gist/all` - Export everything to Gist
- `GET /export/obsidian` - Export to Obsidian vault

### Lifecycle

- `GET /events` - Server-sent events stream (SSE) for session close notification
- `POST /save` - Save ephemeral content to file (for stdin mode)

---

## 5. DATA PERSISTENCE

### Config Paths (platform-specific)

1. **Tags** (`~/.config/hl/tags.json`):
```json
[{
  "id": "abc123def456",
  "name": "SECURITY",
  "instruction": "Security-sensitive code requiring careful review",
  "createdAt": "2024-01-15T10:30:00Z"
}]
```

2. **Exit Modes** (`~/.config/hl/exit-modes.{ext}.json`):
   - Per-file-type (.go, .ts, .py, etc.)
```json
[{
  "id": "xyz789abc123",
  "name": "Apply",
  "color": "#22c55e",
  "instruction": "Apply changes exactly as annotated",
  "order": 0,
  "createdAt": "2024-01-15T10:30:00Z"
}]
```

3. **Tag Usage** (`~/.config/hl/tag-usage.json`):
   - Tracks frequency per file type for smart suggestions
```json
{
  ".go": {
    "abc123def456": { "count": 15, "lastUsed": "2024-12-19T12:00:00Z" },
    "xyz789abc123": { "count": 8, "lastUsed": "2024-12-15T14:00:00Z" }
  }
}
```

---

## 6. MCP INTEGRATION

Three tools exposed via MCP (stdio transport):

### review_file

```json
{
  "file_path": "path/to/file.go",
  "exit_modes": [
    {"name": "Apply", "instruction": "...", "color": "green"},
    {"name": "Revise", "instruction": "...", "color": "yellow"}
  ]
}
```
- Opens file for annotation
- Returns structured output + diagrams as attachments

### review_diff

```json
{
  "git_diff_args": ["--staged"],
  // OR
  "diff_content": "unified diff text",
  "label": "my-changes.diff",
  "exit_modes": [...]
}
```
- Opens diff in dual-column view
- Supports git-aware (preferred) or raw content

### review_content

```json
{
  "content": "# Plan\n\n1. Refactor...",
  "label": "implementation-plan.md",
  "exit_modes": [...]
}
```
- Opens ephemeral content (agent-generated)
- Can be saved to file

---

## 7. INPUT MODES

1. **File Mode**:
   ```bash
   hl main.go
   ```
   - Full file annotation

2. **Stdin Mode** (with label for syntax highlighting):
   ```bash
   cat file.go | hl -l main.go
   ```
   - Arbitrary content, labeled for highlighting
   - Optional: save to file with button

3. **Diff Mode** (MCP):
   - Git diff (automatic highlighting for +/- lines)
   - Raw unified diff
   - Dual-column view with diff metadata

---

## 8. SPECIAL FEATURES

### Obsidian Export
- Export annotations to Obsidian vault
- Requires `HL_OBSIDIAN_VAULT` env var
- Creates timestamped note with annotations

### Gist Export
- Export content/annotations to GitHub Gist
- Creates public or private Gist with GitHub token

### Clipboard Operations
- Copy raw content, annotations, or both
- Useful for web-based workflows

### Onboarding
- Interactive setup wizard
- Pre-configures common tags (SECURITY, TODO, etc.)
- Pre-configures common exit modes (Apply, Revise, etc.)
- One-time setup

---

## 9. ARCHITECTURE PATTERNS

### Thread Safety
- All stores protected by `sync.RWMutex`
- Safe concurrent access from HTTP handlers

### Error Handling
- Graceful degradation (missing config files don't crash app)
- Validation of content nodes before persistence
- Security: Token-based CSRF protection, constant-time token comparison

### Security
- Token middleware on all endpoints
- 10MB request size limit (for base64 images)
- `--no-ext-diff` on git commands to prevent external drivers
- `GIT_PAGER=""` to prevent hanging on git operations

### Separation of Concerns
1. **Content**: File/diff/ephemeral loading + syntax highlighting
2. **Store**: Annotation CRUD + session context
3. **Exit Mode**: Mode state machine + persistence
4. **Tags**: Tag CRUD + usage tracking
5. **Output**: Formatting to structured text (CLI or MCP)
6. **Server**: HTTP API + SSE lifecycle

---

## 10. FEATURE PRIORITY FOR REWRITE

### Must-Have (non-negotiable)
- Line selection and annotation
- Text + Tag + Diagram + Media content nodes
- Exit mode cycling and selection
- Session context (global comment)
- Persistent tag/exit-mode storage
- Keyboard shortcuts (/, Tab, g, Ctrl+K)
- Output format with LEGEND/SESSION/annotations
- MCP integration (review_file, review_diff, review_content)

### Nice-to-Have (establishes expected UX)
- Gist/Obsidian export
- Clipboard copy operations
- Tag usage tracking + smart suggestions
- Color-coded exit mode borders
- Poly-editor for management
- Onboarding wizard
