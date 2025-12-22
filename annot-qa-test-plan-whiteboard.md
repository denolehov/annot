# annot QA Test Plan — Comprehensive Testing Artifact

## Executive Summary

This document consolidates **500+ test scenarios** gathered by 6 parallel QA expert agents, covering every aspect of annot's functionality. Use this as your reference when testing the app to ensure it's robust, reliable, and bullet-proof.

---

## Test Category Overview

| Category                    | Scenarios   | High Risk   | Coverage                              |
| --------------------------- | ----------- | ----------- | ------------------------------------- |
| **A. Input Modes**          | 100+        | 15          | File, diff, content, stdin            |
| **B. Annotation CRUD**      | 80+         | 12          | Create, update, delete, content nodes |
| **C. UI/UX Interactions**   | 106         | 26          | Keyboard, mouse, focus, accessibility |
| **D. Tags & Exit Modes**    | 75          | 6           | CRUD, persistence, validation         |
| **E. Output Format/MCP**    | 74          | 17          | Structured output, MCP integration    |
| **F. Platform/Integration** | 100+        | 15          | CLI, filesystem, window lifecycle     |

**Total: ~535 scenarios, ~91 high-risk**

---

## A. INPUT MODES (File/Diff/Content/Stdin)

### A.1 review_file Mode

#### Critical Scenarios (Test First)
| ID   | Scenario                        | Risk     | Notes                               |
| ---- | ------------------------------- | -------- | ----------------------------------- |
| A1.1 | **File not found**              | CRITICAL | Exit with clear error message       |
| A1.2 | **Permission denied**           | CRITICAL | Exit with error, no crash           |
| A1.3 | **Binary file (PNG, PDF, ELF)** | CRITICAL | Graceful error, not garbled output  |
| A1.4 | **Invalid UTF-8 sequences**     | CRITICAL | fs::read_to_string fails gracefully |
| A1.5 | **Empty file (0 bytes)**        | MEDIUM   | Opens, shows empty state            |

#### Edge Cases
| ID    | Scenario                               | Risk   | Notes                                  |
| ----- | -------------------------------------- | ------ | -------------------------------------- |
| A1.6  | Very long line (>10KB)                 | HIGH   | Horizontal scroll or wrap, no crash    |
| A1.7  | 100K+ lines                            | HIGH   | Virtual scroll, acceptable performance |
| A1.8  | File with mixed line endings (CRLF/LF) | MEDIUM | Lines parsed correctly                 |
| A1.9  | Unicode filename (日本語.rs)           | MEDIUM | UTF-8 filename handling                |
| A1.10 | File with no extension (Makefile)      | LOW    | Plain text fallback                    |

### A.2 review_diff Mode

#### Critical Scenarios
| ID   | Scenario                   | Risk     | Notes                               |
| ---- | -------------------------- | -------- | ----------------------------------- |
| A2.1 | **Git not installed**      | CRITICAL | Clear error message                 |
| A2.2 | **Invalid git reference**  | HIGH     | Git error captured, returned        |
| A2.3 | **Not in git repository**  | HIGH     | "fatal: not a git repository" error |
| A2.4 | **Malformed diff content** | HIGH     | is_diff returns false, error        |
| A2.5 | **Empty diff string**      | HIGH     | Error: "Not a valid diff"           |

#### Edge Cases
| ID    | Scenario                   | Risk   | Notes                           |
| ----- | -------------------------- | ------ | ------------------------------- |
| A2.6  | 1000+ file diff            | HIGH   | Parse all files, track metadata |
| A2.7  | Huge hunk (10K+ lines)     | HIGH   | Memory usage acceptable         |
| A2.8  | Binary file in diff        | MEDIUM | "Binary files differ" handled   |
| A2.9  | File path with spaces      | MEDIUM | Paths extracted correctly       |
| A2.10 | Rename detection (-M flag) | LOW    | old_name/new_name populated     |

### A.3 review_content Mode (Ephemeral)

#### Critical Scenarios
| ID   | Scenario                 | Risk   | Notes                           |
| ---- | ------------------------ | ------ | ------------------------------- |
| A3.1 | **Empty content string** | HIGH   | Error or empty state (clarify)  |
| A3.2 | **1GB+ content**         | HIGH   | Memory limits, graceful failure |

#### Features to Test
| ID   | Scenario                     | Risk   | Notes                              |
| ---- | ---------------------------- | ------ | ---------------------------------- |
| A3.3 | Markdown with tables         | LOW    | Tables formatted, aligned          |
| A3.4 | Markdown with code blocks    | LOW    | Syntax highlighted per language    |
| A3.5 | Image paste (ephemeral=true) | MEDIUM | Base64 captured, stored            |
| A3.6 | Custom exit modes from MCP   | LOW    | Override defaults, colors assigned |

### A.4 Stdin Mode

#### Critical Scenarios
| ID   | Scenario                | Risk     | Notes                       |
| ---- | ----------------------- | -------- | --------------------------- |
| A4.1 | **Empty stdin**         | CRITICAL | "stdin is empty" error      |
| A4.2 | **Invalid UTF-8 piped** | CRITICAL | read_to_string fails, error |
| A4.3 | **Both stdin and file** | MEDIUM   | File wins, warning printed  |

---

## B. ANNOTATION CRUD OPERATIONS

### B.1 Basic Operations

| ID   | Scenario                             | Risk   | Notes                         |
| ---- | ------------------------------------ | ------ | ----------------------------- |
| B1.1 | Create single-line annotation        | LOW    | Lines 5-5, content stored     |
| B1.2 | Create multi-line annotation         | LOW    | Lines 10-15, normalized key   |
| B1.3 | Reverse selection (end before start) | LOW    | Auto-normalized to min-max    |
| B1.4 | Edit existing annotation (upsert)    | LOW    | Same key replaces content     |
| B1.5 | Delete annotation                    | LOW    | HashMap remove, state updated |

### B.2 Content Node Types

| ID   | Scenario                          | Risk   | Notes                     |
| ---- | --------------------------------- | ------ | ------------------------- |
| B2.1 | Text-only annotation              | LOW    | ContentNode::Text stored  |
| B2.2 | Single tag ([# SECURITY])         | LOW    | Tag in LEGEND, deduped    |
| B2.3 | Text + Tag combination            | MEDIUM | Vec<ContentNode> ordered  |
| B2.4 | Media node (image paste)          | MEDIUM | [Figure 1] placeholder    |
| B2.5 | Excalidraw diagram                | HIGH   | JSON + PNG, mode-specific |
| B2.6 | Multiple media (figure numbering) | MEDIUM | Global counter increments |

### B.3 Boundary Conditions

| ID   | Scenario                      | Risk   | Notes                                  |
| ---- | ----------------------------- | ------ | -------------------------------------- |
| B3.1 | Annotation on line 1          | MEDIUM | No context line (line 0 doesn't exist) |
| B3.2 | Annotation on last line       | LOW    | Context line from line-1               |
| B3.3 | Annotation beyond file length | HIGH   | .get() returns None, silent failure    |
| B3.4 | Overlapping annotations       | LOW    | Different keys coexist                 |
| B3.5 | Adjacent annotations          | LOW    | Both in output, sorted                 |

### B.4 Session Operations

| ID   | Scenario                                    | Risk   | Notes                       |
| ---- | ------------------------------------------- | ------ | --------------------------- |
| B4.1 | Session comment (press 'g')                 | LOW    | session_comment stored      |
| B4.2 | Exit mode selection (Tab)                   | LOW    | selected_exit_mode_id set   |
| B4.3 | Exit mode cycling (Tab/Shift+Tab)           | MEDIUM | Modulo arithmetic, wrapping |
| B4.4 | Full session (comment + mode + annotations) | LOW    | All in output, proper order |

---

## C. UI/UX INTERACTIONS

### C.1 Keyboard Shortcuts — Global

| Key         | Action                         | Condition            | Risk   |
| ----------- | ------------------------------ | -------------------- | ------ |
| `Tab`       | Cycle exit mode forward        | No editor open       | MEDIUM |
| `Shift+Tab` | Cycle exit mode backward       | No editor open       | MEDIUM |
| `g`         | Open session comment           | No editor open       | MEDIUM |
| `c`         | Annotate hovered line          | Hover + no selection | MEDIUM |
| `:`         | Open CommandPalette            | No editor open       | HIGH   |
| `Cmd+S`     | Save modal                     | Any time             | LOW    |
| `Cmd+=`     | Zoom in                        | Any time             | LOW    |
| `Cmd+-`     | Zoom out                       | Any time             | LOW    |
| `Cmd+0`     | Reset zoom                     | Any time             | LOW    |
| `Escape`    | Dismiss innermost modal/editor | Modal open           | HIGH   |

### C.2 Line Selection

| ID   | Scenario                   | Risk   | Notes                       |
| ---- | -------------------------- | ------ | --------------------------- |
| C2.1 | Single click gutter        | LOW    | Line selected, editor opens |
| C2.2 | Drag gutter range          | LOW    | Multi-line selection        |
| C2.3 | Shift+click extend         | MEDIUM | Range extension             |
| C2.4 | Click + button             | LOW    | Single line selected        |
| C2.5 | Cannot select diff header  | MEDIUM | isLineSelectable check      |
| C2.6 | Cannot cross hunk boundary | HIGH   | Selection constrained       |

### C.3 Annotation Editor

| ID   | Scenario                      | Risk   | Notes                     |
| ---- | ----------------------------- | ------ | ------------------------- |
| C3.1 | Editor opens on selection     | LOW    | Focus in TipTap           |
| C3.2 | Sealed editor click to unseal | MEDIUM | Toggle editable state     |
| C3.3 | `#` triggers tag autocomplete | HIGH   | Menu appears, navigation  |
| C3.4 | `/` triggers slash commands   | MEDIUM | Command menu              |
| C3.5 | Create tag from selection     | HIGH   | Popover → CommandPalette  |
| C3.6 | Image paste (MCP mode only)   | MEDIUM | CLI shows "not supported" |

### C.4 CommandPalette (Poly-Editor)

| ID   | Scenario            | Risk   | Notes                      |
| ---- | ------------------- | ------ | -------------------------- |
| C4.1 | Namespace selection | MEDIUM | Arrow nav, Enter to select |
| C4.2 | Item filtering      | MEDIUM | Real-time filter           |
| C4.3 | Create new tag      | HIGH   | Form, Cmd+Enter saves      |
| C4.4 | Edit existing tag   | MEDIUM | Pre-filled form            |
| C4.5 | Delete tag/mode     | HIGH   | Pending-delete state       |
| C4.6 | Reorder exit modes  | MEDIUM | Cmd+Alt+Up/Down            |
| C4.7 | Copy to clipboard   | LOW    | Content/annotations/both   |
| C4.8 | Escape navigation   | MEDIUM | Back through states        |

### C.5 Focus & Accessibility

| ID   | Scenario                 | Risk   | Notes                        |
| ---- | ------------------------ | ------ | ---------------------------- |
| C5.1 | Focus trap in modals     | MEDIUM | Escape closes, Tab navigates |
| C5.2 | Focus visible indicators | MEDIUM | :focus-visible styles        |
| C5.3 | Keyboard-only navigation | HIGH   | All actions possible         |
| C5.4 | Screen reader support    | HIGH   | ARIA roles, announcements    |
| C5.5 | Color contrast (WCAG AA) | LOW    | 4.5:1 ratio                  |

### C.6 Window Behaviors

| ID   | Scenario                          | Risk   | Notes                  |
| ---- | --------------------------------- | ------ | ---------------------- |
| C6.1 | Draggable header region           | MEDIUM | data-tauri-drag-region |
| C6.2 | Traffic lights position (macOS)   | HIGH   | (12, 22) from top-left |
| C6.3 | Window resize responsive          | HIGH   | Content reflows        |
| C6.4 | Window hidden until content loads | MEDIUM | No white flash         |

---

## D. TAGS & EXIT MODES

### D.1 Tag Operations

| ID   | Scenario                   | Risk   | Notes                     |
| ---- | -------------------------- | ------ | ------------------------- |
| D1.1 | Create tag with valid name | LOW    | Saved to tags.json        |
| D1.2 | Tag with empty name        | MEDIUM | Validation error          |
| D1.3 | Tag with duplicate name    | MEDIUM | Different IDs, both saved |
| D1.4 | Tag with special chars     | MEDIUM | Escaped for HTML          |
| D1.5 | Tag with Unicode           | MEDIUM | UTF-8 preserved           |
| D1.6 | Edit tag name              | LOW    | ID unchanged              |
| D1.7 | Delete tag                 | MEDIUM | deleted_tag_ids tracked   |

### D.2 Exit Mode Operations

| ID   | Scenario               | Risk   | Notes                    |
| ---- | ---------------------- | ------ | ------------------------ |
| D2.1 | Create with all fields | LOW    | Saved to exit-modes.json |
| D2.2 | Invalid hex color      | MEDIUM | Validation error         |
| D2.3 | Empty instruction      | MEDIUM | Validation error         |
| D2.4 | Order field respected  | LOW    | Sorted by order          |
| D2.5 | Ephemeral from MCP     | MEDIUM | is_ephemeral=true        |
| D2.6 | Delete selected mode   | MEDIUM | Selection cleared        |

### D.3 Persistence & Concurrency

| ID   | Scenario                             | Risk   | Notes                  |
| ---- | ------------------------------------ | ------ | ---------------------- |
| D3.1 | Tags persist across restart          | MEDIUM | tags.json roundtrip    |
| D3.2 | Corrupt JSON graceful fallback       | MEDIUM | Empty list, no crash   |
| D3.3 | **Concurrent saves (two instances)** | HIGH   | File locking (fs4)     |
| D3.4 | Config dir auto-created              | MEDIUM | ensure_config_dir()    |
| D3.5 | Disk full during save                | MEDIUM | Error logged, graceful |

### D.4 Autocomplete & Search

| ID   | Scenario                       | Risk   | Notes                   |
| ---- | ------------------------------ | ------ | ----------------------- |
| D4.1 | Tag autocomplete prefix match  | LOW    | Case-insensitive filter |
| D4.2 | No matches shows create option | LOW    | "Create [query]" shown  |
| D4.3 | Tags deduplicated in LEGEND    | HIGH   | BTreeMap deduplication  |

---

## E. OUTPUT FORMAT & MCP INTEGRATION

### E.1 Output Format Correctness

| ID   | Scenario                           | Risk   | Notes                               |                |
| ---- | ---------------------------------- | ------ | ----------------------------------- | -------------- |
| E1.1 | **Empty session (no annotations)** | HIGH   | Empty string output                 |                |
| E1.2 | Single-line annotation format      | MEDIUM | `file.rs:5\n> 5                     | ...\n└──> ...` |
| E1.3 | Multi-line with context            | MEDIUM | Context line (start-1) if non-empty |                |
| E1.4 | Line number width alignment        | MEDIUM | Right-aligned, consistent           |                |
| E1.5 | Multiple annotations sorted        | MEDIUM | By start_line ascending             |                |
| E1.6 | Separator between annotations      | LOW    | `---` on own line                   |                |

### E.2 Legend & Session Blocks

| ID   | Scenario                        | Risk   | Notes                    |
| ---- | ------------------------------- | ------ | ------------------------ |
| E2.1 | LEGEND with tags (alphabetical) | MEDIUM | BTreeMap → A-Z order     |
| E2.2 | Tag deduplication               | HIGH   | Same tag once in legend  |
| E2.3 | SESSION with exit mode          | HIGH   | Name + instruction       |
| E2.4 | SESSION with comment            | MEDIUM | Comment before exit mode |
| E2.5 | No LEGEND when no tags          | LOW    | Block omitted            |

### E.3 Special Characters

| ID   | Scenario               | Risk   | Notes                 |                         |
| ---- | ---------------------- | ------ | --------------------- | ----------------------- |
| E3.1 | Quotes in annotation   | LOW    | Preserved as-is       |                         |
| E3.2 | Newlines in annotation | MEDIUM | Indented continuation |                         |
| E3.3 | Unicode and emoji      | LOW    | UTF-8 pass-through    |                         |
| E3.4 | Angle brackets `<>`    | MEDIUM | Not HTML-encoded      |                         |
| E3.5 | Pipe character `       | `      | MEDIUM                | No conflict with format |

### E.4 Output Modes (CLI/MCP/Clipboard)

| ID   | Scenario                      | Risk   | Notes                      |
| ---- | ----------------------------- | ------ | -------------------------- |
| E4.1 | CLI mode: full data inline    | MEDIUM | Base64 + JSON in text      |
| E4.2 | MCP mode: images separated    | HIGH   | Text + FormatResult.images |
| E4.3 | Clipboard mode: compact       | LOW    | No JSON blobs              |
| E4.4 | Figure numbering across modes | MEDIUM | Global counter             |

### E.5 MCP Integration

| ID   | Scenario                             | Risk   | Notes                             |
| ---- | ------------------------------------ | ------ | --------------------------------- |
| E5.1 | **finish_session via channel (MCP)** | HIGH   | ResultSender used                 |
| E5.2 | **finish_session via stdout (CLI)**  | HIGH   | Print + app.exit()                |
| E5.3 | MCP response header                  | MEDIUM | "=== REVIEW SESSION COMPLETE ===" |
| E5.4 | MCP response with images             | HIGH   | Separate Content items            |
| E5.5 | MCP empty session message            | MEDIUM | "completed without annotations"   |

---

## F. PLATFORM & INTEGRATION

### F.1 CLI Argument Parsing

| ID   | Scenario               | Risk   | Notes                              |
| ---- | ---------------------- | ------ | ---------------------------------- |
| F1.1 | Relative path          | LOW    | Resolved from cwd                  |
| F1.2 | Absolute path          | LOW    | Used directly                      |
| F1.3 | Path with spaces       | MEDIUM | Quoted handling                    |
| F1.4 | Symlink to file        | MEDIUM | Followed, label shows symlink name |
| F1.5 | Broken symlink         | MEDIUM | Error: file not found              |
| F1.6 | `annot mcp` subcommand | HIGH   | MCP server starts                  |

### F.2 Config Directory

| ID   | Scenario                  | Risk   | Notes                     |
| ---- | ------------------------- | ------ | ------------------------- |
| F2.1 | Linux: ~/.config/annot/   | MEDIUM | XDG_CONFIG_HOME respected |
| F2.2 | macOS: ~/.config/annot/   | HIGH   | Consistent with Linux     |
| F2.3 | Windows: %APPDATA%\annot\ | MEDIUM | dirs::config_dir()        |
| F2.4 | Dir doesn't exist         | MEDIUM | Auto-created              |
| F2.5 | Dir not writable          | MEDIUM | Save fails gracefully     |

### F.3 Window Lifecycle (CLI)

| ID   | Scenario                      | Risk   | Notes                     |
| ---- | ----------------------------- | ------ | ------------------------- |
| F3.1 | Window opens after content    | LOW    | visible: false initially  |
| F3.2 | Default size 1000x700         | LOW    | tauri.conf.json           |
| F3.3 | Close triggers finish_session | LOW    | Output printed, exit      |
| F3.4 | Cmd+Q (macOS)                 | MEDIUM | May bypass finish_session |
| F3.5 | DevTools in debug build       | LOW    | Opened automatically      |

### F.4 Window Lifecycle (MCP)

| ID   | Scenario                         | Risk   | Notes                      |
| ---- | -------------------------------- | ------ | -------------------------- |
| F4.1 | Window hidden initially          | LOW    | Created visible: false     |
| F4.2 | **Tool call blocks until close** | HIGH   | tokio::spawn_blocking      |
| F4.3 | **Concurrent tool calls**        | HIGH   | Windows serialize or queue |
| F4.4 | should_exit=false in MCP         | LOW    | Prevents app.exit()        |

### F.5 macOS-Specific

| ID   | Scenario                   | Risk   | Notes                       |
| ---- | -------------------------- | ------ | --------------------------- |
| F5.1 | Traffic lights at (12, 22) | HIGH   | Vertically centered         |
| F5.2 | TitleBarStyle: Overlay     | LOW    | Content under title bar     |
| F5.3 | Hidden title               | LOW    | No "annot" text             |
| F5.4 | Dock icon (CLI)            | LOW    | Visible while running       |
| F5.5 | No dock icon (MCP)         | LOW    | ActivationPolicy::Accessory |

### F.6 Performance

| ID   | Scenario                      | Risk   | Notes                  |
| ---- | ----------------------------- | ------ | ---------------------- |
| F6.1 | Small file startup < 1s       | LOW    | Baseline               |
| F6.2 | Large file (171KB) < 5s       | MEDIUM | Parsing + highlighting |
| F6.3 | Virtual scroll 100K lines     | HIGH   | 60fps scrolling        |
| F6.4 | Memory < 500MB for large file | MEDIUM | No leaks               |

---

## Priority Testing Matrix

### CRITICAL (Test First, Block Release)
1. File not found / permission denied errors
2. Binary file / invalid UTF-8 handling
3. Empty stdin / empty file edge cases
4. finish_session CLI vs MCP paths
5. Concurrent config saves (file locking)
6. MCP tool call blocking behavior
7. Tag deduplication in LEGEND
8. Exit mode in SESSION output

### HIGH RISK (Test Thoroughly)
1. Very long lines / very large files
2. Diff mode with invalid/malformed diffs
3. Tag autocomplete navigation
4. CommandPalette tag creation
5. Line selection constraints (hunk boundaries)
6. Traffic lights position (macOS)
7. Window resize responsiveness
8. Figure numbering across media types

### MEDIUM RISK (Standard Testing)
1. All annotation CRUD operations
2. Content node combinations
3. Keyboard shortcuts in all contexts
4. Focus management in modals
5. Config directory creation
6. Output format alignment
7. MCP response structure

### LOW RISK (Sanity Checks)
1. Basic file open
2. Single annotation creation
3. Copy/save operations
4. Zoom in/out/reset
5. Exit mode color rendering

---

## Test Execution Checklist

### Setup
```bash
# Clean config for isolated testing
rm -rf ~/.config/annot/

# Build debug binary
pnpm tauri build --debug

# Run with perf timing
ANNOT_PERF=1 ./target/debug/annot test-fixtures/sample.md
```

### Fixtures
- `test-fixtures/sample.md` (17.5KB) — basic markdown
- `test-fixtures/large.md` (171KB) — performance testing
- Create edge case fixtures as needed (unicode filenames, binary, etc.)

### Verification Checklist
- [ ] No crashes or panics
- [ ] Error messages clear and actionable
- [ ] Data persists correctly across restart
- [ ] Concurrent access doesn't corrupt config
- [ ] Output format matches spec
- [ ] All keyboard shortcuts work
- [ ] Focus management correct in all modals
- [ ] Traffic lights position correct (macOS)
- [ ] Performance acceptable for large files

---

## Notes for Testers

1. **Start with Critical scenarios** — these are release blockers
2. **Test on multiple platforms** — macOS, Linux, Windows
3. **Test both CLI and MCP modes** — they have different code paths
4. **Monitor memory** with large files — watch for leaks
5. **Test concurrent access** — open two instances, save simultaneously
6. **Verify output parsing** — output should be machine-readable
7. **Check accessibility** — keyboard-only navigation must work

---

*Generated by 6 parallel QA expert agents analyzing the annot codebase*