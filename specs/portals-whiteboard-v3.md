# Portals v3: Inline Design & Security

---

## Visual Design: Truly Inline Portals

Based on your sketch — portals flow in the document with existing line gutter:

```
┌─ Line Gutter ────────────────────────────────────────────┐
│ 13  Some content here...                                 │
│ 14  And more content...                                  │
│     ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
│  >  src/auth/login.go:18-20                              │ ← portal header
│ 18  func Login(ctx context.Context) error {              │ ← file's real line numbers
│ 19      token, err := authenticate(ctx)                  │
│ 20      return err                                       │
│     ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
│ 15  And now we're back to the document                   │
│ 16  More prose here...                                   │
└──────────────────────────────────────────────────────────┘
```

### Visual Distinction for Embedded Lines

Portal lines need to look "embedded" vs native document lines. Options:

**Shadow treatment (your suggestion)**:
- Portal section has subtle `inset-shadow` (like it's recessed)
- Creates depth illusion — "this came from elsewhere"

**Color tinting**:
- Portal lines have faint background tint (e.g., blue at 5% opacity)
- Header line (`> src/auth/login.go`) is slightly stronger tint

**Border treatment**:
- Dotted top/bottom separator (shown as `┄┄┄` above)
- Or: Thin left accent bar for entire portal section

**Combined approach**:
```css
.portal-section {
  box-shadow: inset 0 2px 4px rgba(0,0,0,0.1);
  background: var(--portal-bg);  /* very faint tint */
  border-top: 1px dashed var(--border-subtle);
  border-bottom: 1px dashed var(--border-subtle);
}

.portal-header {
  color: var(--text-muted);
  font-size: 0.9em;
}
```

---

## Syntax: Link-Style with Label

Your insight: `![label](path:lines)` — the label could communicate something!

```markdown
Here's how authentication works:

![Auth entry point](src/auth/login.go:42-58)

The retry logic at line 47 handles transient failures.
```

### How the Label Gets Used

**In annot UI**:
```
┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
 >  Auth entry point — src/auth/login.go:42-58
42  func Login(ctx context.Context) error {
43      for attempt := 0; attempt < 3; attempt++ {
...
┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
```

**On export** (baked markdown):
```markdown
**Auth entry point** — `src/auth/login.go:42-58`
```go
func Login(ctx context.Context) error {
    for attempt := 0; attempt < 3; attempt++ {
        ...
    }
}
```
```

### Syntax Comparison

| Syntax | Example | Agent Natural? | Label Support |
|--------|---------|---------------|---------------|
| Link-style | `![Auth flow](src/auth.go:1-10)` | High (knows markdown) | ✓ |
| @ref | `@src/auth.go:1-10` | High (short) | ✗ |
| Code block | `` ```portal:src/auth.go:1-10``` `` | Medium | ✗ |
| MDX | `<FileRef path="..." />` | Low | ✓ (via props) |

**Recommendation**: Link-style wins — markdown-native, label support, agents already emit `![...](...)`

### Edge Cases

**No label** — agent just wants to embed, no commentary:
```markdown
![](src/auth/login.go:42-58)
```
Renders with just the path:
```
 >  src/auth/login.go:42-58
42  func Login...
```

**Multi-portal sequence** — agent showing a flow:
```markdown
Request handling starts here:
![1. Route matching](src/router.go:12-20)

Then dispatches to handler:
![2. Handler dispatch](src/handlers/dispatch.go:5-15)

Finally renders response:
![3. Response render](src/render.go:30-45)
```

---

## Security: Oracle Findings Summary

**Threat**: LLM confusion/prompt injection → `@../../../../etc/passwd`
**Blast radius**: HIGH — any file readable by user (SSH keys, cloud creds, .env)

### Recommendation: Sandbox to Project Root

```rust
fn resolve_portal_path(path: &str, project_root: &Path) -> Result<PathBuf> {
    let full_path = project_root.join(path);
    let canonical = fs::canonicalize(&full_path)?;

    // Verify still within sandbox
    if !canonical.starts_with(project_root) {
        return Err("Portal path escapes project root");
    }
    Ok(canonical)
}
```

**Project root detection**:
- CLI mode: CWD (your decision)
- Git-aware fallback: `git rev-parse --show-toplevel`
- Override: `--portal-scope=/custom/root` flag

**Defense in depth**:
- File size limit: 1MB default
- Binary file warning
- Sensitive path blocklist (`**/id_rsa`, `**/.env`, `**/.aws/*`)

**Error behavior**: Return MCP validation error (your decision):
```json
{
  "error": "Portal path escapes project root: ../../../../etc/passwd"
}
```

### UX Impact

- 90% use cases: No impact (files within project)
- 10% use cases: Need `--portal-scope` for monorepo parents, etc.

---

## Resolution Timing: Hybrid Approach

Backend scans, resolves, provides file map:

```typescript
// MCP response structure
{
  content: "Here's the auth flow:\n\n![Auth entry point](src/auth/login.go:42-58)\n\nNote the retry...",
  portals: {
    "src/auth/login.go:42-58": {
      content: "func Login(ctx context.Context) error {\n    for attempt...",
      language: "go",
      startLine: 42,
      endLine: 58
    }
  }
}
```

Frontend just renders using the map — single round trip, no IPC chatter.

---

## Annotations on Portals

You confirmed this is a **main feature**. Behavior:

**User annotates portal line 44**:
```
 >  src/auth/login.go:42-58
42  func Login(ctx context.Context) error {
43      for attempt := 0; attempt < 3; attempt++ {
44  >>>     token, err := authenticate(ctx)  <<<  [annotation here]
45          if err == nil {
```

**Output (multi-file format)**:
```
SESSION:
  Exit: Apply

---

content.md:5 (portal):
    > [# QUESTION] Why authenticate inside loop?

---

src/auth/login.go:44:
   44 | token, err := authenticate(ctx)
    > [# QUESTION] Why authenticate inside loop?
```

Portal annotations reference **both** the document location AND the original file.

---

## Open Decisions

Still need your picks on:

1. **Visual treatment** — Shadow? Tint? Both? Border style?
2. **Export format** — Prose line before code block? (My lean: yes)
3. **Syntax confirmation** — Link-style `![label](path:lines)`?
