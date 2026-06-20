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
pnpm test:rust                # All Rust tests
pnpm test:rust -- <name>      # Single Rust test
cargo insta review            # Review snapshot changes (from src-tauri/)

# Type checking
pnpm check
```

**Dev vs Build gotcha**: `cargo build` produces a binary that connects to localhost:1420. For standalone testing, use `pnpm tauri build --debug`.

## Ubuntu / Linux

Install system dependencies before running any build commands:

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad \
  libgstreamer-plugins-bad1.0-dev
```

Then install Rust (if not already present):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

MCP registration on Linux (`mcpServers` in settings.json is not supported — use CLI):

```bash
claude mcp add --scope user annot annot mcp
```

## NixOS

A `flake.nix` is provided. Enter the dev shell before running any build commands:

```bash
nix develop          # sets up cargo, pnpm, webkitgtk, GStreamer, pkg-config, etc.
```

The shell hook also exports `GDK_BACKEND=x11` and `WEBKIT_DISABLE_DMABUF_RENDERER=1`
so the dev server renders correctly under Wayland.

To install the binary into your Nix profile (wraps it with the required env vars):

```bash
nix profile install path:.   # run outside nix develop
```

MCP registration — same as Ubuntu above.

## Windows

Prerequisites:
- **Rust** — install via [rustup](https://rustup.rs/)
- **Node.js 24** — install via [nodejs.org](https://nodejs.org/)
- **pnpm** — `npm install -g pnpm`
- **WebView2 Runtime** — preinstalled on Windows 11; Windows 10 users should download the
  [Evergreen Bootstrapper](https://developer.microsoft.com/microsoft-edge/webview2/)

Build:

```powershell
pnpm install
pnpm tauri build                      # release binary: src-tauri\target\release\annot.exe
pnpm tauri build --debug --no-bundle  # debug build (matches what CI does)
```

MCP registration (CLI, same as Ubuntu):

```bash
claude mcp add --scope user annot annot mcp
```

Make sure `annot.exe` is on your `PATH`. See **Window creation & the WebView2 deadlock**
below for Windows-specific pitfalls — in particular, kill any stray `annot.exe`
process before rebuilding or you'll get `Access is denied (os error 5)`.

## Architecture

**Backend** (`src-tauri/src/`):
- `lib.rs` — Entry points (CLI vs MCP mode), IPC command registration
- `review.rs` — Session state, content loading
- `commands.rs` — All Tauri IPC handlers
- `output/` — Structured output rendering for LLM consumption
- `mcp/` — Model Context Protocol server
- `config.rs` — Persistent user settings (tags, exit modes)

**Frontend** (`src/lib/`):
- `composables/` — Svelte 5 runes-based state (useAnnotations, etc.)
- `components/` — UI components (LineRow, CodeViewer, etc.)
- `CommandPalette/` — `:` command palette with namespaces
- `tiptap.ts` — Rich text editor configuration
- `types.ts` — Shared TypeScript types

**Three review modes**: `review_file`, `review_diff`, `review_content` — all block until window closes.

### Window creation & the WebView2 deadlock (READ BEFORE ADDING ANY WINDOW)

`WebviewWindowBuilder::build()` re-enters the OS event loop on Windows (WebView2).
Call it from a thread that is **blocking the main/UI thread** and it deadlocks
forever — no error, no window. This has bitten us twice (diagram windows, then the
MCP server). Use the right pattern for where the code runs:

- **From a `#[tauri::command]`** — make the command `async`. Sync commands run on
  the main thread; `async` ones run on the runtime, freeing the event loop to
  service `build()`. Example: `mermaid_window.rs::open_mermaid_window`,
  `excalidraw_window.rs`.
- **From any other background thread** (e.g. the MCP server's `spawn_blocking`
  pool) — never call `build()` directly. Marshal it onto the main thread with
  `app_handle.run_on_main_thread(...)` and send the built window's label back over
  an `mpsc` channel. Example: `mcp/mod.rs::run_session_with_state`.

Never call `build()` from a synchronous command, or while holding a lock the main
thread also needs.

**Operational corollary**: a running MCP server (`annot-dev`) holds a lock on
`target/debug/annot.exe`. Builds then fail with `Access is denied (os error 5)`.
Kill the stray `annot.exe` before rebuilding.

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
Add buttons inside the `{#if trailing}` block in `LineRow.svelte`. Use `.line-action` class.

### Left Border Indicators
Use `::before` pseudo-elements with `position: absolute; left: 0; width: 3px;`. For overlapping indicators, use `repeating-linear-gradient`.

### Display Index vs Source Line Numbers
- `displayIndex`: 1-indexed position in rendered array (UI selection)
- Source lines: `line.origin.line` (file) or `line.origin.new_line`/`old_line` (diff)

Use `getLineNumber(line)` and `getFilePath(line)` from `line-utils.ts`.
