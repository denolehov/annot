# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**annot** is a human-in-the-loop annotation tool for AI workflows. Opens files, diffs, or content in a native window for structured annotation, outputs results when closed.

**Before developing features**, read `docs/manifesto.md` — it defines what annot is and isn't.

## Build Commands

```bash
# Development
pnpm demo              # Opens sample.md
pnpm demo:diff         # Opens sample.diff
pnpm demo:portals      # Opens portals.md (portal links demo)
pnpm tauri dev -- -- <file>   # Open specific file

# Production
pnpm tauri build              # Release build
pnpm tauri build --debug      # Debug build with embedded frontend

# Testing
pnpm test                     # All frontend tests
pnpm vitest run <file>        # Single frontend test file
cargo test                    # All Rust tests (from src-tauri/)
cargo test <name>             # Single Rust test
cargo insta review            # Review snapshot changes

# Type checking
pnpm check
```

**Dev vs Build gotcha**: `cargo build` produces a binary that connects to localhost:1420. For standalone testing, use `pnpm tauri build --debug`.

## Architecture

**Backend** (`src-tauri/src/`):
- `lib.rs` — Entry points (CLI vs MCP mode), IPC command registration
- `review.rs` — Session state, content loading
- `commands.rs` — All Tauri IPC handlers
- `output/` — Structured output rendering for LLM consumption
- `mcp/` — Model Context Protocol server
- `config.rs` — Persistent user settings (tags, exit modes, bookmarks)
- `terraform.rs` — Natural prose directives feature

**Frontend** (`src/lib/`):
- `composables/` — Svelte 5 runes-based state (useAnnotations, useTerraformRegions, etc.)
- `components/` — UI components (LineRow, CodeViewer, etc.)
- `CommandPalette/` — `:` command palette with namespaces
- `tiptap.ts` — Rich text editor configuration
- `types.ts` — Shared TypeScript types

**Three review modes**: `review_file`, `review_diff`, `review_content` — all block until window closes.

## Reference Materials

- `docs/features.md` — **Canonical product features** (update when adding features)
- `docs/manifesto.md` — Philosophy and directional model
- `README.md` — MCP integration, keyboard shortcuts
- `src/lib/HelpOverlay.svelte` — Update when adding keyboard shortcuts

## Testing Patterns

**Rust output tests** use insta snapshots in `src-tauri/src/output/snapshots/`. When format changes:
1. `cargo test` — fails with diff
2. `cargo insta review` — accept or reject
3. Commit `.snap` files

**Frontend tests** mock Tauri IPC with `vi.mock("@tauri-apps/api/core")`.

## Code Style

- **Declarative over imperative**: `map`/`collect`/`join` over manual loops
- **Composables pattern**: Svelte 5 runes in `src/lib/composables/`

## Agent Output Preferences

- **Use `review_content` for reports/summaries**: Present plans, analysis, or structured output via the MCP tool instead of inline text.

## UI Patterns

### Line Actions (right-side icons)
Add buttons inside the `{#if trailing || showBookmarkIcon || terraformRegionStart}` block in `LineRow.svelte`. Use `.line-action` class.

### Left Border Indicators
Use `::before` pseudo-elements with `position: absolute; left: 0; width: 3px;`. For overlapping indicators, use `repeating-linear-gradient`.

### Display Index vs Source Line Numbers
- `displayIndex`: 1-indexed position in rendered array (UI selection)
- Source lines: `line.origin.line` (file) or `line.origin.new_line`/`old_line` (diff)

Use `getLineNumber(line)` and `getFilePath(line)` from `line-utils.ts`.

### Terraform Regions
Persist to backend with source line numbers. Use `useTerraformRegions` composable for state.
