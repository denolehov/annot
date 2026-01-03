# Multi-File Sessions: Playlist Model

## Core Concept

Files are a **queue**. You're always looking at one file, but there's a sense of progression and you can jump anywhere.

```
┌─────────────────────────────────────────────────────────────┐
│  ◀  auth.rs                              2/5  ▶    [Apply] │
├─────────────────────────────────────────────────────────────┤
│   1 │ fn authenticate() {                                   │
│   2 │     let token = get_token();                          │
│   3 │     validate(token)?;                                 │
│   4 │     Ok(())                                            │
│   5 │ }                                                     │
│                                                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
        ●───●───◉───○───○
            ↑ clickable progress indicator
```

---

## Navigation

| Key       | Action                             |
| --------- | ---------------------------------- |
| `←` / `h` | Previous file                      |
| `→` / `l` | Next file                          |
| `Ctrl+P`  | Command palette (fuzzy find files) |
| `1-9`     | Jump to file 1-9 directly          |
| Click dot | Jump to that file                  |

**Command Palette** (`Ctrl+P`):
```
┌─────────────────────────────────────────────────────────────┐
│ > auth                                                      │
├─────────────────────────────────────────────────────────────┤
│   src/auth.rs .......................... 2 annotations      │
│   src/db/auth_tokens.rs ................ —                  │
│   tests/auth_test.rs ................... 1 annotation       │
└─────────────────────────────────────────────────────────────┘
```

- Fuzzy match on filename
- Shows annotation count per file
- `Enter` jumps to file, `Esc` cancels

---

## Header Anatomy

```
┌─────────────────────────────────────────────────────────────┐
│  ◀  auth.rs                              2/5  ▶    [Apply] │
│     ↑                                    ↑ ↑       ↑
│     filename (truncated with ... if long) │ │       exit mode
│                                           │ total files
│                                           current position
└─────────────────────────────────────────────────────────────┘
```

When agent provides per-file context:
```
┌─────────────────────────────────────────────────────────────┐
│  ◀  auth.rs                              2/5  ▶    [Apply] │
│     Entry point — validates JWT tokens                      │
├─────────────────────────────────────────────────────────────┤
```

---

## Progress Indicator States

```
○ = unannotated (no feedback yet)
● = has annotation(s)
◉ = current file

Examples:
●───●───◉───○───○   File 3 of 5, first two have annotations
○───◉───○───○───○   File 2 of 5, nothing annotated yet
●───●───●───●───◉   On last file, all others annotated
```

For many files (>7), collapse to:
```
●●●◉○○○○○○○○  (12 files, showing dots)

or with count:
◉ 1/47  [●12 ○35]   (12 annotated, 35 unannotated)
```

---

## Entry Points

### CLI
```bash
annot file1.rs file2.rs file3.rs     # explicit list
annot "src/**/*.rs"                   # glob (shell expands)
annot $(git diff --name-only HEAD~3)  # from git
annot .                               # all tracked files
```

### MCP Tool: `review_files`

```typescript
interface ReviewFilesInput {
  files: Array<{
    path: string;
    context?: string;  // shown as subtitle in header
  }>;
  session_context?: string;  // overall session comment
  exit_modes?: ExitMode[];
}
```

Example agent call:
```json
{
  "files": [
    { "path": "src/auth.rs", "context": "Entry point" },
    { "path": "src/db/users.rs", "context": "User model" },
    { "path": "src/routes/login.rs", "context": "HTTP handler" }
  ],
  "session_context": "Review the authentication flow"
}
```

**Order is preserved.** Agent orders for narrative, globs order by filesystem.

---

## Output Format

```
LEGEND:
  [# SECURITY] Check for injection vulnerabilities

SESSION:
  Context: Review the authentication flow
  Exit: Apply (instruction text)

  Files: 3
    auth.rs .............. 2 annotations
    db/users.rs .......... —
    routes/login.rs ...... 1 annotation

---

auth.rs:10-12:
   10 │ fn authenticate(token: &str) -> Result<User> {
   11 │     let claims = decode_jwt(token)?;
   12 │     load_user(claims.sub)
       └──> [# SECURITY] Verify token isn't expired

auth.rs:25-25:
   25 │     Ok(user)
       └──> Consider logging successful auth

---

routes/login.rs:8-11:
    8 │ async fn login(body: Json<LoginRequest>) -> Response {
    9 │     let user = authenticate(&body.token)?;
   10 │     set_cookie(user.id);
   11 │     Ok(Json(user))
       └──> Rate limit this endpoint
```

---

## Multi-File Diff

Diffs naturally become multi-file playlists:

```bash
annot --diff                     # git diff HEAD (default)
annot --diff HEAD~3..HEAD        # range
annot --diff feature-branch      # vs current
```

Each file in the diff becomes a playlist item:
```
┌─────────────────────────────────────────────────────────────┐
│  ◀  auth.rs (+15 -3)                     2/4  ▶   [Reject] │
├─────────────────────────────────────────────────────────────┤
│ @@ -10,6 +10,8 @@                                           │
│   10 │   let token = get_token();                           │
│ + 11 │   validate(token)?;  // NEW                          │
│ + 12 │   log_attempt();     // NEW                          │
│   13 │   Ok(())                                             │
└─────────────────────────────────────────────────────────────┘
        ●───◉───○───○
```

Header shows `(+15 -3)` change stats per file.

---

## Decisions (locked in)

| Decision               | Choice              | Rationale                                                  |
| ---------------------- | ------------------- | ---------------------------------------------------------- |
| Cross-file annotations | No                  | Keep annotations file-scoped; use text to reference others |
| Exit mode scope        | Session-level       | One decision applies to all files                          |
| File order             | Controlled by input | Agent orders for narrative, globs preserve fs order        |
| Large file sets        | Command palette     | `Ctrl+P` fuzzy finder scales to any size                   |

---

## Open Questions [?]

1. **Empty files in playlist?** If a file has no annotations when you finish, include it in output?
   - Option A: List all files (shows what was reviewed)
   - Option B: Only files with annotations (less noise)
   - [?] Leaning toward A for completeness

2. **"Skip" behavior?** Should there be a way to mark a file as "reviewed, no feedback"?
   - Explicit skip vs just having no annotations
   - [?] Maybe progress dot changes color when visited?

3. **Session persistence?** If you accidentally close, can you resume?
   - Current: ephemeral, everything lost
   - [?] Could auto-save state for recovery

4. **Diff-specific**: File added/deleted in diff — how to show?
   - Added: Just show new content, header says "(new file)"
   - Deleted: Show old content, header says "(deleted)"
