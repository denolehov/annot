---
id: B1
kind: refactor
wave: 0
depends_on: []
status: ready
---

# [Spec]: B1 — FileSource trait (content-source seam)

## Requirements
- **Problem:** Annot only ever holds patch text; nothing can answer "give me the full file at side X", which unfold (S3) and substrate (B4) require.
- **Beneficiary:** B4 (loads both sides), S3 (slices gap lines), parked jj tier (this trait is its front door).
- **Done when:** `GitShellSource` returns full text for old/new sides of files in a real repo (unit-tested against a fixture repo); `RawPatchSource` returns `None`.

## Entities

```rust
pub enum Side { Old, New }                       // shared with A1's anchor types

pub trait FileSource: Send + Sync {
    /// Full text of the file at `path` on `side`. Ok(None) = unavailable
    /// (raw patch mode, binary, or side doesn't exist e.g. Old of an added file).
    fn full_text(&self, path: &str, side: Side) -> Result<Option<Arc<str>>, AnnotError>;
}

pub struct GitShellSource {
    repo_root: PathBuf,
    /// (path, side) -> oid, from B2's enumerator. Zero oid => working tree (fs read).
    oids: HashMap<(String, Side), Option<String>>,
    cache: Mutex<HashMap<String /* oid or path */, Arc<str>>>,
}

pub struct RawPatchSource;                        // full_text => Ok(None), always
```

## Approach

**Keystone:** fetch *whole files*, cached, never gap slices — every later
unfold is then a local slice (Zed and hunk both converged here). Rejected
alternative: per-gap `git show file:40-60`-style queries — N round trips,
no reuse, and git can't even express line ranges for blobs.

- Blob retrieval: single `git cat-file --batch` child process, lazily spawned,
  fed `<oid>\n` lines, response header `<oid> <type> <size>` then payload.
  One process for the whole session (Zed's pattern), not one `git show` per file.
- Working-tree side (zero oid): `std::fs::read_to_string(repo_root.join(path))`.
- Size cap ~1 MB per file (hunk's guard) → treat oversize as `Ok(None)`
  (unfold affordance simply won't render).
- `Ok(None)` is the **capability signal**: UI derives "can unfold?" from it —
  no separate mode flag to keep in sync.

**Seams:**
- Parked JjLib tier = third impl backed by `materialize_tree_value` (returns
  full file contents — plugs in directly).
- `Arc<str>` return → B4 holds the same allocation for diffing; no copies.

## Structure
- New: `src-tauri/src/source.rs` (trait + both impls + tests)
- `src-tauri/src/lib.rs` — module registration
- Consumed later by B4 (pipeline) and S3 (unfold IPC); no call sites yet — this node is deliberately standalone.

## Operations
1. `Side` + trait + `RawPatchSource` (trivial) + unit test.
2. `GitShellSource::new(repo_root, oids)`; batch process management (spawn on first use, restart on death).
3. Fixture-repo tests: added/deleted/modified/renamed file × old/new side; oversize file → `None`; binary → `None`.

## Norms
- Errors via `AnnotError` (`src-tauri/src/error.rs`), thiserror.
- No tokio needed — synchronous child-process I/O is fine here (commands calling it are already async or on the blocking pool).

## Safeguards
- Never panic on missing oid/path — `Ok(None)`.
- Batch process death mid-session must not poison the session: respawn once, then degrade to `None`.

## Scope
- In: trait, two impls, fixture tests.
- Out: enumeration of oids (B2 provides), any UI, jj impl (parked).
