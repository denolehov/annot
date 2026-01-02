# annot — Product Features

> **annot: ephemeral space for human-AI thinking**

A native annotation tool that opens content in a window, lets humans add structured feedback with line-level precision, and outputs results for AI consumption when closed.

---

## Core Value Proposition

annot is a **structured dialogue medium** — a space where human and AI take turns refining content. It's designed to:

- **Pull humans into the loop** at decision points
- **Capture rich, located feedback** — not just yes/no, but *what* and *where*
- **Shape content through reaction** — propose replacements, sketch alternatives, add context
- **Exit cleanly** — ephemeral by design, zero persistence burden

---

## Review Modes

### File Review
Open any source file for annotation. Syntax highlighting adapts to language. Navigate by line numbers.

### Diff Review
Review git changes (`--staged`, `main...HEAD`) or raw unified diffs. Color-coded: additions green, deletions red. Annotations capture both old and new line numbers.

### Content Review
Review agent-generated content — plans, drafts, analysis. Markdown rendering with Mermaid diagrams and portal links that embed live code.

---

## Annotation System

### Line Selection
- Click line numbers to select individual lines
- Shift+Drag for ranges
- Multi-line annotations supported

### Rich Annotation Content
- Plain text with inline formatting
- **Tags**: Semantic mini-prompts like `[# SECURITY]` or `[# TODO]` — composable with prose and each other
- **Images**: Paste screenshots directly into annotations
- **Excalidraw diagrams**: Sketch ideas visually (can also convert Mermaid diagrams to Excalidraw)
- **Replace blocks**: Propose code replacements with before/after display
- **Bookmark references**: Link to previously captured moments

### Slash Commands in Annotation Editor
- `/` triggers command menu (Excalidraw, Replace block, etc.)
- `#` triggers tag autocomplete
- `@` triggers bookmark reference autocomplete

### Tag System
- Create custom tags with names and LLM instructions
- Tags persist across sessions
- Usage tracking for smart suggestions

---

## Exit Modes

Declare your intent before closing the window:

- **Tab/Shift+Tab** cycles through modes
- Selected mode colors the window footer
- Each mode has instruction text for AI interpretation

### Types
- **Persistent**: Saved modes like "Apply", "Reject", "Revise"
- **Ephemeral**: Agent-provided modes for context-specific workflows (via MCP)
- **Command-linked**: Modes that trigger Claude Code slash commands

---

## Portal Links (Live Code Embeds)

In content review, embed live code snippets:

```markdown
See the validation logic in [validate](src/lib.rs#L45-L60)
```

This fetches and displays the actual code inline, syntax-highlighted. Max 50 portals, 500 lines each. Sensitive paths blocked.

---

## Bookmarks

Capture moments for future reference:

- **Session bookmarks**: Snapshot the entire current review with an optional label
- **Selection bookmarks**: Bookmark specific line ranges with optional labels
- Reference bookmarks in annotations with `@` syntax
- Bookmarks are **detached** — if deleted, references keep their content

---

## Content Shaping

annot isn't a document editor — humans shape content through **reaction**, not direct authorship:

### Replace Blocks
- Propose code replacements via `/replace` command
- Shows original (tinted) alongside your replacement (green accent)
- Output includes diff-style format for AI to apply changes

### Save to File
- Export content as markdown via command palette
- Portal links expand to embedded code blocks
- Optionally overwrite source or create new file

### Export to Obsidian
- Send content to Obsidian vault
- Auto-names note from first H1 heading

---

## Diagrams

### Excalidraw
- Open diagram editor from annotation
- Saves as JSON + PNG
- Embedded inline in annotation content

### Mermaid
- Rendered from markdown code blocks
- Convert to Excalidraw for sketching

---

## Session Context

Press `Shift+C` to add a high-level comment that applies to the entire review (not a specific line). Appears in the SESSION block of output.

---

## Command Palette (`:`)

Press `:` (colon) to open. Seven namespaces:

### Tags
- Browse, create, edit, delete tags
- Examples: TODO, SECURITY, REFACTOR, BUG, PERF
- Press `e` to edit selected tag

### Exit Modes
- Browse, set, create, edit, delete, reorder modes
- Examples: Apply, Revise, Reject, Discuss
- Press `s` to set as active
- Press `r` to reorder (drag with arrow keys)

### Bookmarks
- Browse, edit labels, delete bookmarks
- Search by label or content
- Shows project context
- Sorted by creation date (newest first)

### Copy
- Copy content only
- Copy annotations only
- Copy both together

### Save
- Opens save modal for file export

### Obsidian
- Export to configured vaults
- Add/edit/delete vault references

### Theme
- System / Light / Dark toggle

---

## Keyboard-First Design

| Key | Action |
|-----|--------|
| Click line # | Select/deselect line |
| Shift+Drag | Select range |
| `/` | Slash command menu (Excalidraw, Replace) |
| `#` | Tag autocomplete |
| `@` | Bookmark reference autocomplete |
| Tab/Shift+Tab | Cycle exit modes |
| `Shift+C` | Session context editor |
| `:` | Command palette |
| `Alt+Tab` | Command palette → Exit modes |
| Ctrl+F | Search |
| `e` | Edit item (in command palette) |
| `r` | Reorder items (exit modes only) |
| `Cmd+D` | Delete item (in command palette) |

---

## Output Format

Structured for AI consumption:

```
LEGEND:
  [# TAG] Instruction text for this tag

SESSION:
  Reviewing plan.md with embedded files: src/lib.rs, src/main.rs
  Context: Please focus on the error handling approach
  Exit: Apply
    Proceed with this plan as written

---

src/lib.rs:45-52:
    44 | fn previous_line() {  // context
    45 | fn example() {
    46 |     // code
    > [# SECURITY] Check for injection vulnerabilities here
    > Consider validating the input before processing

SAVED: /path/to/file.md
```

---

## MCP Integration

Three tools exposed via Model Context Protocol:

1. **review_file** — Open file at path
2. **review_diff** — Review git or raw diffs
3. **review_content** — Review agent-generated content

All block until window closes, returning structured output with annotations, exit mode, and any images.

**Ephemeral exit modes**: Agents can provide context-specific exit modes each time they invoke annot — these appear alongside persistent modes but don't persist after the session.

---

## Configuration

Persisted in `~/.config/annot/`:
- Tags and exit modes
- Bookmarks with snapshots
- Usage statistics
- Theme preference
- Obsidian vault paths

Atomic writes with file locking for concurrent safety.

---

## Platform Support

Native Tauri app for:
- macOS (traffic light positioning, overlay title bar)
- Linux
- Windows
