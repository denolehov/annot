# Spec: Markdown Support for annot

## Goal
Add markdown rendering with section tracking (breadcrumb navigation like diff hunks) for `.md` files, including code block highlighting and table auto-formatting.

## Design

### Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   markdown.rs   │     │   state.rs      │     │  +page.svelte   │
│  parse_md()     │────▶│ MarkdownMeta    │────▶│ ContentTracker  │
│  comrak AST     │     │ sections[],     │     │ (generic)       │
│                 │     │ code_blocks[]   │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                                                │
        ▼                                                ▼
┌─────────────────┐                              ┌─────────────────┐
│  Render HTML    │                              │  Smart Header   │
│  comrak → HTML  │                              │ # Heading · Sub │
│  syntect codes  │                              │ (breadcrumb)    │
└─────────────────┘                              └─────────────────┘
```

### Generalized Content Tracking

Extract a reusable pattern from diff hunk tracking:

```typescript
// lib/content-tracker.ts

interface Boundary<T> {
  line: number;      // Display/source line number
  data: T;           // Mode-specific payload
}

class ContentTracker<T> {
  private boundaries: Boundary<T>[] = [];

  constructor(items: Boundary<T>[]) {
    this.boundaries = items.sort((a, b) => a.line - b.line);
  }

  // Binary search: find largest boundary.line <= targetLine
  findAt(targetLine: number): Boundary<T> | null { ... }
}

// Usage examples:

// Diff mode
type HunkPayload = { fileIndex: number; hunkIndex: number };
const hunkTracker = new ContentTracker<HunkPayload>(hunkBoundaries);

// Markdown mode
type SectionPayload = { sectionIndex: number };
const sectionTracker = new ContentTracker<SectionPayload>(sectionBoundaries);

// Future: Code mode (sticky function/class names)
type SymbolPayload = { symbolIndex: number; kind: 'function' | 'class' };
const symbolTracker = new ContentTracker<SymbolPayload>(symbolBoundaries);
```

### Data Model

```rust
// markdown.rs

pub struct MarkdownMetadata {
    pub sections: Vec<SectionInfo>,
    pub code_blocks: Vec<CodeBlockInfo>,
    pub tables: Vec<TableInfo>,
}

pub struct SectionInfo {
    pub source_line: u32,           // 1-indexed line in source
    pub level: u8,                  // 1-6 for h1-h6
    pub title: String,              // "Getting Started"
    pub parent_index: Option<usize>, // For breadcrumb: points to parent section
}

pub struct CodeBlockInfo {
    pub start_line: u32,            // Source line (1-indexed)
    pub end_line: u32,
    pub language: Option<String>,   // "rust", "python", etc.
}

pub struct TableInfo {
    pub start_line: u32,
    pub end_line: u32,
    pub formatted_lines: Vec<String>, // Auto-aligned source lines
}
```

### Smart Header Display

```
┌──────────────────────────────────────────────────────────────┐
│ # Getting Started · ## Installation · ### macOS             │
└──────────────────────────────────────────────────────────────┘
```

- Uses `·` separator (consistent with diff mode)
- Shows heading hierarchy as breadcrumb
- Updates on scroll via ContentTracker

### Rendering Pipeline

```
1. Detect .md file (by extension)
2. Parse with comrak → AST
3. Walk AST:
   - Extract SectionInfo[] from headings (with parent chain)
   - Extract CodeBlockInfo[] (start/end lines, language)
   - Extract TableInfo[] (start/end lines)
4. For tables: reformat source with aligned columns
5. Render to HTML:
   - Inline markdown (bold, italics, links) → comrak HTML
   - Code blocks → syntect highlighting (consistent with diff mode)
6. Return Line[] + MarkdownMetadata to frontend
```

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Library | **comrak** | Full GFM tables, stable, widely used |
| Line mapping | **Source lines** | 1:1 with source, preserves annotation positions (like hl) |
| Code highlighting | **syntect** | Consistent look with diff mode |
| Table display | **Reformat source** | Aligned columns in source view (like hl) |
| Inline rendering | **comrak HTML** | Bold, italics, links rendered inline |
| Tracking pattern | **Generalized ContentTracker** | Reusable for diff, markdown, code symbols |
| Header separator | **`·`** | Consistent with diff mode |

## Implementation Order

1. Add `comrak` to Cargo.toml
2. Create `src-tauri/src/markdown.rs`:
   - `is_markdown()` detection
   - `parse_markdown()` → MarkdownMetadata
   - Table column alignment
3. Integrate syntect for code blocks in markdown
4. Update `state.rs` to handle markdown files
5. Add `MarkdownMetadata` to `ContentResponse`
6. Create `src/lib/content-tracker.ts` (extract from diff code)
7. Refactor `+page.svelte` to use ContentTracker for both modes
8. Add markdown breadcrumb header

## Scope

**In:**
- comrak parsing + HTML rendering
- Section extraction with parent chain
- Code block detection + syntect highlighting
- Table auto-alignment (source reformatting)
- Inline markdown rendering (bold, italics, links)
- Generalized ContentTracker utility
- Breadcrumb header with `·` separator

**Out:**
- LaTeX/math rendering
- Mermaid/diagram rendering
- WYSIWYG markdown editing
- Collapsible sections
- Table of contents sidebar
