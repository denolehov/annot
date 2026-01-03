# Spec: Portals — Live Code Embeds

## Goal

Enable agents to reference real file contents in `review_content` markdown, rendered inline with visual distinction, annotatable with dual-file output.

---

## Design

### Syntax

GitHub-style image link with line anchor:

```markdown
![optional label](path/to/file.go#L42-L58)
```

**Examples:**
```markdown
Here's the auth flow:

![Auth entry point](src/auth/login.go#L42-L58)

Note the retry logic on line 47.
```

**Variations:**
- With label: `![Auth flow](src/auth.go#L42-L58)`
- No label: `![](src/auth.go#L42-L58)`
- Single line: `![](src/auth.go#L42)`

**Note:** Line range is required. Whole-file embeds are not supported.

---

### Visual Rendering

**Inline occurrence** (portal link in prose):
```
13  Check out [📎 this function] in the auth module.
                └─ styled link ─┘
```

**Portal expansion** (always shown, after the paragraph containing inline refs or standalone):
```
    ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
 >  this function — src/auth/login.go#L42-L58
42  func Login(ctx context.Context) error {
43      for attempt := 0; attempt < 3; attempt++ {
44          token, err := authenticate(ctx)
45      }
46  }
    ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
```

**Visual distinction:**
- Dotted top/bottom separator
- Inset shadow (recessed appearance)
- Header: `> {label} — {path}` (muted text)
- Real line numbers from source file (42, 43, 44...)

---

### Architecture

```
Agent → review_content({ content: "![Auth](src/auth.go#L42-L58)" })
         │
         ▼
    MCP Handler
         │── Scan markdown for file reference syntax
         │── Validate paths (blocklist check)
         │── Read files, extract line ranges
         │── Build resolved file map
         ▼
    AppState::from_markdown_with_portals()
         │── Parse markdown AST
         │── Inject header + content lines
         ▼
    Frontend renders with visual distinction
         │
         ▼
    User annotates line 44
         │
         ▼
    Output (dual-reference):

    content.md:5 (embedded):
        > [# QUESTION] Why authenticate inside loop?

    src/auth/login.go:44:
       44 | token, err := authenticate(ctx)
        > [# QUESTION] Why authenticate inside loop?
```

---

### Line Types (State)

```rust
enum LineType {
    // ... existing variants ...
    PortalHeader {
        label: Option<String>,
        source_path: String,  // "src/auth.go#L42-L58"
    },
    PortalContent {
        source_file: PathBuf,
        original_line: u32,
    },
}
```

---

### Security

**Sensitive path blocklist** (validation error if matched):
- `**/id_rsa`, `**/id_ed25519`
- `**/.env`, `**/secrets.*`, `**/credentials.*`
- `**/.aws/*`, `**/.gcp/*`

**Error behavior**: MCP tool call validation error.

---

### Error Cases

All errors returned as MCP validation errors (not rendered in UI):

| Case | Error Message |
|------|--------------|
| File not found | `File not found: src/missing.go` |
| Lines out of range | `Lines 100-110 exceed file length (50 lines): src/file.go` |
| Sensitive file | `Access denied: path matches sensitive file pattern` |
| File too large | `File exceeds 1MB size limit: src/huge.go` |
| No line range | `Line range required: src/auth.go (use #L1-L50)` |

---

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Syntax | `![label](path#L42-L58)` | GitHub-familiar, markdown-native, agents know it |
| Line anchor style | `#L42-L58` | Standard GitHub convention |
| Line range required | Yes | Prevents accidental large embeds |
| Inline behavior | Show link + expand portal | Best of both: prose flows, code visible |
| Resolution timing | Backend pre-resolve | Single round trip, security at MCP boundary |
| Error handling | MCP validation error | Fail early, agent sees error |
| Security | Sensitive path blocklist | Simple, effective |
| Visual | Shadow + dotted border | Clearly "embedded", not native content |
| Line numbering | File's original numbers | 42, 43, 44 — not mixed with doc numbers |
| Caching | Show stale | No live refresh |

---

## Scope

**In:**
- File reference syntax parsing in `review_content`
- Backend file resolution with blocklist validation
- Visual rendering with shadow + border
- Inline link + expanded content display
- Dual-reference annotation output
- Error handling as MCP validation errors

**Out (future):**
- Export/copy with baked content
- Remote URLs (GitHub, etc.)
- Live refresh when file changes
- Expand/collapse UI
- Multi-project scope override
- Whole-file embeds (no line range)
