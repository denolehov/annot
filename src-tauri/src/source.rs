//! Content-source seam: `side -> full file text | None`.
//!
//! Fetches *whole files*, cached, never gap slices — every later unfold is a
//! local slice. Consumed by the diff pipeline (loads both sides) and unfold
//! IPC (slices gap lines).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::AnnotError;
use crate::vcs::BlobRef;

/// Which side of a diff content belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Old,
    New,
}

/// Files larger than this are treated as unavailable — the unfold affordance
/// simply won't render for them.
pub(crate) const MAX_FILE_SIZE: u64 = 1024 * 1024;

pub trait FileSource: Send + Sync {
    /// Full text of the file at `path` on `side`.
    ///
    /// `Ok(None)` means unavailable: raw patch mode, binary or oversize
    /// content, or the side doesn't exist (e.g. `Old` of an added file).
    /// `Ok(None)` is the capability signal — the UI derives "can unfold?"
    /// from it. Current implementations degrade to `Ok(None)` rather than
    /// erroring; the `Result` exists for future sources whose failures
    /// callers should surface.
    fn full_text(&self, path: &str, side: Side) -> Result<Option<Arc<str>>, AnnotError>;

    /// Whether the text served for `path` is a *materialized merge conflict* —
    /// marker text standing in for several sides that were never merged.
    ///
    /// Only jj can answer yes: there a conflict is a committed object a rebase
    /// carries around, so the text is real content. Git's tier errors on
    /// unmerged paths instead, and raw patches have no such notion — hence the
    /// default. The frontend uses this to decide whether marker lines in a
    /// file are structure to style or just prose that happens to contain
    /// `<<<<<<<`.
    fn is_conflicted(&self, _path: &str) -> bool {
        false
    }
}

/// Raw patch mode: only the patch text exists, full files are never available.
pub struct RawPatchSource;

impl FileSource for RawPatchSource {
    fn full_text(&self, _path: &str, _side: Side) -> Result<Option<Arc<str>>, AnnotError> {
        Ok(None)
    }
}

/// Serves full file texts from a git repo via gix: blobs from the object
/// database, working-tree content from raw fs reads.
pub struct GixSource {
    /// `gix::Repository` is `Send` but `!Sync`; `FileSource` needs `Sync`.
    /// A held repository keeps its pack caches warm across lookups.
    repo: Mutex<gix::Repository>,
    workdir: Option<PathBuf>,
    /// (path, side) -> blob reference. Entry absent = side nonexistent
    /// (e.g. `Old` of an added file). Built by the pipeline from
    /// `FileEntry.{old_oid,new_oid}`.
    oids: HashMap<(String, Side), BlobRef>,
    /// Blobs keyed by oid (content-addressed, never stale); working-tree
    /// files keyed by `"wt:{path}"` — stale if the file changes mid-session,
    /// which is fine: the session sees one consistent snapshot.
    cache: Mutex<HashMap<String, Arc<str>>>,
}

impl GixSource {
    pub fn new(repo: gix::Repository, oids: HashMap<(String, Side), BlobRef>) -> Self {
        Self {
            workdir: repo.workdir().map(|p| p.to_path_buf()),
            repo: Mutex::new(repo),
            oids,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Raw bytes, deliberately unfiltered (no smudge/CRLF normalization):
    /// display should show the real file, and the diff engine handles CRLF.
    fn read_working_tree(&self, path: &str) -> Option<Vec<u8>> {
        let full = self.workdir.as_ref()?.join(path);
        let meta = std::fs::metadata(&full).ok()?;
        if meta.len() > MAX_FILE_SIZE {
            return None;
        }
        std::fs::read(&full).ok()
    }

    fn read_blob(&self, hex: &str) -> Option<Vec<u8>> {
        let id = gix::ObjectId::from_hex(hex.as_bytes()).ok()?;
        let repo = self.repo.lock();
        let header = repo.find_header(id).ok()?;
        if header.kind() != gix::object::Kind::Blob || header.size() > MAX_FILE_SIZE {
            return None;
        }
        let object = repo.find_object(id).ok()?;
        Some(object.detach().data)
    }
}

/// Binary gate shared by every tier: a NUL byte means binary (NUL is valid
/// UTF-8, so this isn't subsumed by the UTF-8 check); invalid UTF-8 also
/// yields `None`.
pub(crate) fn bytes_to_text(bytes: Vec<u8>) -> Option<Arc<str>> {
    if bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes).ok().map(Arc::from)
}

/// Serves full file texts from a jj repo.
///
/// Unlike `GixSource` this is eager: the enumerator materialized every side
/// already, because a conflicted side has no blob to fetch later — it's a
/// merge of several, resolved into marker text at diff time. Since the
/// pipeline reads both sides of every file anyway, eager costs nothing extra.
///
/// A present key with a `None` value is the capability signal: the side exists
/// but has no reviewable text (binary, oversize, non-UTF-8, symlink,
/// submodule).
pub struct JjSource {
    texts: HashMap<(String, Side), Option<Arc<str>>>,
    /// Paths (either side) whose text is materialized conflict markers.
    conflicted: HashSet<String>,
}

impl JjSource {
    pub fn new(
        texts: HashMap<(String, Side), Option<Arc<str>>>,
        conflicted: HashSet<String>,
    ) -> Self {
        Self { texts, conflicted }
    }
}

impl FileSource for JjSource {
    fn full_text(&self, path: &str, side: Side) -> Result<Option<Arc<str>>, AnnotError> {
        Ok(self.texts.get(&(path.to_string(), side)).cloned().flatten())
    }

    fn is_conflicted(&self, path: &str) -> bool {
        self.conflicted.contains(path)
    }
}

impl FileSource for GixSource {
    fn full_text(&self, path: &str, side: Side) -> Result<Option<Arc<str>>, AnnotError> {
        let Some(blob_ref) = self.oids.get(&(path.to_string(), side)) else {
            return Ok(None);
        };
        // Never hold `cache` and `repo` at once: check cache, unlock, fetch,
        // re-lock to insert.
        let (cache_key, bytes) = match blob_ref {
            BlobRef::WorkingTree => {
                let key = format!("wt:{path}");
                if let Some(hit) = self.cache.lock().get(&key) {
                    return Ok(Some(hit.clone()));
                }
                (key, self.read_working_tree(path))
            }
            BlobRef::Oid(hex) => {
                if let Some(hit) = self.cache.lock().get(hex.as_str()) {
                    return Ok(Some(hit.clone()));
                }
                (hex.clone(), self.read_blob(hex))
            }
        };
        let Some(text) = bytes.and_then(bytes_to_text) else {
            return Ok(None);
        };
        self.cache.lock().insert(cache_key, text.clone());
        Ok(Some(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, hash_object};
    use proptest::prelude::*;
    use std::path::Path;

    /// Two commits + one uncommitted file:
    /// - commit 1: modified.txt v1, deleted.txt, old_name.txt, big.txt (>1 MB), bin.dat
    /// - commit 2: modified.txt v2, added.txt, rm deleted.txt, mv old_name -> new_name
    /// - working tree: wt.txt (uncommitted)
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        std::fs::write(p.join("modified.txt"), "one\ntwo\n").unwrap();
        std::fs::write(p.join("deleted.txt"), "doomed\n").unwrap();
        std::fs::write(p.join("old_name.txt"), "renamed content\n").unwrap();
        std::fs::write(p.join("big.txt"), "x".repeat(1_100_000)).unwrap();
        std::fs::write(p.join("bin.dat"), b"\x00\x01binary").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "one"]);
        std::fs::write(p.join("modified.txt"), "one\nTWO\n").unwrap();
        std::fs::write(p.join("added.txt"), "fresh\n").unwrap();
        git(p, &["rm", "-q", "deleted.txt"]);
        git(p, &["mv", "old_name.txt", "new_name.txt"]);
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "two"]);
        std::fs::write(p.join("wt.txt"), "working tree\n").unwrap();
        dir
    }

    fn source(dir: &tempfile::TempDir) -> GixSource {
        let p = dir.path();
        let oid = |rev_path: &str| BlobRef::Oid(git(p, &["rev-parse", rev_path]));
        let mut oids = HashMap::new();
        oids.insert(
            ("modified.txt".into(), Side::Old),
            oid("HEAD~1:modified.txt"),
        );
        oids.insert(("modified.txt".into(), Side::New), oid("HEAD:modified.txt"));
        oids.insert(("deleted.txt".into(), Side::Old), oid("HEAD~1:deleted.txt"));
        oids.insert(("added.txt".into(), Side::New), oid("HEAD:added.txt"));
        oids.insert(
            ("old_name.txt".into(), Side::Old),
            oid("HEAD~1:old_name.txt"),
        );
        oids.insert(("new_name.txt".into(), Side::New), oid("HEAD:new_name.txt"));
        oids.insert(("big.txt".into(), Side::Old), oid("HEAD~1:big.txt"));
        oids.insert(("bin.dat".into(), Side::Old), oid("HEAD~1:bin.dat"));
        oids.insert(("wt.txt".into(), Side::New), BlobRef::WorkingTree);
        oids.insert(
            ("bogus.txt".into(), Side::New),
            BlobRef::Oid("deadbeef".repeat(5)),
        );
        oids.insert(
            ("garbage-hex.txt".into(), Side::New),
            BlobRef::Oid("not-a-hex-oid".into()),
        );
        GixSource::new(gix::discover(p).unwrap(), oids)
    }

    fn text(src: &impl FileSource, path: &str, side: Side) -> Option<Arc<str>> {
        src.full_text(path, side).unwrap()
    }

    #[test]
    fn raw_patch_source_is_always_none() {
        assert!(text(&RawPatchSource, "anything.txt", Side::Old).is_none());
        assert!(text(&RawPatchSource, "anything.txt", Side::New).is_none());
    }

    #[test]
    fn added_deleted_modified_renamed_matrix() {
        let dir = fixture();
        let src = source(&dir);

        assert!(text(&src, "added.txt", Side::Old).is_none());
        assert_eq!(
            text(&src, "added.txt", Side::New).as_deref(),
            Some("fresh\n")
        );

        assert_eq!(
            text(&src, "deleted.txt", Side::Old).as_deref(),
            Some("doomed\n")
        );
        assert!(text(&src, "deleted.txt", Side::New).is_none());

        assert_eq!(
            text(&src, "modified.txt", Side::Old).as_deref(),
            Some("one\ntwo\n")
        );
        assert_eq!(
            text(&src, "modified.txt", Side::New).as_deref(),
            Some("one\nTWO\n")
        );

        assert_eq!(
            text(&src, "old_name.txt", Side::Old).as_deref(),
            Some("renamed content\n")
        );
        assert_eq!(
            text(&src, "new_name.txt", Side::New).as_deref(),
            Some("renamed content\n")
        );
    }

    #[test]
    fn oversize_is_none() {
        let dir = fixture();
        let src = source(&dir);
        assert!(text(&src, "big.txt", Side::Old).is_none());
        // a normal lookup afterwards still works
        assert_eq!(
            text(&src, "modified.txt", Side::Old).as_deref(),
            Some("one\ntwo\n")
        );
    }

    #[test]
    fn size_cap_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        let at_cap = "x".repeat(MAX_FILE_SIZE as usize);
        let mut oids = HashMap::new();
        oids.insert(
            ("at-cap.txt".into(), Side::New),
            BlobRef::Oid(hash_object(p, at_cap.as_bytes())),
        );
        oids.insert(
            ("over-cap.txt".into(), Side::New),
            BlobRef::Oid(hash_object(
                p,
                "x".repeat(MAX_FILE_SIZE as usize + 1).as_bytes(),
            )),
        );
        let src = GixSource::new(gix::discover(p).unwrap(), oids);
        assert!(text(&src, "over-cap.txt", Side::New).is_none());
        assert_eq!(
            text(&src, "at-cap.txt", Side::New).as_deref(),
            Some(at_cap.as_str())
        );
    }

    #[test]
    fn empty_file_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        let mut oids = HashMap::new();
        oids.insert(
            ("empty.txt".into(), Side::New),
            BlobRef::Oid(hash_object(p, b"")),
        );
        let src = GixSource::new(gix::discover(p).unwrap(), oids);
        assert_eq!(text(&src, "empty.txt", Side::New).as_deref(), Some(""));
    }

    #[test]
    fn binary_is_none() {
        let dir = fixture();
        let src = source(&dir);
        assert!(text(&src, "bin.dat", Side::Old).is_none());
    }

    #[test]
    fn working_tree_reads() {
        let dir = fixture();
        let src = source(&dir);
        assert_eq!(
            text(&src, "wt.txt", Side::New).as_deref(),
            Some("working tree\n")
        );
    }

    #[test]
    fn missing_oid_and_unmapped_path_are_none() {
        let dir = fixture();
        let src = source(&dir);
        assert!(text(&src, "bogus.txt", Side::New).is_none()); // oid not in odb
        assert!(text(&src, "garbage-hex.txt", Side::New).is_none()); // unparseable oid
        assert!(text(&src, "never-mapped.txt", Side::New).is_none()); // no map entry
        assert!(text(&src, "deleted.txt", Side::New).is_none()); // absent side
    }

    #[test]
    fn cache_returns_the_same_allocation() {
        let dir = fixture();
        let src = source(&dir);
        let a = text(&src, "modified.txt", Side::New).unwrap();
        let b = text(&src, "modified.txt", Side::New).unwrap();
        assert!(Arc::ptr_eq(&a, &b));

        let wa = text(&src, "wt.txt", Side::New).unwrap();
        let wb = text(&src, "wt.txt", Side::New).unwrap();
        assert!(Arc::ptr_eq(&wa, &wb));
    }

    fn blob_content() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // arbitrary bytes: binary, invalid UTF-8, NULs, no trailing newline
            proptest::collection::vec(any::<u8>(), 0..512),
            // valid UTF-8
            ".*".prop_map(String::into_bytes),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(16))]

        /// Blobs round-trip byte-exact through the odb; NUL-bearing or
        /// invalid-UTF-8 content gates to None.
        #[test]
        fn odb_roundtrips_arbitrary_contents(
            contents in proptest::collection::vec(blob_content(), 1..6)
        ) {
            let dir = tempfile::tempdir().unwrap();
            let p: &Path = dir.path();
            git(p, &["init"]);
            let mut oids = HashMap::new();
            for (i, bytes) in contents.iter().enumerate() {
                oids.insert(
                    (format!("f{i}"), Side::New),
                    BlobRef::Oid(hash_object(p, bytes)),
                );
            }
            let src = GixSource::new(gix::discover(p).unwrap(), oids);
            for (i, bytes) in contents.iter().enumerate() {
                let got = src.full_text(&format!("f{i}"), Side::New).unwrap();
                let expected = if bytes.contains(&0) {
                    None
                } else {
                    std::str::from_utf8(bytes).ok()
                };
                prop_assert_eq!(got.as_deref(), expected);
            }
        }
    }
}
