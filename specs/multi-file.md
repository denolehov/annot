# Spec: Multi-File Sessions (Playlist Model)

## Goal

Enable annot to open multiple files in a single session, navigating between them like a playlist while preserving the ephemeral, keyboard-first annotation experience.

## Design

### Data Model Change

```
AppState (extended)
├── files: Vec<FileEntry>         // NEW: ordered list of files
│   ├── path: String
│   ├── label: String
│   ├── lines: Vec<Line>
│   ├── annotations: HashMap<String, Annotation>
│   └── diff_metadata: Option<DiffMetadata>  // per-file for multi-file diff
├── current_file_index: usize     // NEW: active file
├── tags: Vec<Tag>                // shared across session
├── exit_modes: Vec<ExitMode>     // shared across session
├── selected_exit_mode_id: Option<String>
└── session_comment: Option<Vec<ContentNode>>
```

Single-file mode: `files.len() == 1`, no progress counter shown.

### Header Layout

```
[●●●] <filename> [· diff-stats] [· hunk-context]    [progress]  [⎘] [💾]
                                                    ◉ 2/5 [●1 ○4]
```

Progress format: `◉ current/total [●annotated ○unannotated]`

### Navigation

| Key              | Action           |
| ---------------- | ---------------- |
| `←` / `h`        | Previous file    |
| `→` / `l`        | Next file        |
| `1-9`            | Jump to file 1-9 |
| `:files <query>` | Fuzzy find file  |

### CLI Interface

```bash
annot file1.rs file2.rs           # explicit list
annot "src/**/*.rs"                # glob
annot $(git diff --name-only)      # from git
annot .                            # tracked files (git ls-files)
annot -a .                         # all files including untracked
```

### MCP Tool: `review_files`

```typescript
interface ReviewFilesInput {
  files: Array<{ path: string }>;
  exit_modes?: ExitMode[];
}
```

### Multi-File Diff

```bash
git diff | annot
```

Each file in the diff becomes a playlist entry. Within each file, existing hunk navigation (`j`/`k`) works as before.

### Output Format

Unchanged structure. Only files with annotations appear:

```
SESSION:
  Exit: Apply (merge changes)

---

auth.rs:10-12:
   10 │ code
       └──> annotation

---

routes/login.rs:8-11:
    8 │ code
       └──> annotation
```

## Decisions

| Decision               | Choice           | Rationale                                   |
| ---------------------- | ---------------- | ------------------------------------------- |
| Cross-file annotations | No               | Keep annotations file-scoped; simpler model |
| Exit mode scope        | Session-level    | One decision applies to all files           |
| File order             | Input-controlled | Agent/CLI determines sequence               |
| Empty files in output  | Omitted          | Less noise, matches current behavior        |
| Large file sets        | `:files` command | Scales to any size                          |
| Untracked files        | `-a` flag        | Explicit opt-in                             |

## Open Questions

None.

## Scope

**In:**
- Multiple files in single session
- Progress counter in header
- `←`/`→`/`h`/`l` navigation between files
- `1-9` quick jump
- `:files` command palette namespace
- CLI: multiple args, `.`, `-a` flag
- MCP: `review_files` tool
- Multi-file diff as playlist

**Out:**
- Per-file `context` from agent (future)
- `session_context` overall guidance (future)
- Session persistence/recovery (future)
- Cross-file annotations
- Per-file exit modes
