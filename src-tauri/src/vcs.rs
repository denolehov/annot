//! Structured diff enumeration backed by gitoxide (`gix`).
//!
//! `review_diff` accepts a `DiffTarget` — not arbitrary git CLI args — so the
//! revision semantics annot owns are exactly three comparisons: worktree vs
//! HEAD, index vs HEAD, and tree vs tree. Everything else (revspec grammar,
//! rename detection thresholds, pathspec magic) is delegated to gix.
//!
//! Non-UTF-8 paths are a hard error — a lossily-converted path would silently
//! fail content lookups downstream, hiding a file from review. Unmerged
//! (conflicted) paths are an error too, never a silent skip.

use std::collections::BTreeMap;
use std::path::Path;

use gix::bstr::{BString, ByteSlice};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::AnnotError;

/// What to diff. The MCP schema for `review_diff`'s `target` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffTarget {
    /// Worktree vs HEAD: staged + unstaged combined, untracked files included.
    WorkingTree,
    /// Index vs HEAD.
    Staged,
    /// Two-revision diff.
    Range {
        from: String,
        to: String,
        /// If true, diff from merge_base(from, to) to `to` (like `from...to`).
        #[serde(default)]
        merge_base: bool,
    },
}

impl DiffTarget {
    /// Display label for the review window.
    pub fn label(&self) -> String {
        match self {
            DiffTarget::WorkingTree => "diff".into(),
            DiffTarget::Staged => "staged".into(),
            DiffTarget::Range {
                from,
                to,
                merge_base,
            } => format!("{from}{}{to}", if *merge_base { "..." } else { ".." }),
        }
    }
}

/// Compose `git diff` CLI arguments equivalent to a target — the legacy
/// patch-text path uses these until the in-process pipeline (B4) lands.
pub fn to_git_args(target: &DiffTarget, pathspecs: &[String]) -> Vec<String> {
    let mut args = match target {
        DiffTarget::WorkingTree => vec!["HEAD".to_string()],
        DiffTarget::Staged => vec!["--staged".to_string()],
        DiffTarget::Range {
            from,
            to,
            merge_base,
        } => vec![format!(
            "{from}{}{to}",
            if *merge_base { "..." } else { ".." }
        )],
    };
    if !pathspecs.is_empty() {
        args.push("--".into());
        args.extend(pathspecs.iter().cloned());
    }
    args
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed { similarity: u8 },
    Copied,
    TypeChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobRef {
    Oid(String),
    WorkingTree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub status: FileStatus,
    /// `None` for added files.
    pub old_path: Option<String>,
    /// `None` for deleted files.
    pub new_path: Option<String>,
    /// `None` = old side nonexistent (added file).
    pub old_oid: Option<String>,
    /// `None` = new side nonexistent (deleted file).
    pub new_oid: Option<BlobRef>,
}

/// Enumerate changed files for `target` in the repository containing `cwd`.
pub fn enumerate(
    cwd: &Path,
    target: &DiffTarget,
    pathspecs: &[String],
) -> Result<Vec<FileEntry>, AnnotError> {
    let repo = gix::discover(cwd)
        .map_err(|e| AnnotError::Diff(format!("failed to open git repository: {e}")))?;
    let mut entries = match target {
        DiffTarget::Range {
            from,
            to,
            merge_base,
        } => range_entries(&repo, from, to, *merge_base, pathspecs)?,
        DiffTarget::Staged => staged_entries(&repo, pathspecs)?,
        DiffTarget::WorkingTree => working_tree_entries(&repo, pathspecs)?,
    };
    // Status items arrive interleaved from two producer threads — the sort is
    // mandatory for deterministic output, not cosmetic.
    entries.sort_by(|a, b| entry_path(a).cmp(entry_path(b)));
    Ok(entries)
}

fn entry_path(e: &FileEntry) -> &str {
    e.new_path
        .as_deref()
        .or(e.old_path.as_deref())
        .unwrap_or("")
}

fn diff_err(e: impl std::fmt::Display) -> AnnotError {
    AnnotError::Diff(e.to_string())
}

fn path_string(loc: &[u8]) -> Result<String, AnnotError> {
    std::str::from_utf8(loc)
        .map(str::to_string)
        .map_err(|_| AnnotError::Diff("non-UTF-8 path in git repository is not supported".into()))
}

/// Broad type class for TypeChanged detection: exec-bit changes stay
/// Modified; blob <-> symlink <-> submodule flips are TypeChanged (git's `T`).
fn kind_class(kind: gix::objs::tree::EntryKind) -> u8 {
    use gix::objs::tree::EntryKind;
    match kind {
        EntryKind::Blob | EntryKind::BlobExecutable => 0,
        EntryKind::Link => 1,
        EntryKind::Commit => 2,
        EntryKind::Tree => 3,
    }
}

fn tree_mode_class(mode: gix::objs::tree::EntryMode) -> u8 {
    kind_class(mode.kind())
}

fn index_mode_class(mode: gix::index::entry::Mode) -> Option<u8> {
    mode.to_tree_entry_mode().map(|m| kind_class(m.kind()))
}

/// Similarity percentage for a rename/copy detected by the index diff, which
/// (unlike the tree diff) carries no line stats. Identity => 100; otherwise a
/// best-effort content ratio; 50 (the detection threshold) if unreadable.
fn rename_similarity(
    repo: &gix::Repository,
    source_id: &gix::hash::oid,
    id: &gix::hash::oid,
) -> u8 {
    if source_id == id {
        return 100;
    }
    let load = |oid: &gix::hash::oid| -> Option<String> {
        let data = repo.find_object(oid.to_owned()).ok()?.detach().data;
        String::from_utf8(data).ok()
    };
    match (load(source_id), load(id)) {
        (Some(old), Some(new)) => {
            (similar::TextDiff::from_lines(&old, &new).ratio() * 100.0).round() as u8
        }
        _ => 50,
    }
}

// ---------------------------------------------------------------------------
// Range: tree vs tree
// ---------------------------------------------------------------------------

fn peel_to_tree<'r>(id: gix::Id<'r>, rev: &str) -> Result<gix::Tree<'r>, AnnotError> {
    id.object()
        .map_err(|e| AnnotError::Diff(format!("failed to load '{rev}': {e}")))?
        .peel_to_tree()
        .map_err(|e| AnnotError::Diff(format!("'{rev}' does not point to a tree: {e}")))
}

fn range_entries(
    repo: &gix::Repository,
    from: &str,
    to: &str,
    merge_base: bool,
    pathspecs: &[String],
) -> Result<Vec<FileEntry>, AnnotError> {
    let resolve = |rev: &str| {
        repo.rev_parse_single(rev)
            .map_err(|e| AnnotError::Diff(format!("failed to resolve '{rev}': {e}")))
    };
    let mut from_id = resolve(from)?;
    let to_id = resolve(to)?;
    if merge_base {
        from_id = repo.merge_base(from_id, to_id).map_err(|e| {
            AnnotError::Diff(format!(
                "failed to compute merge base of '{from}' and '{to}': {e}"
            ))
        })?;
    }
    let old_tree = peel_to_tree(from_id, from)?;
    let new_tree = peel_to_tree(to_id, to)?;

    let changes = repo
        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
        .map_err(|e| AnnotError::Diff(format!("tree diff failed: {e}")))?;

    let mut entries = Vec::new();
    for change in changes {
        if let Some(entry) = range_entry(change)? {
            entries.push(entry);
        }
    }
    filter_by_pathspec(repo, entries, pathspecs)
}

fn range_entry(
    change: gix::object::tree::diff::ChangeDetached,
) -> Result<Option<FileEntry>, AnnotError> {
    use gix::object::tree::diff::ChangeDetached as Change;
    Ok(match change {
        Change::Addition {
            location,
            entry_mode,
            id,
            ..
        } => {
            if entry_mode.is_tree() {
                return Ok(None);
            }
            let path = path_string(&location)?;
            Some(FileEntry {
                status: FileStatus::Added,
                old_path: None,
                new_path: Some(path),
                old_oid: None,
                new_oid: Some(BlobRef::Oid(id.to_string())),
            })
        }
        Change::Deletion {
            location,
            entry_mode,
            id,
            ..
        } => {
            if entry_mode.is_tree() {
                return Ok(None);
            }
            let path = path_string(&location)?;
            Some(FileEntry {
                status: FileStatus::Deleted,
                old_path: Some(path),
                new_path: None,
                old_oid: Some(id.to_string()),
                new_oid: None,
            })
        }
        Change::Modification {
            location,
            previous_entry_mode,
            previous_id,
            entry_mode,
            id,
        } => {
            if entry_mode.is_tree() && previous_entry_mode.is_tree() {
                return Ok(None);
            }
            let path = path_string(&location)?;
            let status = if tree_mode_class(previous_entry_mode) != tree_mode_class(entry_mode) {
                FileStatus::TypeChanged
            } else {
                FileStatus::Modified
            };
            Some(FileEntry {
                status,
                old_path: Some(path.clone()),
                new_path: Some(path),
                old_oid: Some(previous_id.to_string()),
                new_oid: Some(BlobRef::Oid(id.to_string())),
            })
        }
        Change::Rewrite {
            source_location,
            source_id,
            diff,
            id,
            location,
            copy,
            ..
        } => {
            let similarity = diff
                .map(|d| (d.similarity * 100.0).round() as u8)
                .unwrap_or(100);
            let status = if copy {
                FileStatus::Copied
            } else {
                FileStatus::Renamed { similarity }
            };
            Some(FileEntry {
                status,
                old_path: Some(path_string(&source_location)?),
                new_path: Some(path_string(&location)?),
                old_oid: Some(source_id.to_string()),
                new_oid: Some(BlobRef::Oid(id.to_string())),
            })
        }
    })
}

fn filter_by_pathspec(
    repo: &gix::Repository,
    entries: Vec<FileEntry>,
    pathspecs: &[String],
) -> Result<Vec<FileEntry>, AnnotError> {
    if pathspecs.is_empty() {
        return Ok(entries);
    }
    let mut search = repo
        .pathspec(
            true,
            pathspecs.iter().map(|s| s.as_str()),
            true,
            &gix::index::State::new(repo.object_hash()),
            gix::worktree::stack::state::attributes::Source::IdMapping,
        )
        .map_err(|e| AnnotError::Diff(format!("invalid pathspec: {e}")))?;
    let mut included = |path: &Option<String>| {
        path.as_deref()
            .is_some_and(|p| search.is_included(p.as_bytes().as_bstr(), Some(false)))
    };
    Ok(entries
        .into_iter()
        .filter(|e| included(&e.old_path) || included(&e.new_path))
        .collect())
}

// ---------------------------------------------------------------------------
// Staged / WorkingTree: index- and status-based layers + merge
// ---------------------------------------------------------------------------

/// Reduced, gix-free change descriptions — `merge::merge` is a pure function
/// over these so the combination table is unit-testable without fixtures.
mod merge {
    use super::{BlobRef, FileEntry, FileStatus};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum Staged {
        Added {
            index_oid: String,
            intent_to_add: bool,
        },
        Modified {
            head_oid: String,
            index_oid: String,
            type_changed: bool,
        },
        Deleted {
            head_oid: String,
        },
        Renamed {
            old_path: String,
            head_oid: String,
            index_oid: String,
            similarity: u8,
            copy: bool,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum WtKind {
        Modified,
        TypeChanged,
        Deleted,
        IntentToAdd,
        Untracked,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct Wt {
        pub kind: WtKind,
        /// The index entry's oid (== HEAD oid when the path has no staged
        /// change). `None` for untracked paths.
        pub index_oid: Option<String>,
    }

    /// Fuse the staged (HEAD vs index) and worktree (index vs worktree)
    /// layers into one HEAD-vs-worktree entry for `path`. `None` = no row.
    pub(super) fn merge(path: &str, staged: Option<Staged>, wt: Option<Wt>) -> Option<FileEntry> {
        let p = || Some(path.to_string());
        let entry =
            |status, old_path: Option<String>, old_oid, new_path: Option<String>, new_oid| {
                Some(FileEntry {
                    status,
                    old_path,
                    new_path,
                    old_oid,
                    new_oid,
                })
            };
        match (staged, wt) {
            (None, None) => None,

            // Unstaged-only rows: the index oid IS the HEAD oid (no staged change).
            (None, Some(w)) => match w.kind {
                WtKind::Modified => entry(
                    FileStatus::Modified,
                    p(),
                    w.index_oid,
                    p(),
                    Some(BlobRef::WorkingTree),
                ),
                WtKind::TypeChanged => entry(
                    FileStatus::TypeChanged,
                    p(),
                    w.index_oid,
                    p(),
                    Some(BlobRef::WorkingTree),
                ),
                WtKind::Deleted => entry(FileStatus::Deleted, p(), w.index_oid, None, None),
                WtKind::IntentToAdd | WtKind::Untracked => entry(
                    FileStatus::Added,
                    None,
                    None,
                    p(),
                    Some(BlobRef::WorkingTree),
                ),
            },

            // Staged-only rows: worktree matches the index, so the new side
            // is the real index oid — the pinned `git diff HEAD` semantics.
            (Some(s), None) => match s {
                Staged::Added {
                    index_oid,
                    intent_to_add,
                } => {
                    let new = if intent_to_add {
                        BlobRef::WorkingTree
                    } else {
                        BlobRef::Oid(index_oid)
                    };
                    entry(FileStatus::Added, None, None, p(), Some(new))
                }
                Staged::Modified {
                    head_oid,
                    index_oid,
                    type_changed,
                } => entry(
                    if type_changed {
                        FileStatus::TypeChanged
                    } else {
                        FileStatus::Modified
                    },
                    p(),
                    Some(head_oid),
                    p(),
                    Some(BlobRef::Oid(index_oid)),
                ),
                Staged::Deleted { head_oid } => {
                    entry(FileStatus::Deleted, p(), Some(head_oid), None, None)
                }
                Staged::Renamed {
                    old_path,
                    head_oid,
                    index_oid,
                    similarity,
                    copy,
                } => entry(
                    if copy {
                        FileStatus::Copied
                    } else {
                        FileStatus::Renamed { similarity }
                    },
                    Some(old_path),
                    Some(head_oid),
                    p(),
                    Some(BlobRef::Oid(index_oid)),
                ),
            },

            // Both layers: old side from the staged change, new side from the
            // worktree (WorkingTree unless the file is gone).
            (Some(s), Some(w)) => {
                let wt_deleted = w.kind == WtKind::Deleted;
                match s {
                    Staged::Added { .. } => {
                        if wt_deleted {
                            None // added to index, deleted in worktree: absent on both sides
                        } else {
                            entry(
                                FileStatus::Added,
                                None,
                                None,
                                p(),
                                Some(BlobRef::WorkingTree),
                            )
                        }
                    }
                    Staged::Modified {
                        head_oid,
                        type_changed,
                        ..
                    } => {
                        if wt_deleted {
                            entry(FileStatus::Deleted, p(), Some(head_oid), None, None)
                        } else {
                            entry(
                                if type_changed || w.kind == WtKind::TypeChanged {
                                    FileStatus::TypeChanged
                                } else {
                                    FileStatus::Modified
                                },
                                p(),
                                Some(head_oid),
                                p(),
                                Some(BlobRef::WorkingTree),
                            )
                        }
                    }
                    // Staged deletion + untracked recreation: vs HEAD that's a
                    // modification (git would omit it if content is identical
                    // — accepted divergence, the diff simply renders empty).
                    Staged::Deleted { head_oid } => entry(
                        FileStatus::Modified,
                        p(),
                        Some(head_oid),
                        p(),
                        Some(BlobRef::WorkingTree),
                    ),
                    Staged::Renamed {
                        old_path,
                        head_oid,
                        similarity,
                        copy,
                        ..
                    } => {
                        if wt_deleted {
                            entry(
                                FileStatus::Deleted,
                                Some(old_path),
                                Some(head_oid),
                                None,
                                None,
                            )
                        } else {
                            entry(
                                if copy {
                                    FileStatus::Copied
                                } else {
                                    FileStatus::Renamed { similarity }
                                },
                                Some(old_path),
                                Some(head_oid),
                                p(),
                                Some(BlobRef::WorkingTree),
                            )
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn wt(kind: WtKind) -> Option<Wt> {
            Some(Wt {
                kind,
                index_oid: Some("idx".into()),
            })
        }

        fn untracked() -> Option<Wt> {
            Some(Wt {
                kind: WtKind::Untracked,
                index_oid: None,
            })
        }

        fn staged_modified() -> Option<Staged> {
            Some(Staged::Modified {
                head_oid: "head".into(),
                index_oid: "idx".into(),
                type_changed: false,
            })
        }

        #[test]
        fn nothing_yields_nothing() {
            assert_eq!(merge("f", None, None), None);
        }

        #[test]
        fn unstaged_only_rows() {
            let m = merge("f", None, wt(WtKind::Modified)).unwrap();
            assert_eq!(m.status, FileStatus::Modified);
            assert_eq!(m.old_oid.as_deref(), Some("idx"));
            assert_eq!(m.new_oid, Some(BlobRef::WorkingTree));

            let d = merge("f", None, wt(WtKind::Deleted)).unwrap();
            assert_eq!(d.status, FileStatus::Deleted);
            assert_eq!(d.new_path, None);
            assert_eq!(d.new_oid, None);

            let t = merge("f", None, wt(WtKind::TypeChanged)).unwrap();
            assert_eq!(t.status, FileStatus::TypeChanged);

            let u = merge("f", None, untracked()).unwrap();
            assert_eq!(u.status, FileStatus::Added);
            assert_eq!(u.old_oid, None);
            assert_eq!(u.new_oid, Some(BlobRef::WorkingTree));

            let ita = merge("f", None, wt(WtKind::IntentToAdd)).unwrap();
            assert_eq!(ita.status, FileStatus::Added);
            assert_eq!(ita.new_oid, Some(BlobRef::WorkingTree));
        }

        #[test]
        fn staged_only_rows() {
            let a = merge(
                "f",
                Some(Staged::Added {
                    index_oid: "idx".into(),
                    intent_to_add: false,
                }),
                None,
            )
            .unwrap();
            assert_eq!(a.status, FileStatus::Added);
            assert_eq!(a.new_oid, Some(BlobRef::Oid("idx".into())));

            // the pinned case: staged change, clean worktree => real index oid
            let m = merge("f", staged_modified(), None).unwrap();
            assert_eq!(m.old_oid.as_deref(), Some("head"));
            assert_eq!(m.new_oid, Some(BlobRef::Oid("idx".into())));

            let r = merge(
                "new",
                Some(Staged::Renamed {
                    old_path: "old".into(),
                    head_oid: "head".into(),
                    index_oid: "idx".into(),
                    similarity: 100,
                    copy: false,
                }),
                None,
            )
            .unwrap();
            assert_eq!(r.status, FileStatus::Renamed { similarity: 100 });
            assert_eq!(r.old_path.as_deref(), Some("old"));
            assert_eq!(r.new_path.as_deref(), Some("new"));

            let ita = merge(
                "f",
                Some(Staged::Added {
                    index_oid: "empty".into(),
                    intent_to_add: true,
                }),
                None,
            )
            .unwrap();
            assert_eq!(ita.new_oid, Some(BlobRef::WorkingTree));
        }

        #[test]
        fn both_layer_rows() {
            // staged-A + worktree-M => Added with worktree content
            let a = merge(
                "f",
                Some(Staged::Added {
                    index_oid: "idx".into(),
                    intent_to_add: false,
                }),
                wt(WtKind::Modified),
            )
            .unwrap();
            assert_eq!(a.status, FileStatus::Added);
            assert_eq!(a.new_oid, Some(BlobRef::WorkingTree));

            // staged-A + worktree-D => absent on both sides
            assert_eq!(
                merge(
                    "f",
                    Some(Staged::Added {
                        index_oid: "idx".into(),
                        intent_to_add: false,
                    }),
                    wt(WtKind::Deleted),
                ),
                None
            );

            // staged-M + worktree-M => one Modified row vs HEAD
            let m = merge("f", staged_modified(), wt(WtKind::Modified)).unwrap();
            assert_eq!(m.status, FileStatus::Modified);
            assert_eq!(m.old_oid.as_deref(), Some("head"));
            assert_eq!(m.new_oid, Some(BlobRef::WorkingTree));

            // staged-M + worktree-rm => Deleted vs HEAD
            let d = merge("f", staged_modified(), wt(WtKind::Deleted)).unwrap();
            assert_eq!(d.status, FileStatus::Deleted);
            assert_eq!(d.old_oid.as_deref(), Some("head"));

            // staged-D + untracked recreation => Modified vs HEAD
            let re = merge(
                "f",
                Some(Staged::Deleted {
                    head_oid: "head".into(),
                }),
                untracked(),
            )
            .unwrap();
            assert_eq!(re.status, FileStatus::Modified);
            assert_eq!(re.new_oid, Some(BlobRef::WorkingTree));

            // staged-R + worktree-M of destination => rename with WT content
            let r = merge(
                "new",
                Some(Staged::Renamed {
                    old_path: "old".into(),
                    head_oid: "head".into(),
                    index_oid: "idx".into(),
                    similarity: 90,
                    copy: false,
                }),
                wt(WtKind::Modified),
            )
            .unwrap();
            assert_eq!(r.status, FileStatus::Renamed { similarity: 90 });
            assert_eq!(r.old_path.as_deref(), Some("old"));
            assert_eq!(r.new_oid, Some(BlobRef::WorkingTree));

            // staged-R + worktree-rm of destination => Deleted from old path
            let rd = merge(
                "new",
                Some(Staged::Renamed {
                    old_path: "old".into(),
                    head_oid: "head".into(),
                    index_oid: "idx".into(),
                    similarity: 100,
                    copy: false,
                }),
                wt(WtKind::Deleted),
            )
            .unwrap();
            assert_eq!(rd.status, FileStatus::Deleted);
            assert_eq!(rd.old_path.as_deref(), Some("old"));
            assert_eq!(rd.new_path, None);
        }
    }
}

/// Convert one HEAD-vs-index change into `(dest_path, Staged)`.
/// `worktree_index` is consulted for the intent-to-add flag on additions.
fn staged_change(
    repo: &gix::Repository,
    change: gix::diff::index::Change,
    worktree_index: &gix::index::State,
) -> Result<(String, merge::Staged), AnnotError> {
    use gix::diff::index::Change;
    Ok(match change {
        Change::Addition {
            location,
            index,
            id,
            ..
        } => {
            let intent_to_add = worktree_index
                .entries()
                .get(index)
                .is_some_and(|e| e.flags.contains(gix::index::entry::Flags::INTENT_TO_ADD));
            (
                path_string(&location)?,
                merge::Staged::Added {
                    index_oid: id.to_hex().to_string(),
                    intent_to_add,
                },
            )
        }
        Change::Deletion { location, id, .. } => (
            path_string(&location)?,
            merge::Staged::Deleted {
                head_oid: id.to_hex().to_string(),
            },
        ),
        Change::Modification {
            location,
            previous_entry_mode,
            previous_id,
            entry_mode,
            id,
            ..
        } => {
            let type_changed = match (
                index_mode_class(previous_entry_mode),
                index_mode_class(entry_mode),
            ) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            };
            (
                path_string(&location)?,
                merge::Staged::Modified {
                    head_oid: previous_id.to_hex().to_string(),
                    index_oid: id.to_hex().to_string(),
                    type_changed,
                },
            )
        }
        Change::Rewrite {
            source_location,
            source_id,
            location,
            id,
            copy,
            ..
        } => {
            let similarity = rename_similarity(repo, &source_id, &id);
            (
                path_string(&location)?,
                merge::Staged::Renamed {
                    old_path: path_string(&source_location)?,
                    head_oid: source_id.to_hex().to_string(),
                    index_oid: id.to_hex().to_string(),
                    similarity,
                    copy,
                },
            )
        }
    })
}

fn staged_entries(
    repo: &gix::Repository,
    pathspecs: &[String],
) -> Result<Vec<FileEntry>, AnnotError> {
    let head_tree = repo.head_tree_id_or_empty().map_err(diff_err)?;
    let index = repo.index_or_empty().map_err(diff_err)?;
    let mut pathspec = repo
        .pathspec(
            true,
            pathspecs.iter().map(|s| s.as_str()),
            true,
            &index,
            gix::worktree::stack::state::attributes::Source::IdMapping,
        )
        .map_err(|e| AnnotError::Diff(format!("invalid pathspec: {e}")))?;

    let mut changes = Vec::new();
    repo.tree_index_status(
        &head_tree,
        &index,
        Some(&mut pathspec),
        gix::status::tree_index::TrackRenames::AsConfigured,
        |change, _tree_index, worktree_index| -> Result<_, std::convert::Infallible> {
            changes.push(staged_change(repo, change.into_owned(), worktree_index));
            Ok(gix::diff::index::Action::Continue(()))
        },
    )
    .map_err(diff_err)?;

    let mut entries = Vec::new();
    for change in changes {
        let (path, staged) = change?;
        // git diff --staged omits intent-to-add entries: nothing is staged yet.
        if matches!(
            staged,
            merge::Staged::Added {
                intent_to_add: true,
                ..
            }
        ) {
            continue;
        }
        if let Some(entry) = merge::merge(&path, Some(staged), None) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn working_tree_entries(
    repo: &gix::Repository,
    pathspecs: &[String],
) -> Result<Vec<FileEntry>, AnnotError> {
    use gix::status::plumbing::index_as_worktree::{Change as IwChange, EntryStatus};

    let iter = repo
        .status(gix::progress::Discard)
        .map_err(diff_err)?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_submodules(None)
        .into_iter(pathspecs.iter().map(|s| BString::from(s.as_str())))
        .map_err(diff_err)?;

    let mut staged: BTreeMap<String, merge::Staged> = BTreeMap::new();
    let mut worktree: BTreeMap<String, merge::Wt> = BTreeMap::new();
    // The worktree index isn't accessible per-item here; intent-to-add
    // additions are detected via the IndexWorktree layer instead, which
    // always reports them as EntryStatus::IntentToAdd.
    let empty_index = gix::index::State::new(repo.object_hash());

    for item in iter {
        let item = item.map_err(diff_err)?;
        match item {
            gix::status::Item::TreeIndex(change) => {
                let (path, s) = staged_change(repo, change, &empty_index)?;
                staged.insert(path, s);
            }
            gix::status::Item::IndexWorktree(item) => {
                use gix::status::index_worktree::Item;
                match item {
                    Item::Modification {
                        entry,
                        rela_path,
                        status,
                        ..
                    } => {
                        let path = path_string(&rela_path)?;
                        let kind = match status {
                            EntryStatus::Conflict { .. } => {
                                return Err(AnnotError::Diff(format!(
                                    "unmerged path (unresolved conflict): {path}"
                                )));
                            }
                            EntryStatus::NeedsUpdate(_) => continue,
                            EntryStatus::IntentToAdd => merge::WtKind::IntentToAdd,
                            EntryStatus::Change(change) => match change {
                                IwChange::Removed => merge::WtKind::Deleted,
                                IwChange::Type { .. } => merge::WtKind::TypeChanged,
                                IwChange::Modification { .. } => merge::WtKind::Modified,
                                IwChange::SubmoduleModification(_) => continue,
                            },
                        };
                        let index_oid =
                            (kind != merge::WtKind::IntentToAdd).then(|| entry.id.to_string());
                        worktree.insert(path, merge::Wt { kind, index_oid });
                    }
                    Item::DirectoryContents { entry, .. } => {
                        if entry.status == gix::dir::entry::Status::Untracked
                            && entry
                                .disk_kind
                                .is_some_and(|k| !matches!(k, gix::dir::entry::Kind::Directory))
                        {
                            worktree.insert(
                                path_string(&entry.rela_path)?,
                                merge::Wt {
                                    kind: merge::WtKind::Untracked,
                                    index_oid: None,
                                },
                            );
                        }
                    }
                    Item::Rewrite { .. } => {
                        // index-worktree rename tracking is disabled; loud is
                        // better than a silently mis-merged rename.
                        return Err(AnnotError::Diff(
                            "unexpected worktree rename item from git status".into(),
                        ));
                    }
                }
            }
        }
    }

    let mut entries = Vec::new();
    for (path, s) in staged {
        let wt = worktree.remove(&path);
        if let Some(entry) = merge::merge(&path, Some(s), wt) {
            entries.push(entry);
        }
    }
    for (path, wt) in worktree {
        if let Some(entry) = merge::merge(&path, None, Some(wt)) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::git;
    use std::fs;
    use std::process::Command;

    fn strs(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    fn oid(dir: &Path, rev_path: &str) -> String {
        git(dir, &["rev-parse", rev_path])
    }

    const WT: DiffTarget = DiffTarget::WorkingTree;
    const STAGED: DiffTarget = DiffTarget::Staged;

    fn range(from: &str, to: &str) -> DiffTarget {
        DiffTarget::Range {
            from: from.into(),
            to: to.into(),
            merge_base: false,
        }
    }

    /// One commit: a.txt, b.txt, old.txt, .gitignore (ignoring ignored.txt).
    /// Local `diff.renames=true` pins rename detection regardless of the
    /// developer's global config (`enumerate` runs without the hermetic env).
    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        git(p, &["config", "diff.renames", "true"]);
        fs::write(p.join("a.txt"), "alpha\n").unwrap();
        fs::write(p.join("b.txt"), "bravo\n").unwrap();
        fs::write(p.join("old.txt"), "stable rename content\n").unwrap();
        fs::write(p.join(".gitignore"), "ignored.txt\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "base"]);
        dir
    }

    /// `repo()` plus a second commit: modify a.txt, add new.txt,
    /// delete b.txt, rename old.txt -> renamed.txt.
    fn two_commit_repo() -> tempfile::TempDir {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "alpha2\n").unwrap();
        fs::write(p.join("new.txt"), "fresh\n").unwrap();
        git(p, &["rm", "-q", "b.txt"]);
        git(p, &["mv", "old.txt", "renamed.txt"]);
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "second"]);
        dir
    }

    fn find<'a>(entries: &'a [FileEntry], path: &str) -> &'a FileEntry {
        entries
            .iter()
            .find(|e| e.new_path.as_deref() == Some(path) || e.old_path.as_deref() == Some(path))
            .unwrap_or_else(|| panic!("no entry for {path}: {entries:?}"))
    }

    // --- DiffTarget / to_git_args ---

    #[test]
    fn to_git_args_table() {
        assert_eq!(to_git_args(&WT, &[]), vec!["HEAD"]);
        assert_eq!(to_git_args(&STAGED, &[]), vec!["--staged"]);
        assert_eq!(to_git_args(&range("main", "HEAD"), &[]), vec!["main..HEAD"]);
        assert_eq!(
            to_git_args(
                &DiffTarget::Range {
                    from: "main".into(),
                    to: "HEAD".into(),
                    merge_base: true,
                },
                &[]
            ),
            vec!["main...HEAD"]
        );
        assert_eq!(
            to_git_args(&WT, &strs(&["src/", "*.rs"])),
            vec!["HEAD", "--", "src/", "*.rs"]
        );
    }

    #[test]
    fn diff_target_serde_roundtrip() {
        let range = DiffTarget::Range {
            from: "main".into(),
            to: "HEAD".into(),
            merge_base: true,
        };
        for target in [DiffTarget::WorkingTree, DiffTarget::Staged, range] {
            let json = serde_json::to_string(&target).unwrap();
            assert_eq!(serde_json::from_str::<DiffTarget>(&json).unwrap(), target);
        }
        // wire shape + merge_base default
        assert_eq!(
            serde_json::from_str::<DiffTarget>(r#"{"kind":"range","from":"a","to":"b"}"#).unwrap(),
            DiffTarget::Range {
                from: "a".into(),
                to: "b".into(),
                merge_base: false,
            }
        );
        assert_eq!(
            serde_json::to_string(&DiffTarget::WorkingTree).unwrap(),
            r#"{"kind":"working_tree"}"#
        );
    }

    #[test]
    fn labels() {
        assert_eq!(WT.label(), "diff");
        assert_eq!(STAGED.label(), "staged");
        assert_eq!(range("a", "b").label(), "a..b");
    }

    // --- WorkingTree ---

    #[test]
    fn empty_diff_is_empty_vec() {
        let dir = repo();
        assert_eq!(enumerate(dir.path(), &WT, &[]).unwrap(), vec![]);
    }

    #[test]
    fn unstaged_worktree_diff() {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "alpha2\n").unwrap();
        fs::remove_file(p.join("b.txt")).unwrap();
        let entries = enumerate(p, &WT, &[]).unwrap();
        assert_eq!(
            entries,
            vec![
                FileEntry {
                    status: FileStatus::Modified,
                    old_path: Some("a.txt".into()),
                    new_path: Some("a.txt".into()),
                    old_oid: Some(oid(p, "HEAD:a.txt")),
                    new_oid: Some(BlobRef::WorkingTree),
                },
                FileEntry {
                    status: FileStatus::Deleted,
                    old_path: Some("b.txt".into()),
                    new_path: None,
                    old_oid: Some(oid(p, "HEAD:b.txt")),
                    new_oid: None,
                },
            ]
        );
    }

    #[test]
    fn staged_change_via_head_shows_index_oid() {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "alpha2\n").unwrap();
        git(p, &["add", "a.txt"]);
        let entries = enumerate(p, &WT, &[]).unwrap();
        // Worktree matches the index, so the new side is the real index oid —
        // NOT WorkingTree. The merge keys off layer presence, not mode.
        assert_eq!(
            entries,
            vec![FileEntry {
                status: FileStatus::Modified,
                old_path: Some("a.txt".into()),
                new_path: Some("a.txt".into()),
                old_oid: Some(oid(p, "HEAD:a.txt")),
                new_oid: Some(BlobRef::Oid(oid(p, ":a.txt"))),
            }]
        );
    }

    #[test]
    fn untracked_files_appear_as_added() {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("brand-new.txt"), "hello\n").unwrap();
        fs::create_dir(p.join("nested")).unwrap();
        fs::write(p.join("nested/inner.txt"), "deep\n").unwrap();
        let entries = enumerate(p, &WT, &[]).unwrap();
        assert_eq!(
            find(&entries, "brand-new.txt"),
            &FileEntry {
                status: FileStatus::Added,
                old_path: None,
                new_path: Some("brand-new.txt".into()),
                old_oid: None,
                new_oid: Some(BlobRef::WorkingTree),
            }
        );
        // UntrackedFiles::Files expands directories to per-file entries
        assert_eq!(find(&entries, "nested/inner.txt").status, FileStatus::Added);
    }

    #[test]
    fn gitignored_files_do_not_appear() {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("ignored.txt"), "invisible\n").unwrap();
        assert_eq!(enumerate(p, &WT, &[]).unwrap(), vec![]);
    }

    #[test]
    fn staged_plus_worktree_combinations() {
        let dir = repo();
        let p = dir.path();
        // staged-A + worktree-M
        fs::write(p.join("new.txt"), "v1\n").unwrap();
        git(p, &["add", "new.txt"]);
        fs::write(p.join("new.txt"), "v2\n").unwrap();
        // staged-M + worktree-rm
        fs::write(p.join("a.txt"), "staged\n").unwrap();
        git(p, &["add", "a.txt"]);
        fs::remove_file(p.join("a.txt")).unwrap();
        // staged-R + worktree-M of destination
        git(p, &["mv", "old.txt", "moved.txt"]);
        fs::write(p.join("moved.txt"), "stable rename content\nplus\n").unwrap();

        let entries = enumerate(p, &WT, &[]).unwrap();
        assert_eq!(
            find(&entries, "new.txt"),
            &FileEntry {
                status: FileStatus::Added,
                old_path: None,
                new_path: Some("new.txt".into()),
                old_oid: None,
                new_oid: Some(BlobRef::WorkingTree),
            }
        );
        assert_eq!(
            find(&entries, "a.txt"),
            &FileEntry {
                status: FileStatus::Deleted,
                old_path: Some("a.txt".into()),
                new_path: None,
                old_oid: Some(oid(p, "HEAD:a.txt")),
                new_oid: None,
            }
        );
        let moved = find(&entries, "moved.txt");
        assert!(
            matches!(moved.status, FileStatus::Renamed { .. }),
            "{moved:?}"
        );
        assert_eq!(moved.old_path.as_deref(), Some("old.txt"));
        assert_eq!(moved.new_oid, Some(BlobRef::WorkingTree));
    }

    #[test]
    fn conflict_is_an_error() {
        let dir = repo();
        let p = dir.path();
        git(p, &["switch", "-c", "side"]);
        fs::write(p.join("a.txt"), "side\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "side"]);
        git(p, &["switch", "main"]);
        fs::write(p.join("a.txt"), "main\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "main"]);
        // merge fails with a conflict — run raw, ignoring the exit code
        Command::new("git")
            .args(["merge", "side"])
            .current_dir(p)
            .output()
            .unwrap();
        let err = enumerate(p, &WT, &[]).unwrap_err();
        assert!(err.to_string().contains("unmerged"), "{err}");
    }

    // --- Staged ---

    #[test]
    fn staged_diff() {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "alpha2\n").unwrap();
        fs::write(p.join("new.txt"), "fresh\n").unwrap();
        git(p, &["rm", "-q", "b.txt"]);
        git(p, &["mv", "old.txt", "renamed.txt"]);
        git(p, &["add", "."]);
        let entries = enumerate(p, &STAGED, &[]).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(
            find(&entries, "a.txt"),
            &FileEntry {
                status: FileStatus::Modified,
                old_path: Some("a.txt".into()),
                new_path: Some("a.txt".into()),
                old_oid: Some(oid(p, "HEAD:a.txt")),
                new_oid: Some(BlobRef::Oid(oid(p, ":a.txt"))),
            }
        );
        assert_eq!(
            find(&entries, "new.txt"),
            &FileEntry {
                status: FileStatus::Added,
                old_path: None,
                new_path: Some("new.txt".into()),
                old_oid: None,
                new_oid: Some(BlobRef::Oid(oid(p, ":new.txt"))),
            }
        );
        assert_eq!(
            find(&entries, "b.txt"),
            &FileEntry {
                status: FileStatus::Deleted,
                old_path: Some("b.txt".into()),
                new_path: None,
                old_oid: Some(oid(p, "HEAD:b.txt")),
                new_oid: None,
            }
        );
        assert_eq!(
            find(&entries, "renamed.txt"),
            &FileEntry {
                status: FileStatus::Renamed { similarity: 100 },
                old_path: Some("old.txt".into()),
                new_path: Some("renamed.txt".into()),
                old_oid: Some(oid(p, "HEAD:old.txt")),
                new_oid: Some(BlobRef::Oid(oid(p, ":renamed.txt"))),
            }
        );
    }

    #[test]
    fn paths_with_spaces_and_unicode() {
        let dir = repo();
        let p = dir.path();
        let tricky = "spa ce δοκιμή 试.txt";
        git(p, &["mv", "old.txt", tricky]);
        let entries = enumerate(p, &STAGED, &[]).unwrap();
        assert_eq!(
            entries,
            vec![FileEntry {
                status: FileStatus::Renamed { similarity: 100 },
                old_path: Some("old.txt".into()),
                new_path: Some(tricky.into()),
                old_oid: Some(oid(p, "HEAD:old.txt")),
                new_oid: Some(BlobRef::Oid(oid(p, "HEAD:old.txt"))),
            }]
        );
    }

    #[cfg(unix)]
    #[test]
    fn typechange() {
        let dir = repo();
        let p = dir.path();
        fs::remove_file(p.join("a.txt")).unwrap();
        std::os::unix::fs::symlink("b.txt", p.join("a.txt")).unwrap();
        git(p, &["add", "a.txt"]);
        let entries = enumerate(p, &STAGED, &[]).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, FileStatus::TypeChanged);
        assert_eq!(entries[0].old_path.as_deref(), Some("a.txt"));
        assert_eq!(entries[0].new_path.as_deref(), Some("a.txt"));
    }

    // --- Range ---

    #[test]
    fn rev_range() {
        let dir = two_commit_repo();
        let p = dir.path();
        let entries = enumerate(p, &range("HEAD~1", "HEAD"), &[]).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(
            find(&entries, "a.txt").old_oid,
            Some(oid(p, "HEAD~1:a.txt"))
        );
        assert_eq!(
            find(&entries, "a.txt").new_oid,
            Some(BlobRef::Oid(oid(p, "HEAD:a.txt")))
        );
        assert_eq!(find(&entries, "new.txt").status, FileStatus::Added);
        assert_eq!(find(&entries, "b.txt").status, FileStatus::Deleted);
        assert_eq!(
            find(&entries, "renamed.txt"),
            &FileEntry {
                status: FileStatus::Renamed { similarity: 100 },
                old_path: Some("old.txt".into()),
                new_path: Some("renamed.txt".into()),
                old_oid: Some(oid(p, "HEAD~1:old.txt")),
                new_oid: Some(BlobRef::Oid(oid(p, "HEAD:renamed.txt"))),
            }
        );
    }

    #[test]
    fn merge_base_range() {
        let dir = repo();
        let p = dir.path();
        git(p, &["switch", "-c", "feature"]);
        fs::write(p.join("feature.txt"), "feat\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "feature work"]);
        git(p, &["switch", "main"]);
        fs::write(p.join("a.txt"), "moved ahead\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "main moved on"]);

        // plain two-dot from main..feature includes main's later change (as a
        // reverse-modification); merge-base mode shows only the branch's work
        let plain = enumerate(p, &range("main", "feature"), &[]).unwrap();
        assert!(plain.iter().any(|e| e.new_path.as_deref() == Some("a.txt")));

        let mb = enumerate(
            p,
            &DiffTarget::Range {
                from: "main".into(),
                to: "feature".into(),
                merge_base: true,
            },
            &[],
        )
        .unwrap();
        assert_eq!(mb.len(), 1);
        assert_eq!(mb[0].new_path.as_deref(), Some("feature.txt"));
        assert_eq!(mb[0].status, FileStatus::Added);
    }

    #[test]
    fn pathspec_filter() {
        let dir = two_commit_repo();
        let p = dir.path();
        let entries = enumerate(p, &range("HEAD~1", "HEAD"), &strs(&["a.txt"])).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].new_path.as_deref(), Some("a.txt"));
    }

    #[test]
    fn pathspec_filter_working_tree() {
        let dir = repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "alpha2\n").unwrap();
        fs::write(p.join("b.txt"), "bravo2\n").unwrap();
        let entries = enumerate(p, &WT, &strs(&["b.txt"])).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].new_path.as_deref(), Some("b.txt"));
    }

    #[test]
    fn copied_status_via_config() {
        let dir = repo();
        let p = dir.path();
        git(p, &["config", "diff.renames", "copies"]);
        let content: String = (1..=10).map(|i| format!("line {i}\n")).collect();
        fs::write(p.join("src.txt"), &content).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "add src"]);
        // copy detection sources come from the modified set, so modify the
        // source file in the same change-set (no --find-copies-harder analog);
        // multi-line content keeps similarity above the 50% threshold
        fs::write(p.join("src-copy.txt"), &content).unwrap();
        fs::write(p.join("src.txt"), content.replace("line 1\n", "line one\n")).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "copy"]);
        let entries = enumerate(p, &range("HEAD~1", "HEAD"), &[]).unwrap();
        let copy = find(&entries, "src-copy.txt");
        assert_eq!(copy.status, FileStatus::Copied);
        assert_eq!(copy.old_path.as_deref(), Some("src.txt"));
    }

    #[test]
    fn bogus_rev_is_err() {
        let dir = repo();
        let err = enumerate(dir.path(), &range("no-such-rev-zzz", "HEAD"), &[]).unwrap_err();
        assert!(err.to_string().contains("no-such-rev-zzz"), "{err}");
    }

    #[test]
    fn non_repo_dir_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let err = enumerate(dir.path(), &WT, &[]).unwrap_err();
        assert!(err.to_string().contains("repository"), "{err}");
    }
}
