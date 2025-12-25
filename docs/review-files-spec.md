# Spec: Review.files as Session Participants

## Goal

Refactor `Review.files` to represent all files participating in a review session, enabling portals (markdown links to source code) and multi-file diffs to share a unified annotation model.

---

## Design

### Core Mental Model

```
Review.files = "files that are participants of this review session"
```

Participants are files that can receive annotations:
- Root document (markdown, file, or diff-as-view)
- Source files referenced by portals in markdown
- Individual files touched by a diff
- Future: files from globs, files opened from within annot

**Key principle:** What we display (View) ≠ what we store (files). Views render content; `files` stores annotations.

---

### Data Model

```rust
struct Review {
    /// Annotation targets — files participating in this session
    files: HashMap<PathBuf, AnnotationTarget>,
    
    /// How content is displayed (determines output structure)
    root_view: View,
    
    /// Open windows
    windows: HashMap<WindowId, Window>,
    
    /// Session-level state
    session_comment: Option<Vec<ContentNode>>,
    selected_exit_mode_id: Option<String>,
    config: UserConfig,
}

struct AnnotationTarget {
    /// Annotations on this file, keyed by line range
    annotations: HashMap<LineRange, Annotation>,
    
    /// File metadata (language for syntax highlighting)
    metadata: FileMetadata,
}

enum View {
    /// Single file review
    File { path: PathBuf },
    
    /// Diff review — content in DiffFileView, annotations in files
    Diff { 
        label: String, 
        files: Vec<DiffFileView> 
    },
    
    /// Markdown with portals — content split between markdown + portal chunks
    Markdown { 
        path: PathBuf, 
        portals: Vec<Portal> 
    },
}
```

**Note:** `AnnotationTarget` does NOT store file content. Content lives in:
- `View::File` — loaded at session start
- `View::Diff` — hunks contain diff text
- `View::Markdown` — markdown lines + portal chunks

---

### Diff Architecture

```rust
struct DiffFileView {
    /// Entry in Review.files
    path: PathBuf,
    
    /// For renames
    old_path: Option<PathBuf>,
    
    /// Display content (from diff, not disk)
    hunks: Vec<Hunk>,
    
    /// Where this file starts in flattened display
    line_offset: u32,
}

struct Hunk {
    old_range: LineRange,
    new_range: LineRange,
    lines: Vec<DiffLine>,
}
```

**Diff mode uses ONLY data from the diff itself** — no file system access needed.

---

### Portal Architecture

```rust
struct Portal {
    id: PortalId,
    source_line: u32,           // Where in markdown this portal appears
    label: String,              // Display text from link
    target_path: PathBuf,       // Entry in Review.files
    target_range: LineRange,    // Lines 42-58
    content: Vec<Line>,         // Loaded chunk (for display only)
}
```

**Portal flow:**
1. Parse markdown → detect `[label](path#L42-L58)`
2. Add `path` to `Review.files` (empty AnnotationTarget)
3. Load lines 42-58 into `Portal.content`
4. Render portal inline in markdown view
5. Annotations on portal lines → stored in `Review.files[path]`

---

### Output Format

Group by file, sorted by line:

```
LEGEND:
  [# TAG] Instruction

SESSION:
  Reviewing plan.md with embedded files: src/auth.go
  Apply (Apply the suggested changes)

---

plan.md:14:
    14 | ## Implementation Steps
     > [# QUESTION] Is this the right order?

src/auth.go:45-48:
    45 | func Login() {
     > [# SECURITY] Add error handling
```

**Rules:**
- Include only files with annotations (omit empty targets)
- Primary files first, then portal sources (alphabetical within each)
- When portals present, SESSION block lists embedded files

---

### Frontend Changes

Add `file_path` parameter to annotation IPC:

```typescript
// Extract file from line origin
function getLineFile(line: Line): string {
  switch (line.origin.type) {
    case 'external': return line.origin.file;
    case 'diff': return '__diff_file_' + line.origin.file_index;
    default: return '__primary__';
  }
}

// Pass file_path to backend
await invoke('upsert_annotation', { 
  startLine, endLine, 
  file_path: getLineFile(selectedLine),  // NEW
  content 
});
```

No changes to line rendering (already abstracted via `LineOrigin`).

---

### Selection Boundaries

| Boundary                | Behavior                       |
| ----------------------- | ------------------------------ |
| Markdown ↔ Portal       | Selection stops (hard prevent) |
| Portal ↔ Portal         | Selection stops                |
| Hunk ↔ Hunk (same file) | Selection stops                |
| File ↔ File (diff)      | Selection stops                |

Reuse existing diff hunk boundary logic.

---

## Decisions

| Decision                    | Choice                        | Rationale                                         |
| --------------------------- | ----------------------------- | ------------------------------------------------- |
| Symlinks                    | Don't canonicalize            | Respect user's explicit path choice               |
| Case sensitivity            | Normalize on macOS/Windows    | Prevent duplicate HashMap entries                 |
| Overlapping annotations     | Reject                        | Force user clarity — delete first                 |
| Nested portals (md→md→file) | Depth 1 only                  | Prevent complexity; md-to-md portals rejected     |
| Overlapping portal ranges   | Allow                         | Annotations may appear in both views — acceptable |
| Cross-boundary selection    | Hard prevent                  | Clear semantics, simpler model                    |
| Empty annotation targets    | Omit from output              | Signal-to-noise ratio                             |
| Content storage             | In View, not AnnotationTarget | Annotations orthogonal to content                 |
| Diff content source         | Diff itself only              | No file system access needed for diffs            |

---

## Edge Cases

### Path Resolution
| Case                     | Handling                                        |
| ------------------------ | ----------------------------------------------- |
| Relative path in portal  | Resolve relative to markdown's directory        |
| Non-existent file        | Show error in portal region, don't add to files |
| File deleted mid-session | Keep cached content                             |
| Path with `#` character  | Require URL encoding: `path%23name.rs#L1-10`    |

### Annotations
| Case                            | Handling                                |
| ------------------------------- | --------------------------------------- |
| Range exceeds file length       | Clamp to actual length                  |
| Empty annotation content        | Preserve — line selection is meaningful |
| Annotation on deleted diff line | Allow — references removed content      |

### Diffs
| Case         | Handling                              |
| ------------ | ------------------------------------- |
| Binary file  | Show placeholder, not annotatable     |
| Renamed file | Key by new path, show both in display |
| Empty diff   | Show "no changes", return early       |

---

## Migration Path

### Phase 1: Refactor structures
- Rename `FileState` → `AnnotationTarget`
- Remove content from `AnnotationTarget`
- Add `root_view: View` field to `Review`
- Update `format_output()` to iterate all files

### Phase 2: Diff files as participants
- Parse diff → create `View::Diff` with `DiffFileView` per file
- Add each diff file to `Review.files`
- Remove downstream extraction in output.rs

### Phase 3: Portals
- Parse markdown → detect portals
- Add portal sources to `Review.files`
- Load portal chunks into `Portal.content`
- Frontend: add `file_path` to annotation IPC

### Phase 4: Multi-window (future)
- "Open in new window" from portal
- Load full file content for dedicated window

---

## Scope

**In:**
- `AnnotationTarget` (annotations + metadata, no content)
- `View` enum (File, Diff, Markdown)
- `root_view` field on Review
- Diff files as first-class participants
- Portal detection and chunk loading
- Selection boundary enforcement
- Output format changes
- Frontend `file_path` parameter

**Out (future):**
- Multi-root sessions (`annot **/*.md`)
- Open file from portal in new window
- Live file refresh
- Diff-level annotations (use session comment)
