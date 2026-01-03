# Outline View

**Refactoring Tools for Prose** — A Workflowy-inspired overlay for editing markdown document structure.

## Problem

When reviewing AI-generated documents (PRDs, specs, plans), users often want structural feedback:

- "Move section 4 before section 3"
- "Delete the Marketing section entirely"
- "You're missing a Migration Plan"
- "Promote this H2 to H1"

Line annotations are imprecise for this. Writing "move this section earlier" requires interpretation.

## Solution

An overlay (`o` key) showing the heading hierarchy as an editable tree. Users manipulate structure directly, and annot outputs a unified diff.

```
- [H1] Overview
  - [H2] Context
    - > add glossary table
  - [H2] Problem Statement
- [H1] Requirements
```

## Interaction

| Action | Trigger |
|--------|---------|
| Open overlay | `o` |
| Close overlay | `Esc` or click outside |
| Edit heading | Click or Enter |
| Delete node | Backspace/Delete |
| Add child | Tab |
| Reorder | Drag-drop or Alt+Up/Down |
| Undo | Ctrl+Z |

## Node Types

### Structure Nodes

Headings that define document skeleton:

```
- [H1] Overview
  - [H2] Context
  - [H2] Problem Statement
```

### Directive Nodes

Instructions for the AI, prefixed with `>`:

```
- [H1] Overview
  - [H2] Context
    - > reference @docs/legal.md for edge cases
    - > add glossary table for ubiquitous language
```

Directives tell the AI *how* to fill the skeleton. Structure nodes define *what* the skeleton is.

## Edge Cases

### Orphan Text

Text before the first heading or between headings attaches to the preceding header. Moving a heading moves its body text.

### Heading Gaps

H1 → H3 jumps display with appropriate indentation. No phantom H2 nodes.

```
- [H1] Overview
    - [H3] Deep Section
```

### Ambiguous Headers

Duplicate heading names (e.g., two `[H3] Properties`) include parent lineage in diff context:

```diff
  [H1] User
    [H2] Settings
-     [H3] Properties
  [H1] Admin
    [H2] Settings
+     [H3] Properties
```

### Cascade Delete

Deleting a heading removes:
1. The heading
2. All child headings
3. All body text under it
4. All annotations anchored to deleted lines

Prevents ghost annotations on deleted content.

## Output Format

Separate `STRUCTURE:` section with unified diff:

```
STRUCTURE:
--- original
+++ modified
@@ -1,12 +1,16 @@
 - [H1] Overview
-  - [H2] Metadata & Owners
   - [H2] Context
+    - > add glossary table
+    - > reference @docs/legal.md
   - [H2] Problem Statement
 - [H1] Requirements
   - [H2] User Stories
-  - [H2] Functional Logic
+  - [H2] Acceptance Criteria
+- [H1] Design
+  - [H2] User Flows
```

### Why Unified Diff?

1. **Contextual anchoring** — Shows where changes happen relative to neighbors
2. **Training data** — LLMs understand Git diff syntax
3. **Fuzziness** — Handles rename + move gracefully

## Data Model

```rust
struct OutlineNode {
    id: String,
    kind: NodeKind,
    text: String,
    children: Vec<OutlineNode>,
    source_lines: Option<Range<u32>>,
}

enum NodeKind {
    Heading(HeadingLevel),
    Directive,
}

enum HeadingLevel { H1, H2, H3, H4, H5, H6 }
```

## Implementation

### Rust

- Parse markdown → extract heading hierarchy with line ranges
- Track body text attachment between headings
- Store original structure for diff
- On delete: remove annotations in deleted line range
- On session end: serialize trees, diff with `similar`, emit `STRUCTURE:` section

### Frontend

- Overlay component with editable tree
- Visual distinction: headings vs directives
- Keyboard navigation: arrows, Enter to edit, Tab to add child
- Change indicators: green for added, strikethrough for deleted
