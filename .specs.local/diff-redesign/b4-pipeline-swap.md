---
id: B4
kind: refactor
wave: 1
depends_on: [B1, B2, B3]
status: fogged
---

# Primer: B4 — Git pipeline swap (strangler node)

> Fogged. Clear before starting: B1/B2/B3's real APIs replace the sketches here.

**Goal:** Git mode (`git_diff_args`) stops parsing patch text. New pipeline: B2 enumerates files/oids → B1 fetches both sides' full text → B3 computes hunks → **render into the existing flat `Line` stream** (`LineOrigin::Diff` + `DiffSemantics` rows, `DiffMetadata` populated as today). The wire contract does not change; if B4 lands and nobody notices, it worked.

**Why after B1+B2+B3:** it is pure composition of the three; doing any of their work inline here fattens the riskiest kind of node (a producer swap).

**Settled constraints:**
- `unidiff`/`parse_diff` (`src-tauri/src/diff.rs:87`) survives, but only reachable from raw `diff_content` mode (and CLI stdin patches).
- Entry point today: `run_diff_session` (`src-tauri/src/mcp/mod.rs:164`) and the CLI diff path in `lib.rs` — both route through `ContentModel::from_diff`; this node forks git-args mode to `ContentModel::from_git(...)`.
- Full texts + `FileSource` stay alive in session state after load — S3 (unfold) and re-diff need them; don't drop after rendering.
- Syntax highlighting: current pipeline highlights via `highlight.rs`/`syntect` — new pipeline must produce equivalent `html` per line.

**Verification bar:** side-by-side session outputs (old parser vs new pipeline) on real repos agree on file list, hunk boundaries (modulo settled cosmetic divergence), line origins. Existing insta output snapshots stay green.

**Risk to plan around:** hunk-boundary cosmetic divergence from `git diff` (different algorithm defaults) — accepted at design time, but eyeball a corpus before trusting it.
