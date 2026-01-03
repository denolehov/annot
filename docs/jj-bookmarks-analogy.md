# Jujutsu (jj) and annot Bookmarks: A Conceptual Alignment

> **Status**: Exploratory analysis — patterns and metaphors, not a feature spec

---

## Executive Summary

Jujutsu's design contains profound parallels to annot's emerging "bookmarks" feature. Both systems solve the same core problem: **how to mark curiosity in ephemeral spaces without creating persistence burden**.

The analogy is not about copying jj's mechanics (branches, rebase, revsets) but about understanding the *philosophy* underneath them:

- **Immutability as a feature** — in jj, old commits remain intact when you rebase; in annot, past sessions become read-only landmarks
- **History as queryable artifact** — jj's operations log records "what happened"; annot bookmarks record "where I was curious"
- **Lightweight, composable pointers** — jj bookmarks don't require ceremony; annot bookmarks should be just as casual
- **Conflict-aware branching** — jj treats merges as first-class problems; annot can treat diverging exploration paths similarly
- **Working copy as just another commit** — jj treats uncommitted work as a node; annot treats each session as a discrete unit in a graph

---

## Part 1: The Parallel Problem Spaces

### jj's Challenge

In Git, history is brittle. Rebase old commits? You must force-push and risk losing others' work. Edit commit messages? You can't, without risking corruption. The working directory is a special, fragile state. Mistakes are costly.

jj solved this by:
- Making all commits immutable once recorded
- Treating the working copy as a normal commit (no special "uncommitted changes")
- Recording every operation so you can undo/inspect
- Allowing bookmarks to float free of commits (no auto-advance on new commits)

**Result**: You can explore fearlessly. Old states never disappear. You can always trace back.

### annot's Challenge

Annotations live in ephemeral sessions that vanish on Cmd-W. Feedback emerges, the session closes, the artifact lands. But what if the human spots something worth **revisiting later, in a fresh session**?

The tension:
- Sessions should be disposable (true to the manifesto)
- But curiosity shouldn't be lost (practical need)
- Bookmarks bridge this: they're **portals between sessions** without creating a "session persistence" layer

Bookmarks should let you say: *"I marked this spot. In a later session, I want to come back here with fresh eyes."*

---

## Part 2: The Five Core Parallels

### 1. **Immutability + Lineage**

**In jj:**
- Commits are immutable snapshots of your entire repo state
- When you rebase a commit, the old one stays; a new one appears
- Descendants automatically rebase onto new commits (with conflict handling)
- The graph grows but never loses information

**In annot:**
- Sessions are immutable snapshots of feedback on a document
- When you return to an old annotated file, that session is read-only
- A new session creates a new node; you can see the history graph
- The history grows but never loses where you were curious before

**Metaphor**: Just as jj's immutable history lets you reason about "what changed and why," annot's session history lets you reason about "what did I question and why."

**Design implication for bookmarks:**
- A bookmark pins a line in a past session
- Returning to that line in a new session doesn't "move" the old bookmark; both exist
- The output shows the lineage: "You questioned this in Session X; in Session Y, it's still there"

---

### 2. **Operations Log ≈ Session Lineage**

**In jj:**
```
$ jj op log
@  mfqcqsyd 2024-12-30 13:22 ⌛  jj rebase
│  Rebase 6 commits
○  lsxynybt 2024-12-30 13:20 ⌛  jj rebase
│  Rebase 5 commits
○  rqmsmtjm 2024-12-30 13:18 ⌛  jj commit
│  Record commit
○  zsxrpsmo 2024-12-30 13:10 ⌛  jj describe
   Describe commit
```

Each operation is a discrete event, timestamped, with a snapshot of the entire repo state. You can `jj op undo` to any previous state.

**In annot:**
```
SESSION 1 (12:00 PM):
  File: plan.md
  Annotations: 3
  Exit mode: "Ask questions"

BOOKMARK: Line 45, "auth approach feels heavyweight"

SESSION 2 (2:30 PM):
  File: plan.md (same file, new session)
  References: Bookmark from Session 1
  Annotations: 2 (one on the old bookmark)
  Exit mode: "Apply"
```

Each session is a discrete event, timestamped, with a snapshot of feedback. Bookmarks create waypoints in this history.

**Design implication:**
- Bookmarks should include metadata: who created it, when, what session, what was the intent
- The output should show lineage: "This line was questioned in Session 1, revisited in Session 2, resolved in Session 3"
- Future sessions can query the bookmark history: "Show me all unresolved bookmarks" (like revsets)

---

### 3. **Bookmarks as Lightweight, Non-Binding Pointers**

**In jj:**
- Git branches auto-advance when you commit; jj bookmarks don't
- A bookmark is just a name pointing to a commit; you move it manually
- Multiple bookmarks can point to the same commit
- Bookmarks don't require ceremony; they're as casual as naming

**In annot (proposed):**
```
User is annotating line 45 in Session 2.
Types: `/bookmark` in the annotation editor
Popup: "Name this bookmark?"
  [Remember for later] [Skip]

User enters: "auth-approach-rethink"

Later: In Session 3, user can query: "/bookmark auth-approach-rethink"
  → Shows: Session 1 where created, Session 2 where revisited, current context
```

Bookmarks don't "move" when you create new sessions. They're fixed to the past. But they're visible, queryable, and referable.

**Design implication:**
- Bookmarks live in user config (like jj bookmarks live in the `.jj/` state)
- They're lightweight: `(session_id, line, name, label)`
- They can be tagged with semantic markers: `#SECURITY`, `#UNCLEAR`, `#TODO`
- Future sessions can embed them via portal syntax: `[bookmark: auth-approach-rethink]`

---

### 4. **Conflict as First-Class**

**In jj:**
- Merges create conflicts as data, not failures
- You can see conflicted commits in the log
- You can annotate conflicts, rebase with conflicts pending
- Conflicts are *resolved*, not hidden

**In annot (extrapolated):**
- What if a file you bookmark gets substantially rewritten?
- The bookmark still points to old line numbers
- This is **information**: "You questioned this; it's now different"
- The UI should surface this: "This bookmark's line no longer exists in this form"

**Design implication:**
- Bookmarks can be "stale" or "orphaned"
- The session output should flag these: `STALE_BOOKMARKS: auth-approach-rethink (lines 40-45 in Session 1, file now has 120 lines)`
- Stale bookmarks are data for the AI: "This part was rethought; review what changed"

---

### 5. **The Working Copy as a Normal Node**

**In jj:**
- Your uncommitted changes are stored as a "working copy commit"
- It's just another commit; you can rebase it, squash it, merge it
- There's no special "dirty" state; it's a node in the graph

**In annot:**
- The current session is like a "working copy"
- The annotations you're creating now exist in a temporary space
- When you Cmd-W, they're committed to history
- Future sessions are clean (read-only) views of past states
- But you can reference past bookmarks from the current working copy

**Design implication:**
- The current session should display active bookmarks from past sessions
- You should be able to annotate on old bookmarks: "Still unclear" or "Fixed now"
- The output should show this: "Bookmark `auth-approach` from Session 1: revisited, clarified"

---

## Part 3: Concrete Bookmark Behavior (Sketch)

### Creating a Bookmark

```
User selects lines 42-50 in a session.
Types `/` → tag menu
Includes bookmark option: "📌 Mark for later"

Popup dialog:
  Name: [________________]  (required)
  Label: [________________] (optional, like "UNCLEAR", "RETHINK", "SECURITY")

User enters:
  Name: "jwt-complexity"
  Label: RETHINK

Result: Bookmark stored in session metadata
```

### Displaying Bookmarks in a New Session

```
User opens the same file in a new session.
Sidebar or on-line indicator shows: "📌 You bookmarked this before"

Hovering reveals:
  jwt-complexity (Session 2024-12-30 @ 2:15 PM)
  "JWT vs OAuth trade-off feels unresolved"

User can:
  - Click to expand and see the old annotation
  - Add a new annotation referencing the bookmark
  - Mark it "Resolved" or "No longer relevant"
```

### Bookmark Query (Revsets-Like)

```
User types `g` (open session context)
Enters query: "show:bookmarks where label=RETHINK and status=open"

Result: Sidebar shows all open RETHINK bookmarks
  1. auth-approach (Session 2)
  2. jwt-complexity (Session 1)
  3. data-model (Session 2)
```

### Output Format

When a session touches bookmarks:

```
SESSION:
  File: plan.md
  Exit: Apply
  Bookmarks:
    REFERENCED:
      - jwt-complexity (created: Session 1, status: revisited)
    CREATED:
      - data-model-rework (label: UNCLEAR)

---

plan.md:42-50:
    42 | function authenticateUser() {
    43 |   // JWT implementation
     > [# RETHINK] Consider OAuth for enterprise
     > Bookmark: jwt-complexity

---

plan.md:80-95:
    80 | const dataSchema = {
     > [# BOOKMARK] data-model-rework
     > This needs deeper thinking about query patterns
```

---

## Part 4: What annot Can Learn from jj

### Pattern 1: Embrace Immutability as Permission

jj's insight: *Making history immutable gives you permission to experiment.*

For annot:
- Once a session closes, make it immutable
- This isn't a limitation; it's a **feature** that lets you bookmark fearlessly
- You know the bookmark points to a stable snapshot

### Pattern 2: Timestamp Everything

jj records `2024-12-30 13:22`.

For annot:
- Every bookmark should have a timestamp
- Every reference to a bookmark should record when it was revisited
- This creates a narrative: "You asked about this on Dec 30. Revisited it on Jan 2. Finally resolved on Jan 5."

### Pattern 3: Make History Queryable

jj's revsets let you ask: "Show me commits by alice that touched src/auth before the rebase."

For annot:
- Bookmarks should be queryable: "Show me all SECURITY bookmarks created this week"
- Sessions should be queryable: "Sessions where I used the QUESTION tag"
- Export should reflect these queries: "Summary of all UNCLEAR bookmarks"

### Pattern 4: Conflicts Are Data

jj treats merge conflicts as information to reason about.

For annot:
- If a file changes significantly, bookmarks become "stale"
- Stale bookmarks are **information**: something you cared about was rethought
- The AI should see this: "These bookmarks from Session 1 are now orphaned; the file grew from 50 to 120 lines"

### Pattern 5: The Graph Tells a Story

jj's operation log is a narrative of how you arrived here.

For annot:
- A file with multiple sessions + bookmarks + revisions tells a story
- Session 1: "Let me understand this"
- Bookmark: "This part is unclear"
- Session 2: "Revisiting with new context"
- Resolution: "Now it makes sense"
- The output should preserve and surface this narrative

---

## Part 5: Design Tensions & Trade-Offs

### Tension 1: Lightweight vs. Semantic

**jj's approach**: Bookmarks are just names. Semantics come from how you use them.

**annot could go either way**:
- Lightweight: bookmark = name + line + timestamp
- Semantic: bookmark = name + label + reason + intent + resolved_status

**Lean**: Start lightweight. Add semantic layers (tags, status) through tagging infrastructure that already exists.

### Tension 2: Mutable vs. Immutable Bookmarks

**Question**: If I revisit a bookmark in a new session and add new context, should I mutate the old bookmark or create a new annotation on it?

**jj's approach**: Immutable bookmarks; new commits when you change things.

**Lean for annot**: Immutable bookmarks. New annotations in new sessions reference old bookmarks. The lineage is visible in the output.

### Tension 3: Storage Location

**jj**: Bookmarks live in `.jj/state.jj` (local repo state)

**annot options**:
- Option A: Store in user config (global, portable)
- Option B: Store per-file (next to the file, like a sidecar `.annotations.json`)
- Option C: Store in project root (like `.annot/bookmarks.json`)

**Lean**: Option A (global config). Bookmarks are user intent, not file metadata. They travel with the user.

### Tension 4: Bidirectional Reference

**Question**: Can a bookmark reference other bookmarks? Can it embed a portal?

**jj**: No; commits are independent nodes. Relationships are implicit in the graph structure.

**annot**: Could allow bookmark → portal references:
```
Bookmark "jwt-rethink" could say:
  "See also: [auth approach decision](plan.md#bookmark:auth-approach)"
```

**Lean**: Not in v1. Keep bookmarks simple. Link-following comes later.

---

## Part 6: Implementation Sketch

### Data Structures

```rust
#[derive(Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,                    // UUID
    pub name: String,                  // "jwt-complexity"
    pub label: Option<String>,         // "RETHINK" | "UNCLEAR" | custom
    pub created_session: SessionId,
    pub created_at: DateTime,
    pub file_path: PathBuf,
    pub line_range: (u32, u32),        // start, end (1-indexed)
    pub reason: Option<String>,        // "Feels heavyweight"
    pub tags: Vec<String>,             // ["SECURITY", "PERFORMANCE"]
    pub status: BookmarkStatus,        // Open | Resolved | Stale
}

#[derive(Serialize, Deserialize)]
pub enum BookmarkStatus {
    Open,
    Resolved { resolved_session: SessionId, resolved_at: DateTime },
    Stale { reason: String },          // "File changed substantially"
}

#[derive(Serialize, Deserialize)]
pub struct BookmarkReference {
    pub bookmark_id: String,
    pub session_id: SessionId,
    pub action: RefAction,             // Created | Revisited | Annotated | Resolved
    pub annotation: Option<String>,    // "Still applies" | "Fixed now"
}

pub enum RefAction {
    Created,
    Revisited,
    Annotated(String),  // The annotation text
    Resolved,
    Dismissed,
}
```

### Storage

```
~/.config/annot/  (or platform equivalent)
├── tags.json
├── exit-modes.{ext}.json
├── bookmarks.json              // NEW
│   [
│     {
│       "id": "bm_abc123",
│       "name": "jwt-complexity",
│       "label": "RETHINK",
│       "created_session": "sess_20241230_001",
│       "created_at": "2024-12-30T14:22:00Z",
│       "file_path": "/Users/user/project/plan.md",
│       "line_range": [42, 50],
│       "status": "Open"
│     }
│   ]
└── bookmark-history.json       // NEW (references per session)
    [
      {
        "session_id": "sess_20241231_001",
        "bookmark_id": "bm_abc123",
        "action": "Revisited",
        "annotation": "Still relevant, discussing with team"
      }
    ]
```

### UI Components

**In-session bookmark indicator** (on lines):
```
  42 | function auth() {
     > 📌 You bookmarked: "jwt-complexity"
```

**Bookmark sidebar** (when `g` is pressed or sidebar opened):
```
BOOKMARKS (3)
  📌 jwt-complexity (RETHINK)
     Created: Dec 30, 2:15 PM
     Status: Open, revisited 1x

  📌 data-model (UNCLEAR)
     Created: Dec 30, 1:45 PM
     Status: Open

  📌 auth-flow (SECURITY)
     Created: Dec 29, 10:00 AM
     Status: Resolved (Dec 30)
```

**Annotation on a bookmark**:
```
User selects lines 42-50 (which has bookmark "jwt-complexity")
Types: "Still feels too complex for enterprise case"
Presses Tab to open exit modes
The annotation is tagged with: "References: jwt-complexity"
```

### IPC Commands

```typescript
// MCP / Tauri IPC extensions
{
  command: "upsert_bookmark",
  params: {
    name: String,
    label?: String,
    reason?: String,
    line_range: [u32, u32],
    file_path: String,
  }
}

{
  command: "list_bookmarks",
  params: {
    file_path?: String,
    status?: "open" | "resolved" | "stale" | "all",
  }
}

{
  command: "reference_bookmark",
  params: {
    bookmark_id: String,
    action: "revisited" | "annotated" | "resolved",
    annotation?: String,
  }
}

{
  command: "query_bookmarks",
  params: {
    query: String,  // "label=RETHINK and status=open"
  }
}
```

### Output Format

When session includes bookmark references:

```
SESSION:
  File: plan.md
  Exit: Apply (Apply the suggested changes)

BOOKMARKS:
  CREATED:
    - data-model-patterns [UNCLEAR]
      "Needs thinking about spatial indexes"

  REFERENCED:
    - jwt-complexity [RETHINK]
      Status: open, created 2 sessions ago
      Action in this session: Annotated with clarification
      > Updated understanding: "Actually fits well for our use case"
      > Remaining question: Token refresh strategy still TBD

---

plan.md:42-50:
    42 | function auth() {
    43 |   // JWT approach
     > [# RETHINK] OK on further review, but what about refresh?
     > Bookmark: jwt-complexity (references past session)

plan.md:80-95:
    80 | const schema = {
     > [# BOOKMARK] data-model-patterns
     > Need to think about pagination and indexing strategies
```

---

## Part 7: Future Directions (Not v1)

Once bookmarks are stable:

### Revset-Like Queries
```
g → opens bookmark query panel
"label=SECURITY and status=open"
"created_before:7days and not resolved"
"referenced_in:session_id"
```

### Cross-Session Narratives
```
When exporting, show the lineage:
"This line was questioned in 3 sessions:
  Session 1 (2024-12-30): 'Too complex?'
  Session 2 (2025-01-01): 'Clarified with team'
  Session 3 (2025-01-05): 'Resolved: switching approach'
"
```

### Bookmark Embeddings in Content
```
When reviewing a new file, if there's a bookmark about a related concept:
  [! reference]
  Bookmark from previous session:
  "jwt-complexity — RETHINK"
  Context: "Does this auth approach have the same problem we noted before?"
```

### Bookmark Merging
```
Two files get refactored into one.
Bookmarks from both files are now on the merged content.
UI shows: "2 bookmarks from previous files now apply here"
```

---

## Part 8: Connection to annot's Manifesto

**From the manifesto:**
> "Portals embed live code in context"

Bookmarks are the inverse:
- Portals let you embed **external context** (other files) in the current session
- Bookmarks let you embed **past context** (old curiosity) in new sessions

Both are about **reducing cognitive friction**:
- Portals: "Instead of context-switching to understand this reference, see it here"
- Bookmarks: "Instead of forgetting your earlier curiosity, see it here"

**From the manifesto:**
> "annot is not a workspace. There is no session state to manage."

Bookmarks respect this:
- Sessions remain ephemeral (Cmd-W closes them)
- Bookmarks live in config, not in sessions
- You can't edit old sessions; you can only reference them
- Zero session-management overhead

**From the manifesto:**
> "Ephemeral identity (no persistence, zero exit cost)"

Bookmarks embody this:
- They're lightweight pointers, not full session save-states
- Creating a bookmark costs nothing (one line added to JSON)
- You can ignore bookmarks entirely; the session works fine without them
- Bookmarks are **opt-in curvature**, not structural

---

## Part 9: Risks & Mitigations

### Risk 1: Bookmarks Become a Feature Graveyard

**Problem**: Users create bookmarks but never revisit them. They accumulate, creating noise.

**Mitigation (from jj)**:
- jj bookmarks can be deleted; so can annot's
- Include `jj-style` bookmark browser/manager
- Auto-archive bookmarks after 30 days without action
- Output summary: "5 open bookmarks; 12 resolved this session"

### Risk 2: Bookmark Metadata Bloat

**Problem**: You add `label`, `reason`, `tags`, `status`, `references`... soon bookmarks are heavyweight sessions themselves.

**Mitigation**:
- Start minimal: name + line + timestamp
- Add layers incrementally (tags exist; use the existing tag system)
- The "reason" is captured in annotations that reference the bookmark

### Risk 3: Stale Bookmarks Create False Guidance

**Problem**: File changes; bookmark still points to line 42. But line 42 is now inside a function that got renamed. The bookmark is misleading.

**Mitigation (from jj's conflict model)**:
- Detect stale bookmarks: if file grew/shrank significantly, mark as Stale
- Output should surface staleness: "This bookmark is orphaned; file now 120 lines instead of 50"
- AI can reason about it: "Why was this marked? What changed?"

### Risk 4: Cross-File Bookmark Chaos

**Problem**: User has 50 files open in various projects. Bookmarks are scattered. How do you query "show me all SECURITY bookmarks"?

**Mitigation**:
- Global bookmark storage is intentional (like jj)
- Queries include file path / project context
- Bookmark manager (future) lets you browse by project/tag

---

## Part 10: Why This Matters (Philosophical)

jj solved a deep problem in Git: **how to make destructive operations (rebasing, editing history) safe and reversible**.

annot is solving an analogous problem: **how to make ephemeral sessions safe anchors for curiosity without creating persistence overhead**.

Bookmarks are the answer.

Just as jj's immutable history gives you permission to rebase fearlessly, annot's immutable sessions with bookmarks give you permission to explore fearlessly. You know where you were curious before. You can return without fear of losing thread.

This is the **thinking topology** you mentioned: sessions are nodes, bookmarks are edges. The graph shows not just what you built, but how you thought about building it.

---

## Questions for the Design

1. **Bookmark creation UX**: `/bookmark` command in the annotation editor, or a separate bookmark button on line ranges?

2. **Auto-staleness detection**: Should we compute "staleness" heuristically (file grew > 20%), or require explicit user marking?

3. **Bookmark output**: Should resolved/dismissed bookmarks appear in the export, or just open ones?

4. **Namespace**: Global bookmarks, or bookmarks per-file, or per-project?

5. **Bookmark queries in output**: Should the final output include a summary of which bookmarks were created/referenced/resolved?

6. **Integration with tags**: Should bookmarks use the existing tag system (e.g., `bookmark:jwt-complexity` as a tag), or be separate?

---

## Summary: The Analogy in Three Lines

1. **jj makes rebasing safe by keeping old commits immutable and recording all operations.**
   → **annot makes exploration safe by keeping old sessions immutable and bookmarking curiosity.**

2. **jj bookmarks are lightweight pointers that don't auto-advance.**
   → **annot bookmarks are lightweight pointers that don't move across sessions.**

3. **jj's operation log is a queryable history of what you did.**
   → **annot's bookmark lineage is a queryable history of what you wondered about.**

---

## Sources & References

- [jj-vcs/jj GitHub](https://github.com/jj-vcs/jj)
- [Understanding Jujutsu bookmarks — Tech Notes](https://neugierig.org/software/blog/2025/08/jj-bookmarks.html)
- [Jujutsu for Everyone — deleting commits and bookmarks](https://jj-for-everyone.github.io/abandon.html)
- [Jujutsu VCS Tutorial](https://gist.github.com/christianromney/27fd1fca9e5f24ef24d9ed6c9eddda50)
- [annot Manifesto](/Users/denolehov/_p/rust/annot/docs/manifesto.md)
- [Portals Spec](/Users/denolehov/_p/rust/annot/docs/portals-spec.md)
- [Invisible Enrichment Concept](/Users/denolehov/_p/rust/annot/docs/concepts/invisible-enrichment.md)
- [Steering vs Selection Exploration](/Users/denolehov/_p/rust/annot/docs/steering-exploration.md)
