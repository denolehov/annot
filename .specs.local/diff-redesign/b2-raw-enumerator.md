---
id: B2
kind: refactor
wave: 0
depends_on: []
status: ready
---

# [Spec]: B2 — `git diff --raw` enumerator

## Requirements
- **Problem:** Substrate (b) needs, per changed file, the two blob identities — but `git_diff_args` are arbitrary user/agent strings (`review_diff` just appends them: `src-tauri/src/mcp/mod.rs:171-191`). Annot must not parse revision semantics itself.
- **Beneficiary:** B4 (file list + oids feed B1), and the file tree gets rename/status data better than patch parsing gives.
- **Done when:** `enumerate(args)` returns correct entries for modified/added/deleted/renamed/working-tree cases against a fixture repo.

## Entities

```rust
pub enum FileStatus { Modified, Added, Deleted, Renamed { similarity: u8 }, Copied, TypeChanged }

pub struct FileEntry {
    pub status: FileStatus,
    pub old_path: Option<String>,   // None for Added
    pub new_path: Option<String>,   // None for Deleted
    pub old_oid: Option<String>,    // None = nonexistent side; Some(ZERO) normalized to WorkingTree below
    pub new_oid: Option<BlobRef>,
}
pub enum BlobRef { Oid(String), WorkingTree }     // zero oid => WorkingTree
```

## Approach

**Keystone:** run `git diff --raw -z <same args the caller passed>` and let git
resolve every revision/pathspec question; annot only parses the stable `--raw`
record format. Rejected alternative: interpreting `<args>` (revs, ranges,
`--staged`, pathspecs) ourselves — an open-ended reimplementation of git CLI
semantics with permanent drift risk.

Record format (NUL-separated with `-z`):
`:<old_mode> <new_mode> <old_oid> <new_oid> <status>[score]\0<path>[\0<path2>]\0`
— R/C carry two paths; abbreviated oids avoided via `--no-abbrev`.

Flow: `git diff --raw -z --no-abbrev <args>` → parse → `Vec<FileEntry>` →
(B4) build B1's oid map + drive per-file diffing.

**Seams:**
- `FileStatus::Renamed` → file tree shows `old → new` (S1 refit at C1).
- Same enumerator later serves "changed since op X" listings in the parked jj tier (different producer, same `FileEntry`).

## Structure
- New: `src-tauri/src/vcs.rs` (or fold into `source.rs`'s sibling — implementer's call, keep it out of `diff.rs` which is the legacy patch parser)
- Callers: none yet (B4 wires it). Standalone + tests, like B1.

## Operations
1. Runner: `Command::new("git").args(["diff","--raw","-z","--no-abbrev"]).args(user_args)` with cwd = session cwd; capture stderr for error surfacing (mirror `mcp/mod.rs:180-183`).
2. `-z` record parser (NUL split; R/C two-path handling; score suffix on status letter).
3. Zero-oid normalization → `BlobRef::WorkingTree`.
4. Fixture tests: M/A/D/R cases; `--staged`; rev-range (`HEAD~1..HEAD`); pathspec filter; empty diff.

## Norms
- Same subprocess discipline as existing `run_diff_session` (error string from stderr).

## Safeguards
- Unparseable record → error, not silent skip (a missing file in review is worse than a failed session).
- Must handle paths with spaces/unicode (that's what `-z` is for — test it).

## Scope
- In: runner, parser, fixture tests.
- Out: fetching content (B1), diffing (B3), wiring (B4), submodule/binary special-casing beyond "mark and pass through".
