# annot

An annotation tool for human-in-the-loop AI workflows.

## Why

When working with AI coding agents, you sometimes need to *point at things* — mark a section for refactoring, flag a suspicious pattern, or leave a note that says "this, but more X."

annot gives you a focused interface to do exactly that, then gets out of the way.

## What it does

Review and annotate code, diffs, or agent-generated content in a dedicated window — with syntax highlighting, Mermaid diagrams, and a keyboard-first interface. Mark up specific lines, select an exit mode, close the window, and your structured feedback returns to the caller.

### Agent integration

annot exposes MCP tools that AI agents (and sub-agents) can invoke to request human review mid-workflow:

```
You: Review my auth implementation before we continue

Claude: I'll open auth.rs for your review.

        ┌─────────────────────────────────────────────────────┐
        │  review_file                                        │
        │  file_path: "src/auth.rs"                           │
        │  exit_modes: [                                      │
        │    {name: "Apply", instruction: "Apply as-is"},     │
        │    {name: "Revise", instruction: "Needs changes"}   │
        │  ]                                                  │
        └─────────────────────────────────────────────────────┘

        A window opens. You annotate lines 45-52:

            "Token validation is fragile — what happens if it
             expires mid-request? Add retry logic."

        You tag lines 78-82:

            "[# SECURITY] verify this doesn't leak timing info"

        Tab to select "Apply", then close the window.

Claude: Thanks for the review. You selected "Apply" with two annotations:

        Lines 45-52: Token validation concern
        > "Token validation is fragile — what happens if it
        >  expires mid-request? Add retry logic."

        Lines 78-82:
        > "[# SECURITY] verify this doesn't leak timing info"

        Implementing the retry logic first...
```

## Installation

```bash
# Build from source
git clone https://github.com/denolehov/annot.git && cd annot
pnpm install
pnpm tauri build
```

The binary will be at `target/release/annot`.

## Quick start

```bash
# Open a file for annotation
annot main.rs

# Window opens → annotate lines → close window
# Annotations appear in stdout
```

### Annotating

1. Click line numbers to select a range
2. Type your annotation in the editor that appears
3. Use `/tag` to insert composable mini-prompts into your prose
4. Select an exit mode (Tab/Shift+Tab to cycle)
5. Close the window — done

## Features

### Tags

Tags are composable mini-prompts that you weave directly into your prose annotations. Type `/` in the editor to create or insert a tag:

```
/Security    → inserts [# SECURITY]
/TODO        → inserts [# TODO]
/elaborate   → inserts [# ELABORATE]
```

The power is in mixing them with natural language:

```
[# DISSOLVE] this section if [# EXPERT-PANEL] suggests better alternatives

Please [# ELABORATE] on the error handling here

[# SECURITY] verify constant-time comparison, provide [# VISUAL] and [# PROS-CONS]
```

Each tag carries semantic meaning that LLMs can interpret. Tags appear in a LEGEND block in the output:

```
LEGEND:
  [# SECURITY] Security-sensitive code requiring careful review
  [# DISSOLVE] Remove or significantly reduce this section

main.rs:45-52:
    [# DISSOLVE] if [# EXPERT-PANEL] suggests better alternatives
```

### Exit modes

Exit modes let you signal *intent* when closing a review session. Instead of just closing the window, you can indicate what should happen next.

**User-defined modes** persist across sessions. Manage them with `Ctrl+K`.

**Agent-defined modes** are ephemeral and can be passed via MCP for context-specific workflows:

```json
{
  "exit_modes": [
    {"name": "Apply", "instruction": "Apply all changes exactly as annotated"},
    {"name": "Reject", "instruction": "Reject and explain reasoning"},
    {"name": "Discuss", "instruction": "Discuss trade-offs before implementing"}
  ]
}
```

The selected exit mode appears in the output:

```
SESSION:
  Exit: Apply
    Apply all changes exactly as annotated
```

### Session context

Press `g` to add comments that apply to the entire review (not tied to specific lines). These appear in the SESSION block alongside your exit mode selection.

### Poly-editor (`Ctrl+K`)

A unified interface for managing tags and exit modes. Create, edit, reorder, and delete items without leaving the window.

## MCP Integration

```bash
annot mcp  # Start MCP server (stdio transport)
```

**Tools exposed:**

### `review_file`

Open a file for annotation.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Absolute or relative path to the file |
| `exit_modes` | array | no | Ephemeral exit modes for this session |

### `review_diff`

Review git diffs or raw patches.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `git_diff_args` | array | no* | Git diff arguments (e.g., `["--staged"]`) |
| `diff_content` | string | no* | Raw unified diff content |
| `label` | string | no | Display name (default: "diff") |
| `exit_modes` | array | no | Ephemeral exit modes for this session |

*Either `git_diff_args` or `diff_content` must be provided.

### `review_content`

Review agent-generated content (plans, drafts, etc.).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `content` | string | yes | Markdown-formatted text content |
| `label` | string | yes | Display name with .md extension |
| `exit_modes` | array | no | Ephemeral exit modes for this session |

**Exit mode format:**

```json
{
  "name": "Apply",
  "instruction": "Apply all changes exactly as annotated",
  "color": "green"  // optional: green, yellow, red, blue, purple, orange
}
```

## How it works

1. annot opens a window with your content
2. You annotate specific line ranges, weaving tags into prose
3. Select an exit mode to signal intent
4. Closing the window outputs structured annotations
5. In MCP mode, annotations return to the calling agent

No data leaves your machine. No accounts. No cloud.

## Keyboard shortcuts

| Shortcut           | Function                                   |
|--------------------|--------------------------------------------|
| Click line numbers | Select/deselect lines                      |
| Shift+Click        | Select range                               |
| `/` (in editor)    | Tag autocomplete menu                      |
| Tab                | Cycle exit mode forward                    |
| Shift+Tab          | Cycle exit mode backward                   |
| g                  | Open session context editor                |
| Ctrl+K             | Open poly-editor (tags/exit modes manager) |

## Tech stack

- **Backend**: Rust + Tauri v2
- **Frontend**: Svelte 5 + TypeScript
- **Rich editor**: TipTap
