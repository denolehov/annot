# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**annot** is a Rust/Tauri/Svelte rewrite of [hl](https://github.com/denolehov/hl) — an ephemeral, human-in-the-loop annotation tool for AI workflows. It opens files, diffs, or piped content in a native window, allows users to annotate specific line ranges with structured feedback, and outputs annotations to stdout (or MCP responses) when the window closes.

### Core Concept

annot interrupts LLM workflows to get human feedback, then resumes. It's designed to be:
- **Ephemeral**: Opens, collects feedback, exits
- **Keyboard-first**: /, g, Tab, Ctrl+K shortcuts
- **LLM-aware**: Output format designed for Claude consumption

## Build Commands

```bash
# Development (runs both Vite dev server and Tauri)
pnpm demo         # Opens lib.rs as demo file
pnpm tauri dev -- -- <file>  # Open specific file

# Build for production
pnpm tauri build           # Release build
pnpm tauri build --debug   # Debug build with embedded frontend

# Testing
pnpm test         # Frontend tests (Vitest)
pnpm test:watch   # Watch mode
cargo test        # Rust tests (from src-tauri/)

# Type checking
pnpm check        # TypeScript + Svelte
```

### Important: Dev vs Build modes

- `pnpm tauri dev` / `cargo build` → Uses Vite dev server at localhost:1420
- `pnpm tauri build` → Embeds frontend assets into binary

**Gotcha**: Running `./target/debug/annot` directly after `cargo build` shows white screen because it tries to load from localhost:1420. Must use `pnpm tauri build --debug` for standalone binary.

## Architecture

### Three Review Modes

1. **review_file** — Opens a file at a path for annotation
2. **review_diff** — Opens a unified diff (git or raw) with dual-column view
3. **review_content** — Opens ephemeral agent-generated content (plans, drafts)

All modes block until the window closes, then return structured output.

### Data Model

```
Annotation
├── start_line: u32 (1-indexed)
├── end_line: u32
└── content: Vec<ContentNode>
    ├── Text { text: String }
    ├── Tag { id, name, instruction }  // Composable mini-prompts like [# SECURITY]
    ├── Excalidraw { elements, png }   // Embedded diagrams
    └── Media { data_url }             // Pasted images

ExitMode
├── id: String
├── name: String           // "Apply", "Reject", etc.
├── color: String          // CSS color
├── instruction: String    // LLM-facing guidance
└── is_ephemeral: bool     // true if from MCP, false if persistent

Tag
├── id: String             // 12-char stable ID
├── name: String           // User-created name
└── instruction: String    // LLM prompt text
```

### Output Format

Structured text for LLM consumption:

```
LEGEND:
  [# TAG_NAME] Tag instruction text

SESSION:
  Context: [user's high-level comment]
  Exit: Apply
    Apply instruction text

---

file.rs:45-52:
    45 | fn example() {
    46 |     // code
    > [# SECURITY] Review this for injection vulnerabilities
    > Additional comment text
```

### Persistence

User config stored in platform-specific config directory:
- `tags.json` — Global tag definitions
- `exit-modes.{ext}.json` — Exit modes per file type (.rs, .go, .py)
- `tag-usage.json` — Usage stats for smart suggestions

### Frontend ↔ Backend Communication

Tauri IPC commands replace the HTTP API from the Go version:
- `get_content` — Load file/diff/ephemeral content
- `upsert_annotation` — Create/update annotation
- `delete_annotation` — Remove annotation
- `set_exit_mode` — Select exit mode
- `get_tags` / `upsert_tag` — Tag CRUD
- `finish_session` — Close window, return output

### Key UX Patterns to Preserve

- **Line selection**: Click line numbers to select range
- **Tag menu**: Type `/` in editor to trigger tag autocomplete
- **Exit mode cycling**: Tab/Shift+Tab cycles through modes
- **Poly-editor**: Ctrl+K opens tag/exit-mode manager
- **Session context**: `g` opens file-level comment editor
- **Visual feedback**: Selected exit mode colors the window border

## Reference Materials

**Original Go implementation**: `/Users/denolehov/_p/golang/hl`
- `internal/store/store.go` — Annotation storage model
- `internal/exitmode/exitmode.go` — Exit mode state
- `internal/output/output.go` — Output formatting
- `mcp.go` — MCP protocol integration
- `src/` — TypeScript frontend (Alpine + HTMX + TipTap)

**Tauri documentation**: `/Users/denolehov/_p/docs/tauri-docs`

**Important**: Use dedicated sub-agents (Task tool with Explore or general-purpose type) when exploring the original hl implementation or Tauri docs. This preserves context tokens in the main conversation.

## Tech Stack

- **Backend**: Rust + Tauri v2 + clap (CLI parsing)
- **Frontend**: Svelte 5 + SvelteKit + TypeScript
- **Testing**: Vitest + @testing-library/svelte (frontend), cargo test (Rust)
- **Syntax highlighting**: TBD (tree-sitter or syntect)
- **Diff parsing**: TBD (similar-diff or custom)
- **Rich editor**: TBD (TipTap port or Svelte alternative)

## Tauri Configuration Notes

### Window (tauri.conf.json)
- `titleBarStyle: "Overlay"` — Traffic lights overlay content
- `hiddenTitle: true` — No title text
- `trafficLightPosition: { x: 12, y: 22 }` — Vertically centered in header

### Permissions (capabilities/default.json)
- `core:window:allow-start-dragging` — Required for `data-tauri-drag-region` to work

## Testing Patterns
- **Rust**: Behavior-focused tests on public API (ContentResponse format)
- **Frontend**: Mock IPC with `vi.mock("@tauri-apps/api/core")`, test rendered output
