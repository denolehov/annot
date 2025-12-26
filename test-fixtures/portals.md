# Portals

Portals embed live code snippets from your codebase directly into markdown. They're perfect for documentation that stays in sync with your code.

## How It Works

When annot encounters a link with a line anchor like `[label](path#L1-L20)`, it expands it into a live code embed. The original line remains visible, with the code appearing below.

## Architecture Overview

The portal system has two main components: the [parser](../src-tauri/src/markdown.rs#L204-L237) that detects portal links in markdown, and the [loader](../src-tauri/src/portal.rs#L160-L190) that reads and validates the referenced files.

Security is handled by [validate_portal](../src-tauri/src/portal.rs#L112-L155) which blocks sensitive paths, binary files, and recursive markdown references.

## State Management

Each portal generates three line types - header, content, and footer. The [LoadedPortal](../src-tauri/src/portal.rs#L91-L105) struct captures this along with metadata about where to insert it.

The interleaving happens in [ContentModel::from_markdown](../src-tauri/src/state.rs#L704-L725) which processes portals in reverse order to preserve line indices.

## Edge Cases

Portals without labels like [](../src-tauri/src/lib.rs#L1-L8) use the filename as the display label.

Single-line portals work too: see [MAX_PORTALS](../src-tauri/src/portal.rs#L18-L21) for the document limit.

## Not Portals

Regular links like [GitHub](https://github.com) render normally. Code blocks are also unchanged:

```rust
// This is a fenced code block, not a portal
let x = 42;
```
