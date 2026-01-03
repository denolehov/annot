# Spec: Portals — Live Code Embeds (Final)

## Goal

Enable agents to reference real file contents in markdown documents (both `review_content` and `review_file`), rendered inline with visual distinction, annotatable with output referencing the source file.

---

## Syntax

Standard markdown link with **required** line anchor:

```markdown
[optional label](path/to/file.go#L42-L58)
```

**Variations:**
- With label: `[Auth flow](src/auth.go#L42-L58)`
- No label: `[](src/auth.go#L42-L58)` — uses filename as label
- Single line: `[](src/auth.go#L42)` or `[](src/auth.go#L42-L42)`

**Not a portal:** `[](src/auth.go)` — no line anchor = regular link, ignored.

**Detection**: `NodeValue::Link` where URL has `#L{n}` or `#L{n}-L{m}` anchor.

**Path resolution**: Relative to markdown file's directory.

---

## Validation

### Sensitive Path Blocklist

Reject with error if path matches:
- `**/id_rsa`, `**/id_ed25519`, `**/.ssh/*`
- `**/.env*`, `**/secrets.*`, `**/credentials.*`
- `**/.aws/*`, `**/.gcp/*`

### Resource Limits

| Limit | Value | Behavior on exceed |
|-------|-------|-------------------|
| Max portals per document | 50 | Skip additional, warn |
| Max lines per portal | 500 | Clamp range, warn |
| Max line length | 2000 chars | Truncate with `...` |

---

## Line Anchor Rules

| Pattern | Behavior |
|---------|----------|
| `#L42-L58` | Lines 42 through 58 |
| `#L42` | Single line 42 |
| `#L42-L42` | Single line 42 (normalized) |
| `#l42-l58` | Case-insensitive |
| No anchor | **Not a portal** |
| `#L0` | **Error** — 1-indexed |
| `#L58-L42` | Normalize to `#L42-L58` |
| Out of bounds | Clamp to file length, warn |

---

## Content Validation

| Condition | Behavior |
|-----------|----------|
| File not found | **Error** (MCP) / Skip + warn (CLI) |
| Binary file | **Error** |
| Invalid UTF-8 | **Error** |
| Markdown file (`.md`) | **Reject** — no recursion |

---

## Visual Rendering

```
 13 │ This is some markdown document content.
 14 │ With an inline [portal link](src/main.rs#L78-79)
    ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┤
  > │ portal link — src/main.rs#L78-79                │
 78 │ fn main() {                                     │
 79 │     // some rust code                           │
    ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┤
 15 │ And the main content continues here.
```

### Line Numbers

| Line Type | Gutter |
|-----------|--------|
| Document lines | Sequential (13, 14, 15...) |
| Portal header | `>` marker (non-selectable) |
| Portal content | Source file numbers (78, 79...) |

### Long Paths

- Max header width: 60 chars
- Truncate middle: `src/.../to/file.rs#L1-100`
- Full path in tooltip on hover

### Portal in Nested Structures

| Context | Behavior |
|---------|----------|
| In code block | Literal (not a portal) |
| In blockquote/list/table | Supported |

---

## Data Model

### markdown.rs

```rust
#[derive(Clone, Debug, Serialize)]
pub struct PortalInfo {
    pub source_line: u32,
    pub label: Option<String>,
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
}

pub struct MarkdownMetadata {
    // ... existing ...
    pub portals: Vec<PortalInfo>,
}
```

### state.rs

```rust
#[derive(Clone, Debug, Serialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: u32,
    pub portal_id: String,
}

pub struct Line {
    // ... existing ...
    pub source: Option<SourceLocation>,
}

pub enum LineType {
    // ... existing ...
    PortalHeader,
}
```

### review.rs

Source files **fully loaded** into `Review.files`.

---

## Annotation Storage

Annotations on portal content stored on **source file**.

Same file in multiple portals → annotations shared.

---

## Selection Boundaries

Reuse diff hunk boundary logic:
- Cannot cross portal ↔ document boundary
- Portal header non-selectable

---

## Export & Copy Behavior

### LLM Output (Primary)

Portal content **baked inline** with annotations:

```
src/main.rs:78-79:
    78 | fn main() {
    79 |     // some rust code
     > [# QUESTION] Why is this here?
```

Annotations reference source file, not markdown position.

### Clipboard Copy

**WYSIWYG** — copy what's visible:
- Select portal content → copies source code
- Select markdown lines → copies markdown text

### Obsidian Export

Portal content **baked** with callout syntax:

```markdown
Here's the auth flow:

> [!code]- src/auth.go:42-58
> ```go
> func Login(ctx context.Context) error {
>     // ...
> }
> ```

> [!warning] Line 45
> [# FIX] This swallows the error
```

---

## Output Format

### Ordering

By file, then by line:

```
plan.md:14-15:
    14 | ## Implementation Steps
     > [# QUESTION] Is this the right order?

src/main.rs:78-79:
    78 | fn main() {
     > [# QUESTION] Why is this here?
```

### Session Block

When portals present:
```
SESSION:
  Reviewing plan.md with embedded files: src/main.rs
  Apply (Apply the suggested changes)
```

---

## Error Handling

| Error | MCP Mode | CLI Mode |
|-------|----------|----------|
| File not found | Fail | Skip, warn |
| Out of range | Fail | Clamp, warn |
| Sensitive/binary/invalid | Fail | Skip, warn |
| No base path | Fail | N/A |

---

## Styling

```css
--portal-border: 1px dashed var(--border-muted);
--portal-header-color: var(--text-muted);
--portal-header-bg: transparent;
```

---

## Processing Flow

```
from_markdown(content, base_path)
    │
    ├─ parse_markdown() → extract NodeValue::Link with #L anchor
    │
    ├─ For each portal (max 50):
    │   ├─ Validate, resolve path
    │   ├─ Read, highlight (max 500 lines)
    │   ├─ Register in Review.files
    │   └─ Build PortalHeader + content with SourceLocation
    │
    └─ Interleave at correct positions
```

---

## Decisions

| Decision | Choice |
|----------|--------|
| Syntax | `[label](path#L42-L58)` |
| Line anchor | **Required** |
| Case | `#L` and `#l` accepted |
| Markdown-to-markdown | Reject |
| Limits | 50 portals, 500 lines each |
| Annotations | On source file |
| Selection | Reuse diff hunk logic |
| LLM export | Baked inline |
| Obsidian export | Baked with callouts |
| Clipboard | WYSIWYG |

---

## Scope

**In:**
- Portal detection (`#L` anchor required)
- `PortalInfo`, `SourceLocation`, `PortalHeader`
- Validation, limits
- Source file loading
- Annotation storage
- Selection boundaries
- Export behavior (LLM, Obsidian, clipboard)
- CSS tokens

**Out (future):**
- "Open in new window"
- Remote URLs
- Collapse/expand
- Live refresh
