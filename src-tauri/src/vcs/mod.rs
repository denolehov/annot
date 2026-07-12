//! Version-control substrate: `DiffTarget` in, `FileEntry` + `FileSource` out.
//!
//! Two tiers sit behind this seam — [`git`] (gitoxide) and [`jj`] (jj-lib).
//! `.jj` wins over a colocated `.git`: in a colocated repo git's HEAD and
//! index are export artifacts of jj's state, not user intent.
//!
//! The comparisons annot owns are named, not composed: a diff calculus
//! (`{old: Side, new: Side}` for arbitrary sides) would make annot own the
//! semantics of every pairing. Revision *strings* are backend dialect —
//! annot never parses them, it hands them to gix's revspec parser or jj's
//! revset engine. The user's native tongue per repo is the point.

pub mod git;
pub mod jj;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::AnnotError;
use crate::source::FileSource;

/// What to diff. The MCP schema for `review_diff`'s `target` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffTarget {
    /// Uncommitted work vs its base.
    ///
    /// git: worktree vs HEAD (staged + unstaged, untracked included).
    /// jj: snapshot the working copy, then `@` vs its parent(s) — the
    /// snapshot is the only write annot ever performs on a repository.
    WorkingCopy,
    /// One revision vs its parent(s).
    ///
    /// git: `rev` vs its *first* parent (`git show`'s convention).
    /// jj: `rev` vs its *auto-merged* parents (`jj diff -r`'s convention).
    /// Each tier keeps its native convention: a user gets the answer their
    /// own `diff` command would give.
    ///
    /// In jj, `rev` is a revset, so it may name *several* commits. When they
    /// form one contiguous stack (one root, one head, no gaps) they review as a
    /// single changeset — base of the stack vs its tip, which is what
    /// `annot diff 'trunk()..@'` means. Anything else is an error: several
    /// disjoint stacks have no single diff, and picking one would show changes
    /// the user never asked about.
    Revision { rev: String },
    /// Two revisions. An empty side means "the current revision" and is
    /// filled in by the tier — `HEAD` for git, `@` for jj — because that
    /// name is dialect, not something annot gets to decide.
    Range {
        from: String,
        to: String,
        /// If true, diff from merge_base(from, to) to `to` (like `from...to`).
        #[serde(default)]
        merge_base: bool,
    },
    /// Index vs HEAD. Git-only — the jj tier rejects it.
    Staged,
}

impl DiffTarget {
    /// Display label for the review window.
    pub fn label(&self) -> String {
        match self {
            DiffTarget::WorkingCopy => "diff".into(),
            DiffTarget::Staged => "staged".into(),
            DiffTarget::Revision { rev } => rev.clone(),
            DiffTarget::Range {
                from,
                to,
                merge_base,
            } => format!("{from}{}{to}", if *merge_base { "..." } else { ".." }),
        }
    }
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

/// Serializes as a plain string — the wire doesn't carry `similarity`.
impl serde::Serialize for FileStatus {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            FileStatus::Modified => "modified",
            FileStatus::Added => "added",
            FileStatus::Deleted => "deleted",
            FileStatus::Renamed { .. } => "renamed",
            FileStatus::Copied => "copied",
            FileStatus::TypeChanged => "type_changed",
        })
    }
}

/// How a side's content is addressed.
///
/// Git tier: a real blob oid, or the working-tree file. Jj tier: an opaque
/// content key minted by the enumerator — `JjSource` holds the real
/// `(path, side) -> value` map (a conflicted side has no single id), so the
/// key here carries only *existence*, which is what the pipeline reads it
/// for (`Some` = the side exists; `None` = it doesn't).
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

/// An opened repository, ready to enumerate — the tier is chosen once, here.
pub enum Backend {
    Git(Box<gix::Repository>),
    Jj(Box<jj::JjRepo>),
}

/// Everything a session needs from the VCS: the changed files, a source for
/// their full texts, and (jj) a resolved label naming what was diffed.
pub struct Prepared {
    pub entries: Vec<FileEntry>,
    pub source: Arc<dyn FileSource>,
    /// Tier-supplied label override — jj names commits change-ID-first.
    pub label: Option<String>,
}

/// Open the repository containing `cwd`. `.jj` wins over `.git`.
pub fn open(cwd: &Path) -> Result<Backend, AnnotError> {
    match jj::discover(cwd) {
        Some(root) => Ok(Backend::Jj(Box::new(jj::JjRepo::load(&root)?))),
        None => Ok(Backend::Git(Box::new(git::discover(cwd)?))),
    }
}

/// Open, enumerate, and build the content source for `target`.
pub fn prepare(
    cwd: &Path,
    target: &DiffTarget,
    pathspecs: &[String],
) -> Result<Prepared, AnnotError> {
    match open(cwd)? {
        Backend::Git(repo) => {
            let entries = git::enumerate_in(&repo, target, pathspecs)?;
            let source = Arc::new(crate::source::GixSource::new(
                *repo,
                crate::pipeline::build_oid_map(&entries),
            ));
            Ok(Prepared {
                entries,
                source,
                label: None,
            })
        }
        Backend::Jj(mut repo) => repo.enumerate(target, pathspecs),
    }
}

pub(crate) fn entry_path(e: &FileEntry) -> &str {
    e.new_path
        .as_deref()
        .or(e.old_path.as_deref())
        .unwrap_or("")
}

/// Reorders `entries` into directories-first, alphabetical-within-level
/// order — the same shape the sidebar file tree renders (mirrors `file-
/// tree.ts`'s `flatten()`). Builds a path tree keyed by segment, then walks
/// it depth-first: a node's subdirectories (each fully recursed, sorted by
/// name) come before its own direct files (sorted by name).
///
/// Mandatory, not cosmetic: the frontend's annotation identity keys off array
/// position (`FileKey::diff_file(index)`), and git's status items arrive
/// interleaved from two producer threads.
pub(crate) fn tree_sort(mut entries: Vec<FileEntry>) -> Vec<FileEntry> {
    #[derive(Default)]
    struct Node<'a> {
        dirs: BTreeMap<&'a str, Node<'a>>,
        files: Vec<(&'a str, usize)>,
    }

    let mut root = Node::default();
    for (i, e) in entries.iter().enumerate() {
        let mut node = &mut root;
        let mut segments = entry_path(e).split('/').peekable();
        while let Some(seg) = segments.next() {
            if segments.peek().is_none() {
                node.files.push((seg, i));
            } else {
                node = node.dirs.entry(seg).or_default();
            }
        }
    }

    fn flatten(node: &Node, order: &mut Vec<usize>) {
        for child in node.dirs.values() {
            flatten(child, order);
        }
        let mut files = node.files.clone();
        files.sort_by_key(|(name, _)| *name);
        order.extend(files.into_iter().map(|(_, i)| i));
    }

    let mut order = Vec::with_capacity(entries.len());
    flatten(&root, &mut order);

    let mut slots: Vec<Option<FileEntry>> = entries.drain(..).map(Some).collect();
    order
        .into_iter()
        .map(|i| slots[i].take().unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_target_serde_roundtrip() {
        let targets = [
            DiffTarget::WorkingCopy,
            DiffTarget::Staged,
            DiffTarget::Revision { rev: "@".into() },
            DiffTarget::Range {
                from: "main".into(),
                to: "HEAD".into(),
                merge_base: true,
            },
        ];
        for target in targets {
            let json = serde_json::to_string(&target).unwrap();
            assert_eq!(serde_json::from_str::<DiffTarget>(&json).unwrap(), target);
        }
        assert_eq!(
            serde_json::to_string(&DiffTarget::WorkingCopy).unwrap(),
            r#"{"kind":"working_copy"}"#
        );
        assert_eq!(
            serde_json::from_str::<DiffTarget>(r#"{"kind":"revision","rev":"abc"}"#).unwrap(),
            DiffTarget::Revision { rev: "abc".into() }
        );
        // merge_base defaults to false
        assert_eq!(
            serde_json::from_str::<DiffTarget>(r#"{"kind":"range","from":"a","to":"b"}"#).unwrap(),
            DiffTarget::Range {
                from: "a".into(),
                to: "b".into(),
                merge_base: false,
            }
        );
    }

    #[test]
    fn labels() {
        assert_eq!(DiffTarget::WorkingCopy.label(), "diff");
        assert_eq!(DiffTarget::Staged.label(), "staged");
        assert_eq!(DiffTarget::Revision { rev: "@-".into() }.label(), "@-");
        assert_eq!(
            DiffTarget::Range {
                from: "a".into(),
                to: "b".into(),
                merge_base: false
            }
            .label(),
            "a..b"
        );
    }
}
